//! Variable resolution for workflow templates.
//!
//! Supports three types of variable sources:
//! - `env`: Environment variables
//! - `bash`: Command output (stdout)
//! - `file`: File contents
//!
//! Variables are automatically sorted by dependencies, so they can be
//! defined in any order and reference each other via `{{var.name}}`.

use anyhow::{bail, Context, Result};
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

        let result = match var.var_type {
            VariableType::Env => Self::resolve_env(&source),
            VariableType::Bash => Self::resolve_bash(&source),
            VariableType::File => Self::resolve_file(&source),
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
            required: false,
            default: Some("default_value".to_string()),
        }];

        VariableResolver::resolve_all(&variables, &mut template).unwrap();
        // Custom variables are prefixed with "var."
        assert_eq!(template.get("var.missing_var"), Some(&"default_value".to_string()));
    }

    #[test]
    fn test_resolve_all_required_failure() {
        let mut template = TemplateEngine::new();
        let variables = vec![WorkflowVariable {
            name: "required_var".to_string(),
            var_type: VariableType::Env,
            source: "NONEXISTENT_VAR_77777".to_string(),
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
                required: true,
                default: None,
            },
            WorkflowVariable {
                name: "second".to_string(),
                var_type: VariableType::Bash,
                source: "echo {{var.first}} world".to_string(),
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
                required: true,
                default: None,
            },
            WorkflowVariable {
                name: "first".to_string(),
                var_type: VariableType::Bash,
                source: "echo hello".to_string(),
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
                required: true,
                default: None,
            },
            WorkflowVariable {
                name: "b".to_string(),
                var_type: VariableType::Bash,
                source: "echo {{var.a}} b".to_string(),
                required: true,
                default: None,
            },
            WorkflowVariable {
                name: "a".to_string(),
                var_type: VariableType::Bash,
                source: "echo a".to_string(),
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
                required: true,
                default: None,
            },
            WorkflowVariable {
                name: "b".to_string(),
                var_type: VariableType::Bash,
                source: "echo {{var.a}}".to_string(),
                required: true,
                default: None,
            },
        ];

        let result = VariableResolver::resolve_all(&variables, &mut template);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Circular dependency"));
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
}
