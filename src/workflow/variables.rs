//! Variable resolution for workflow templates.
//!
//! Supports three types of variable sources:
//! - `env`: Environment variables
//! - `bash`: Command output (stdout)
//! - `file`: File contents
//!
//! Variables are automatically sorted by dependencies, so they can be
//! defined in any order and reference each other via `{{var.name}}`.

use anyhow::{Context, Result, bail};
use std::collections::{HashMap, HashSet};
use std::process::Command;

use super::template::TemplateEngine;
use super::types::{VariableType, WorkflowVariable};

/// Resolves workflow variables from their sources.
pub struct VariableResolver;

impl VariableResolver {
    /// Resolve all workflow variables and add them to the template engine.
    ///
    /// Variables are automatically sorted by their dependencies (detected via
    /// `{{var.X}}` patterns in source fields), so they can be defined in any
    /// order. Circular dependencies are detected and reported as errors.
    ///
    /// Custom variables are prefixed with `var.` to distinguish them from
    /// built-in variables like `state_dir`, `index`, and `item.*`.
    pub fn resolve_all(
        variables: &[WorkflowVariable],
        template: &mut TemplateEngine,
    ) -> Result<()> {
        // Sort variables by dependencies
        let sorted = Self::topological_sort(variables)?;

        for var in sorted {
            let value = Self::resolve_one(var, template)?;
            // Prefix with "var." to namespace custom variables
            template.set(format!("var.{}", &var.name), value);
        }
        Ok(())
    }

    /// Extract variable dependencies from a source string.
    /// Looks for `{{var.X}}` patterns and returns the variable names.
    fn extract_dependencies(source: &str) -> HashSet<String> {
        let mut deps = HashSet::new();
        let mut remaining = source;

        while let Some(start) = remaining.find("{{var.") {
            let after_prefix = &remaining[start + 6..]; // Skip "{{var."
            if let Some(end) = after_prefix.find("}}") {
                let var_name = &after_prefix[..end];
                deps.insert(var_name.to_string());
                remaining = &after_prefix[end + 2..];
            } else {
                break;
            }
        }

        deps
    }

    /// Topologically sort variables by their dependencies.
    /// Returns an error if circular dependencies are detected.
    fn topological_sort(variables: &[WorkflowVariable]) -> Result<Vec<&WorkflowVariable>> {
        // Build name -> variable map
        let var_map: HashMap<&str, &WorkflowVariable> =
            variables.iter().map(|v| (v.name.as_str(), v)).collect();

        // Build dependency graph: var_name -> set of variables it depends on
        let mut deps: HashMap<&str, HashSet<String>> = HashMap::new();
        for var in variables {
            let var_deps = Self::extract_dependencies(&var.source);
            // Filter to only include dependencies that are defined variables
            let valid_deps: HashSet<String> = var_deps
                .into_iter()
                .filter(|d| var_map.contains_key(d.as_str()))
                .collect();
            deps.insert(&var.name, valid_deps);
        }

        // Build reverse dependency graph: var_name -> variables that depend on it
        let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();
        for var in variables {
            dependents.insert(&var.name, Vec::new());
        }
        for (var_name, var_deps) in &deps {
            for dep in var_deps {
                if let Some(list) = dependents.get_mut(dep.as_str()) {
                    list.push(var_name);
                }
            }
        }

        // in_degree = number of dependencies each variable has
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        for (var_name, var_deps) in &deps {
            in_degree.insert(var_name, var_deps.len());
        }

        // Start with variables that have no dependencies
        let mut queue: Vec<&str> = in_degree
            .iter()
            .filter(|&(_, count)| *count == 0)
            .map(|(&name, _)| name)
            .collect();

        let mut sorted: Vec<&WorkflowVariable> = Vec::new();

        while let Some(name) = queue.pop() {
            if let Some(&var) = var_map.get(name) {
                sorted.push(var);
            }

            // Decrement in_degree for all variables that depend on this one
            if let Some(deps_on_me) = dependents.get(name) {
                for dependent in deps_on_me {
                    if let Some(count) = in_degree.get_mut(dependent) {
                        *count -= 1;
                        if *count == 0 {
                            queue.push(dependent);
                        }
                    }
                }
            }
        }

        // Check for circular dependencies
        if sorted.len() != variables.len() {
            let remaining: Vec<&str> = in_degree
                .iter()
                .filter(|&(_, count)| *count > 0)
                .map(|(&name, _)| name)
                .collect();
            bail!(
                "Circular dependency detected among variables: {}",
                remaining.join(", ")
            );
        }

        Ok(sorted)
    }

    fn resolve_one(var: &WorkflowVariable, template: &TemplateEngine) -> Result<String> {
        // Expand templates in source (e.g., {{state_dir}}/file.md)
        let source = template.expand(&var.source);
        // Expand templates in path if present
        let path = var.path.as_ref().map(|p| template.expand(p));

        let result = match var.var_type {
            VariableType::Env => Self::resolve_env(&source),
            VariableType::Bash => Self::resolve_bash(&source),
            VariableType::File => Self::resolve_file(&source),
            VariableType::Json => Self::resolve_json(&source, path.as_deref()),
        };

        match result {
            Ok(value) => Ok(value),
            Err(e) => {
                if let Some(default) = &var.default {
                    Ok(default.clone())
                } else if var.required {
                    bail!("Failed to resolve variable '{}': {}", var.name, e)
                } else {
                    Ok(String::new())
                }
            }
        }
    }

    fn resolve_env(name: &str) -> Result<String> {
        std::env::var(name).with_context(|| format!("Environment variable '{}' not set", name))
    }

    fn resolve_bash(command: &str) -> Result<String> {
        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .with_context(|| format!("Failed to execute: {}", command))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Command failed: {}", stderr.trim());
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn resolve_file(path: &str) -> Result<String> {
        std::fs::read_to_string(path).with_context(|| format!("Failed to read file: {}", path))
    }

    fn resolve_json(file_path: &str, json_path: Option<&str>) -> Result<String> {
        let contents = std::fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read JSON file: {}", file_path))?;

        let value: serde_json::Value = serde_json::from_str(&contents)
            .with_context(|| format!("Failed to parse JSON file: {}", file_path))?;

        let path = json_path.unwrap_or("");
        let result = Self::navigate_json_path(&value, path)
            .with_context(|| format!("Failed to resolve path '{}' in {}", path, file_path))?;

        // Convert the result to a string
        match result {
            serde_json::Value::String(s) => Ok(s.clone()),
            serde_json::Value::Number(n) => Ok(n.to_string()),
            serde_json::Value::Bool(b) => Ok(b.to_string()),
            serde_json::Value::Null => Ok("null".to_string()),
            // For objects and arrays, return pretty-printed JSON
            _ => Ok(serde_json::to_string_pretty(result)?),
        }
    }

    /// Navigate a JSON value using dot-notation path.
    /// Supports: .field, .nested.field, .array[0], .array[0].field
    fn navigate_json_path<'a>(
        value: &'a serde_json::Value,
        path: &str,
    ) -> Result<&'a serde_json::Value> {
        // Handle empty path or just "." - return the whole value
        let path = path.trim();
        if path.is_empty() || path == "." {
            return Ok(value);
        }

        // Remove leading dot if present
        let path = path.strip_prefix('.').unwrap_or(path);

        let mut current = value;

        // Parse and navigate each segment
        let mut remaining = path;
        while !remaining.is_empty() {
            // Check for array index: field[n] or [n]
            if let Some(bracket_pos) = remaining.find('[') {
                // Get field name before bracket (if any)
                let field = &remaining[..bracket_pos];
                if !field.is_empty() {
                    current = current
                        .get(field)
                        .ok_or_else(|| anyhow::anyhow!("Field '{}' not found", field))?;
                }

                // Find closing bracket
                let after_bracket = &remaining[bracket_pos + 1..];
                let close_pos = after_bracket
                    .find(']')
                    .ok_or_else(|| anyhow::anyhow!("Unclosed bracket in path"))?;

                // Parse index
                let index_str = &after_bracket[..close_pos];
                let index: usize = index_str
                    .parse()
                    .with_context(|| format!("Invalid array index: {}", index_str))?;

                current = current
                    .get(index)
                    .ok_or_else(|| anyhow::anyhow!("Array index {} out of bounds", index))?;

                // Move past the bracket and continue
                remaining = &after_bracket[close_pos + 1..];
                // Skip leading dot for next segment
                remaining = remaining.strip_prefix('.').unwrap_or(remaining);
            } else {
                // No bracket - find next dot or end
                let (field, rest) = match remaining.find('.') {
                    Some(dot_pos) => (&remaining[..dot_pos], &remaining[dot_pos + 1..]),
                    None => (remaining, ""),
                };

                if !field.is_empty() {
                    current = current
                        .get(field)
                        .ok_or_else(|| anyhow::anyhow!("Field '{}' not found", field))?;
                }

                remaining = rest;
            }
        }

        Ok(current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_env() {
        // SAFETY: This test runs in isolation and only modifies a unique test variable
        unsafe {
            std::env::set_var("TEST_VAR_12345", "test_value");
        }
        let result = VariableResolver::resolve_env("TEST_VAR_12345");
        assert_eq!(result.unwrap(), "test_value");
        // SAFETY: Cleaning up the test variable
        unsafe {
            std::env::remove_var("TEST_VAR_12345");
        }
    }

    #[test]
    fn test_resolve_env_missing() {
        let result = VariableResolver::resolve_env("NONEXISTENT_VAR_99999");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_bash() {
        let result = VariableResolver::resolve_bash("echo hello");
        assert_eq!(result.unwrap(), "hello");
    }

    #[test]
    fn test_resolve_bash_failure() {
        let result = VariableResolver::resolve_bash("exit 1");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_file() {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_var_file.txt");
        std::fs::write(&temp_file, "file contents").unwrap();

        let result = VariableResolver::resolve_file(temp_file.to_str().unwrap());
        assert_eq!(result.unwrap(), "file contents");

        std::fs::remove_file(temp_file).unwrap();
    }

    #[test]
    fn test_resolve_file_missing() {
        let result = VariableResolver::resolve_file("/nonexistent/path/file.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_all_with_defaults() {
        let mut template = TemplateEngine::new();
        let variables = vec![WorkflowVariable {
            name: "missing_var".to_string(),
            var_type: VariableType::Env,
            source: "NONEXISTENT_VAR_88888".to_string(),
            path: None,
            required: false,
            default: Some("default_value".to_string()),
        }];

        VariableResolver::resolve_all(&variables, &mut template).unwrap();
        // Custom variables are prefixed with "var."
        assert_eq!(
            template.get("var.missing_var"),
            Some(&"default_value".to_string())
        );
    }

    #[test]
    fn test_resolve_all_required_failure() {
        let mut template = TemplateEngine::new();
        let variables = vec![WorkflowVariable {
            name: "required_var".to_string(),
            var_type: VariableType::Env,
            source: "NONEXISTENT_VAR_77777".to_string(),
            path: None,
            required: true,
            default: None,
        }];

        let result = VariableResolver::resolve_all(&variables, &mut template);
        assert!(result.is_err());
    }

    #[test]
    fn test_template_expansion_in_source() {
        let mut template = TemplateEngine::new();
        template.set("state_dir", "/tmp/test");

        let variables = vec![WorkflowVariable {
            name: "result".to_string(),
            var_type: VariableType::Bash,
            source: "echo {{state_dir}}".to_string(),
            path: None,
            required: true,
            default: None,
        }];

        VariableResolver::resolve_all(&variables, &mut template).unwrap();
        // Custom variables are prefixed with "var."
        assert_eq!(template.get("var.result"), Some(&"/tmp/test".to_string()));
    }

    #[test]
    fn test_variable_chaining() {
        let mut template = TemplateEngine::new();

        let variables = vec![
            WorkflowVariable {
                name: "first".to_string(),
                var_type: VariableType::Bash,
                source: "echo hello".to_string(),
                path: None,
                required: true,
                default: None,
            },
            WorkflowVariable {
                name: "second".to_string(),
                var_type: VariableType::Bash,
                source: "echo {{var.first}} world".to_string(),
                path: None,
                required: true,
                default: None,
            },
        ];

        VariableResolver::resolve_all(&variables, &mut template).unwrap();
        assert_eq!(template.get("var.first"), Some(&"hello".to_string()));
        assert_eq!(template.get("var.second"), Some(&"hello world".to_string()));
    }

    #[test]
    fn test_variable_chaining_reverse_order() {
        // Variables defined in reverse order should still work
        let mut template = TemplateEngine::new();

        let variables = vec![
            WorkflowVariable {
                name: "second".to_string(),
                var_type: VariableType::Bash,
                source: "echo {{var.first}} world".to_string(),
                path: None,
                required: true,
                default: None,
            },
            WorkflowVariable {
                name: "first".to_string(),
                var_type: VariableType::Bash,
                source: "echo hello".to_string(),
                path: None,
                required: true,
                default: None,
            },
        ];

        VariableResolver::resolve_all(&variables, &mut template).unwrap();
        assert_eq!(template.get("var.first"), Some(&"hello".to_string()));
        assert_eq!(template.get("var.second"), Some(&"hello world".to_string()));
    }

    #[test]
    fn test_variable_chain_three_levels() {
        // Test A -> B -> C dependency chain defined in reverse
        let mut template = TemplateEngine::new();

        let variables = vec![
            WorkflowVariable {
                name: "c".to_string(),
                var_type: VariableType::Bash,
                source: "echo {{var.b}} c".to_string(),
                path: None,
                required: true,
                default: None,
            },
            WorkflowVariable {
                name: "b".to_string(),
                var_type: VariableType::Bash,
                source: "echo {{var.a}} b".to_string(),
                path: None,
                required: true,
                default: None,
            },
            WorkflowVariable {
                name: "a".to_string(),
                var_type: VariableType::Bash,
                source: "echo a".to_string(),
                path: None,
                required: true,
                default: None,
            },
        ];

        VariableResolver::resolve_all(&variables, &mut template).unwrap();
        assert_eq!(template.get("var.a"), Some(&"a".to_string()));
        assert_eq!(template.get("var.b"), Some(&"a b".to_string()));
        assert_eq!(template.get("var.c"), Some(&"a b c".to_string()));
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut template = TemplateEngine::new();

        let variables = vec![
            WorkflowVariable {
                name: "a".to_string(),
                var_type: VariableType::Bash,
                source: "echo {{var.b}}".to_string(),
                path: None,
                required: true,
                default: None,
            },
            WorkflowVariable {
                name: "b".to_string(),
                var_type: VariableType::Bash,
                source: "echo {{var.a}}".to_string(),
                path: None,
                required: true,
                default: None,
            },
        ];

        let result = VariableResolver::resolve_all(&variables, &mut template);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Circular dependency")
        );
    }

    #[test]
    fn test_extract_dependencies() {
        let deps = VariableResolver::extract_dependencies("echo {{var.foo}} and {{var.bar}}");
        assert!(deps.contains("foo"));
        assert!(deps.contains("bar"));
        assert_eq!(deps.len(), 2);
    }

    #[test]
    fn test_extract_dependencies_ignores_non_var() {
        let deps = VariableResolver::extract_dependencies("{{state_dir}}/{{var.name}}/file");
        assert!(deps.contains("name"));
        assert_eq!(deps.len(), 1);
    }

    // JSON variable tests

    #[test]
    fn test_resolve_json_simple_field() {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_json_simple.json");
        std::fs::write(&temp_file, r#"{"name": "test", "version": "1.0.0"}"#).unwrap();

        let result = VariableResolver::resolve_json(temp_file.to_str().unwrap(), Some(".name"));
        assert_eq!(result.unwrap(), "test");

        std::fs::remove_file(temp_file).unwrap();
    }

    #[test]
    fn test_resolve_json_nested_field() {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_json_nested.json");
        std::fs::write(
            &temp_file,
            r#"{"config": {"database": {"host": "localhost"}}}"#,
        )
        .unwrap();

        let result = VariableResolver::resolve_json(
            temp_file.to_str().unwrap(),
            Some(".config.database.host"),
        );
        assert_eq!(result.unwrap(), "localhost");

        std::fs::remove_file(temp_file).unwrap();
    }

    #[test]
    fn test_resolve_json_array_index() {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_json_array.json");
        std::fs::write(&temp_file, r#"{"items": ["first", "second", "third"]}"#).unwrap();

        let result = VariableResolver::resolve_json(temp_file.to_str().unwrap(), Some(".items[1]"));
        assert_eq!(result.unwrap(), "second");

        std::fs::remove_file(temp_file).unwrap();
    }

    #[test]
    fn test_resolve_json_array_object() {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_json_array_obj.json");
        std::fs::write(
            &temp_file,
            r#"{"users": [{"name": "alice"}, {"name": "bob"}]}"#,
        )
        .unwrap();

        let result =
            VariableResolver::resolve_json(temp_file.to_str().unwrap(), Some(".users[0].name"));
        assert_eq!(result.unwrap(), "alice");

        std::fs::remove_file(temp_file).unwrap();
    }

    #[test]
    fn test_resolve_json_number() {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_json_number.json");
        std::fs::write(&temp_file, r#"{"count": 42, "price": 19.99}"#).unwrap();

        let result = VariableResolver::resolve_json(temp_file.to_str().unwrap(), Some(".count"));
        assert_eq!(result.unwrap(), "42");

        let result = VariableResolver::resolve_json(temp_file.to_str().unwrap(), Some(".price"));
        assert_eq!(result.unwrap(), "19.99");

        std::fs::remove_file(temp_file).unwrap();
    }

    #[test]
    fn test_resolve_json_boolean() {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_json_bool.json");
        std::fs::write(&temp_file, r#"{"enabled": true, "debug": false}"#).unwrap();

        let result = VariableResolver::resolve_json(temp_file.to_str().unwrap(), Some(".enabled"));
        assert_eq!(result.unwrap(), "true");

        std::fs::remove_file(temp_file).unwrap();
    }

    #[test]
    fn test_resolve_json_null() {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_json_null.json");
        std::fs::write(&temp_file, r#"{"value": null}"#).unwrap();

        let result = VariableResolver::resolve_json(temp_file.to_str().unwrap(), Some(".value"));
        assert_eq!(result.unwrap(), "null");

        std::fs::remove_file(temp_file).unwrap();
    }

    #[test]
    fn test_resolve_json_no_path_returns_whole() {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_json_whole.json");
        std::fs::write(&temp_file, r#"{"a": 1}"#).unwrap();

        let result = VariableResolver::resolve_json(temp_file.to_str().unwrap(), None);
        assert!(result.unwrap().contains("\"a\": 1"));

        std::fs::remove_file(temp_file).unwrap();
    }

    #[test]
    fn test_resolve_json_missing_field() {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_json_missing.json");
        std::fs::write(&temp_file, r#"{"name": "test"}"#).unwrap();

        let result = VariableResolver::resolve_json(temp_file.to_str().unwrap(), Some(".missing"));
        assert!(result.is_err());

        std::fs::remove_file(temp_file).unwrap();
    }

    #[test]
    fn test_resolve_json_invalid_index() {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_json_invalid_idx.json");
        std::fs::write(&temp_file, r#"{"items": ["a", "b"]}"#).unwrap();

        let result =
            VariableResolver::resolve_json(temp_file.to_str().unwrap(), Some(".items[99]"));
        assert!(result.is_err());

        std::fs::remove_file(temp_file).unwrap();
    }

    #[test]
    fn test_resolve_json_invalid_json() {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_json_invalid.json");
        std::fs::write(&temp_file, "not valid json").unwrap();

        let result = VariableResolver::resolve_json(temp_file.to_str().unwrap(), Some(".field"));
        assert!(result.is_err());

        std::fs::remove_file(temp_file).unwrap();
    }

    #[test]
    fn test_resolve_json_missing_file() {
        let result = VariableResolver::resolve_json("/nonexistent/path.json", Some(".field"));
        assert!(result.is_err());
    }

    #[test]
    fn test_navigate_json_path_without_leading_dot() {
        let value: serde_json::Value = serde_json::from_str(r#"{"name": "test"}"#).unwrap();
        let result = VariableResolver::navigate_json_path(&value, "name");
        assert_eq!(result.unwrap().as_str().unwrap(), "test");
    }

    #[test]
    fn test_navigate_json_path_root_array() {
        let value: serde_json::Value = serde_json::from_str(r#"["a", "b", "c"]"#).unwrap();
        let result = VariableResolver::navigate_json_path(&value, "[1]");
        assert_eq!(result.unwrap().as_str().unwrap(), "b");
    }
}
