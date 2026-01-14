//! Workflow validation.
//!
//! Validates workflow JSON files for structural and semantic correctness.

use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::types::{ExecutionMode, Workflow};

/// Validate a workflow file at the given path.
///
/// Loads the file, parses it as JSON, and runs all validations.
/// Prints results to stdout and returns an error if validation fails.
pub fn validate_workflow_file(path: &str) -> Result<()> {
    println!("Validating workflow: {}\n", path);

    // Check file exists
    let path = Path::new(path);
    if !path.exists() {
        bail!("File not found: {}", path.display());
    }

    // Read file
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;

    // Parse as generic JSON first to check syntax
    let json_value: Value = match serde_json::from_str(&content) {
        Ok(v) => {
            println!("  [ok] JSON syntax valid");
            v
        }
        Err(e) => {
            println!("  [FAIL] JSON syntax error: {}", e);
            bail!("Validation failed: invalid JSON");
        }
    };

    // Validate schema (required fields) before deserializing
    let schema_errors = validate_schema(&json_value);

    // Try to deserialize - this catches additional type errors with line/column info
    let workflow: Workflow = match serde_json::from_str(&content) {
        Ok(w) => w,
        Err(e) => {
            // If we have schema errors, report those (more readable)
            // Otherwise fall back to serde's error with line/column
            if !schema_errors.is_empty() {
                for error in &schema_errors {
                    println!("  [FAIL] {}", error);
                }
                println!("\nValidation failed with {} error(s).", schema_errors.len());
            } else {
                // Format serde error to be more readable while keeping location
                let error_msg = format_serde_error(&e);
                println!("  [FAIL] {}", error_msg);
                println!("\nValidation failed with 1 error(s).");
            }
            bail!("Workflow validation failed");
        }
    };

    // If deserialization succeeded but we found schema issues, report them
    if !schema_errors.is_empty() {
        for error in &schema_errors {
            println!("  [FAIL] {}", error);
        }
        println!("\nValidation failed with {} error(s).", schema_errors.len());
        bail!("Workflow validation failed");
    }

    // Run semantic validations
    let errors = validate_workflow(&workflow);

    // Print results
    if errors.is_empty() {
        println!("  [ok] {} phase(s) found", workflow.phases.len());
        println!("  [ok] Phase IDs are unique");
        println!("  [ok] All dependencies valid");
        println!("  [ok] No circular dependencies");
        println!("  [ok] Execution modes configured correctly");
        println!("  [ok] Nested phases consistent");
        println!("  [ok] Variables valid");
        println!("\nWorkflow is valid!");
        Ok(())
    } else {
        for error in &errors {
            println!("  [FAIL] {}", error);
        }
        println!("\nValidation failed with {} error(s).", errors.len());
        bail!("Workflow validation failed");
    }
}

/// Format serde error to be more readable while preserving line/column info.
fn format_serde_error(e: &serde_json::Error) -> String {
    let msg = e.to_string();
    let line = e.line();
    let col = e.column();

    // Try to extract and improve common error patterns
    if msg.contains("missing field") {
        // Extract field name from "missing field `fieldname`"
        if let Some(start) = msg.find("missing field `") {
            let rest = &msg[start + 15..];
            if let Some(end) = rest.find('`') {
                let field = &rest[..end];
                return format!(
                    "Missing required field '{}' at line {}, column {}",
                    field, line, col
                );
            }
        }
    } else if msg.contains("unknown field") {
        if let Some(start) = msg.find("unknown field `") {
            let rest = &msg[start + 15..];
            if let Some(end) = rest.find('`') {
                let field = &rest[..end];
                return format!("Unknown field '{}' at line {}, column {}", field, line, col);
            }
        }
    } else if msg.contains("invalid type") {
        return format!("Type error at line {}, column {}: {}", line, col, msg);
    }

    // Fallback: just add line/column if not already present
    if !msg.contains("line") {
        format!("{} (at line {}, column {})", msg, line, col)
    } else {
        msg
    }
}

/// Validate JSON schema - check required fields are present with clear error messages.
fn validate_schema(json: &Value) -> Vec<String> {
    let mut errors = Vec::new();

    // Check root is an object
    let obj = match json.as_object() {
        Some(o) => o,
        None => {
            errors.push("Workflow must be a JSON object".to_string());
            return errors;
        }
    };

    // Check required workflow fields
    if !obj.contains_key("name") {
        errors.push("Workflow is missing required field: name".to_string());
    } else if !obj["name"].is_string() {
        errors.push("Workflow field 'name' must be a string".to_string());
    }

    if !obj.contains_key("version") {
        errors.push("Workflow is missing required field: version".to_string());
    } else if !obj["version"].is_string() {
        errors.push("Workflow field 'version' must be a string".to_string());
    }

    if !obj.contains_key("phases") {
        errors.push("Workflow is missing required field: phases".to_string());
        return errors;
    }

    let phases = match obj["phases"].as_array() {
        Some(p) => p,
        None => {
            errors.push("Workflow field 'phases' must be an array".to_string());
            return errors;
        }
    };

    if phases.is_empty() {
        errors.push("Workflow must have at least one phase".to_string());
    }

    // Validate each phase
    for (i, phase) in phases.iter().enumerate() {
        let phase_obj = match phase.as_object() {
            Some(o) => o,
            None => {
                errors.push(format!("Phase {} must be a JSON object", i + 1));
                continue;
            }
        };

        // Get phase id for error messages (or use index)
        let phase_id = phase_obj
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| format!("\"{}\"", s))
            .unwrap_or_else(|| format!("at index {}", i));

        // Check required phase fields
        if !phase_obj.contains_key("id") {
            errors.push(format!("Phase {} is missing required field: id", i + 1));
        } else if !phase_obj["id"].is_string() {
            errors.push(format!("Phase {} field 'id' must be a string", phase_id));
        }

        if !phase_obj.contains_key("name") {
            errors.push(format!(
                "Phase {} is missing required field: name",
                phase_id
            ));
        } else if !phase_obj["name"].is_string() {
            errors.push(format!("Phase {} field 'name' must be a string", phase_id));
        }

        if !phase_obj.contains_key("prompt") {
            errors.push(format!(
                "Phase {} is missing required field: prompt",
                phase_id
            ));
        } else if !phase_obj["prompt"].is_string() {
            errors.push(format!(
                "Phase {} field 'prompt' must be a string",
                phase_id
            ));
        }

        if !phase_obj.contains_key("execution") {
            errors.push(format!(
                "Phase {} is missing required field: execution",
                phase_id
            ));
        } else {
            // Validate execution config
            let exec = &phase_obj["execution"];
            if let Some(exec_obj) = exec.as_object() {
                if !exec_obj.contains_key("mode") {
                    errors.push(format!(
                        "Phase {} execution is missing required field: mode",
                        phase_id
                    ));
                } else if let Some(mode) = exec_obj["mode"].as_str() {
                    if mode != "once" && mode != "iterate" {
                        errors.push(format!(
                            "Phase {} has invalid execution mode: \"{}\" (must be \"once\" or \"iterate\")",
                            phase_id, mode
                        ));
                    }
                } else {
                    errors.push(format!(
                        "Phase {} execution field 'mode' must be a string",
                        phase_id
                    ));
                }
            } else {
                errors.push(format!(
                    "Phase {} field 'execution' must be an object",
                    phase_id
                ));
            }
        }
    }

    // Validate variables if present
    if let Some(vars) = obj.get("variables") {
        if let Some(vars_arr) = vars.as_array() {
            for (i, var) in vars_arr.iter().enumerate() {
                let var_obj = match var.as_object() {
                    Some(o) => o,
                    None => {
                        errors.push(format!("Variable {} must be a JSON object", i + 1));
                        continue;
                    }
                };

                let var_name = var_obj
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(|s| format!("\"{}\"", s))
                    .unwrap_or_else(|| format!("at index {}", i));

                if !var_obj.contains_key("name") {
                    errors.push(format!(
                        "Variable {} is missing required field: name",
                        i + 1
                    ));
                }

                if !var_obj.contains_key("type") {
                    errors.push(format!(
                        "Variable {} is missing required field: type",
                        var_name
                    ));
                } else if let Some(vtype) = var_obj["type"].as_str() {
                    if !["env", "bash", "file", "json"].contains(&vtype) {
                        errors.push(format!(
                            "Variable {} has invalid type: \"{}\" (must be \"env\", \"bash\", \"file\", or \"json\")",
                            var_name, vtype
                        ));
                    }
                }

                if !var_obj.contains_key("source") {
                    errors.push(format!(
                        "Variable {} is missing required field: source",
                        var_name
                    ));
                }
            }
        }
    }

    errors
}

/// Validate a workflow and return a list of errors.
///
/// Returns an empty vector if the workflow is valid.
pub fn validate_workflow(workflow: &Workflow) -> Vec<String> {
    let mut errors = Vec::new();

    // Collect all phase IDs for reference checks
    let phase_ids: HashSet<&str> = workflow.phases.iter().map(|p| p.id.as_str()).collect();

    // 1. Check for duplicate phase IDs
    let mut seen_ids = HashSet::new();
    for phase in &workflow.phases {
        if !seen_ids.insert(&phase.id) {
            errors.push(format!("Duplicate phase ID: \"{}\"", phase.id));
        }
    }

    // 2. Check that all depends_on references exist
    for phase in &workflow.phases {
        for dep in &phase.depends_on {
            if !phase_ids.contains(dep.as_str()) {
                errors.push(format!(
                    "Phase \"{}\" depends on non-existent phase \"{}\"",
                    phase.id, dep
                ));
            }
        }
    }

    // 3. Check for circular dependencies
    if let Some(cycle) = detect_circular_dependencies(workflow) {
        errors.push(format!("Circular dependency detected: {}", cycle));
    }

    // 4. Check iterate mode has iterate_over
    for phase in &workflow.phases {
        if phase.execution.mode == ExecutionMode::Iterate && phase.execution.iterate_over.is_none()
        {
            errors.push(format!(
                "Phase \"{}\" has mode=iterate but no iterate_over path",
                phase.id
            ));
        }
    }

    // 5. Check nested phase consistency
    for phase in &workflow.phases {
        // Check nested_phases references exist
        for nested_id in &phase.nested_phases {
            if !phase_ids.contains(nested_id.as_str()) {
                errors.push(format!(
                    "Phase \"{}\" references non-existent nested phase \"{}\"",
                    phase.id, nested_id
                ));
            }
        }

        // Check parent reference exists
        if let Some(parent_id) = &phase.parent {
            if !phase_ids.contains(parent_id.as_str()) {
                errors.push(format!(
                    "Phase \"{}\" references non-existent parent \"{}\"",
                    phase.id, parent_id
                ));
            }
        }
    }

    // 6. Check bidirectional parent/child consistency
    for phase in &workflow.phases {
        // If phase has a parent, parent should list it in nested_phases
        if let Some(parent_id) = &phase.parent {
            if let Some(parent) = workflow.phases.iter().find(|p| &p.id == parent_id) {
                if !parent.nested_phases.contains(&phase.id) {
                    errors.push(format!(
                        "Phase \"{}\" has parent \"{}\" but parent doesn't list it in nested_phases",
                        phase.id, parent_id
                    ));
                }
            }
        }

        // If phase lists nested_phases, those phases should have this as parent
        for nested_id in &phase.nested_phases {
            if let Some(nested) = workflow.phases.iter().find(|p| &p.id == nested_id) {
                if nested.parent.as_ref() != Some(&phase.id) {
                    errors.push(format!(
                        "Phase \"{}\" lists \"{}\" as nested but nested phase doesn't have it as parent",
                        phase.id, nested_id
                    ));
                }
            }
        }
    }

    // 7. Check variable name uniqueness
    let mut seen_vars = HashSet::new();
    for var in &workflow.variables {
        if !seen_vars.insert(&var.name) {
            errors.push(format!("Duplicate variable name: \"{}\"", var.name));
        }
    }

    // 8. Check for circular variable dependencies
    if let Some(cycle) = detect_variable_cycles(workflow) {
        errors.push(format!("Circular variable dependency: {}", cycle));
    }

    errors
}

/// Detect circular dependencies among phases using DFS.
///
/// Returns the cycle path as a string if found, None otherwise.
fn detect_circular_dependencies(workflow: &Workflow) -> Option<String> {
    // Build adjacency map: phase_id -> depends_on
    let mut deps: HashMap<&str, Vec<&str>> = HashMap::new();
    for phase in &workflow.phases {
        deps.insert(
            &phase.id,
            phase.depends_on.iter().map(|s| s.as_str()).collect(),
        );
    }

    // DFS with path tracking
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();
    let mut path = Vec::new();

    for phase in &workflow.phases {
        if !visited.contains(phase.id.as_str()) {
            if let Some(cycle) =
                dfs_cycle(&phase.id, &deps, &mut visited, &mut rec_stack, &mut path)
            {
                return Some(cycle);
            }
        }
    }

    None
}

fn dfs_cycle<'a>(
    node: &'a str,
    deps: &HashMap<&str, Vec<&'a str>>,
    visited: &mut HashSet<&'a str>,
    rec_stack: &mut HashSet<&'a str>,
    path: &mut Vec<&'a str>,
) -> Option<String> {
    visited.insert(node);
    rec_stack.insert(node);
    path.push(node);

    if let Some(neighbors) = deps.get(node) {
        for &neighbor in neighbors {
            if !visited.contains(neighbor) {
                if let Some(cycle) = dfs_cycle(neighbor, deps, visited, rec_stack, path) {
                    return Some(cycle);
                }
            } else if rec_stack.contains(neighbor) {
                // Found cycle - build cycle string
                let cycle_start = path.iter().position(|&n| n == neighbor).unwrap();
                let cycle: Vec<&str> = path[cycle_start..].to_vec();
                return Some(format!("{} -> {}", cycle.join(" -> "), neighbor));
            }
        }
    }

    path.pop();
    rec_stack.remove(node);
    None
}

/// Detect circular dependencies among variables.
fn detect_variable_cycles(workflow: &Workflow) -> Option<String> {
    if workflow.variables.is_empty() {
        return None;
    }

    // Build dependency graph by scanning for {{var.X}} references
    let var_names: HashSet<&str> = workflow.variables.iter().map(|v| v.name.as_str()).collect();
    let mut deps: HashMap<&str, Vec<&str>> = HashMap::new();

    for var in &workflow.variables {
        let mut var_deps = Vec::new();

        // Scan source for {{var.X}} references
        for other_name in &var_names {
            let pattern = format!("{{{{var.{}}}}}", other_name);
            if var.source.contains(&pattern) {
                var_deps.push(*other_name);
            }
            // Also check path field for json type
            if let Some(path) = &var.path {
                if path.contains(&pattern) {
                    var_deps.push(*other_name);
                }
            }
        }

        deps.insert(&var.name, var_deps);
    }

    // DFS for cycles
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();
    let mut path = Vec::new();

    for var in &workflow.variables {
        if !visited.contains(var.name.as_str()) {
            if let Some(cycle) =
                dfs_cycle(&var.name, &deps, &mut visited, &mut rec_stack, &mut path)
            {
                return Some(cycle);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::types::{ExecutionConfig, Phase, WorkflowDefaults};

    fn make_phase(id: &str, depends_on: Vec<&str>) -> Phase {
        Phase {
            id: id.to_string(),
            name: id.to_string(),
            execution: ExecutionConfig {
                mode: ExecutionMode::Once,
                iterate_over: None,
                item_variable: "item".to_string(),
                skip_if_empty: false,
            },
            agent: None,
            model: None,
            interactive: None,
            skip_permissions: None,
            system_prompt: None,
            prompt: "test".to_string(),
            output: None,
            depends_on: depends_on.into_iter().map(String::from).collect(),
            parent: None,
            nested_phases: vec![],
        }
    }

    fn make_workflow(phases: Vec<Phase>) -> Workflow {
        Workflow {
            name: "test".to_string(),
            version: "1.0".to_string(),
            description: None,
            defaults: WorkflowDefaults::default(),
            variables: vec![],
            definitions: std::collections::HashMap::new(),
            phases,
        }
    }

    #[test]
    fn test_valid_workflow() {
        let workflow = make_workflow(vec![
            make_phase("a", vec![]),
            make_phase("b", vec!["a"]),
            make_phase("c", vec!["b"]),
        ]);
        let errors = validate_workflow(&workflow);
        assert!(errors.is_empty(), "Expected no errors: {:?}", errors);
    }

    #[test]
    fn test_duplicate_phase_id() {
        let workflow = make_workflow(vec![make_phase("a", vec![]), make_phase("a", vec![])]);
        let errors = validate_workflow(&workflow);
        assert!(errors.iter().any(|e| e.contains("Duplicate phase ID")));
    }

    #[test]
    fn test_missing_dependency() {
        let workflow = make_workflow(vec![make_phase("a", vec!["nonexistent"])]);
        let errors = validate_workflow(&workflow);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("depends on non-existent phase"))
        );
    }

    #[test]
    fn test_circular_dependency() {
        let workflow = make_workflow(vec![make_phase("a", vec!["b"]), make_phase("b", vec!["a"])]);
        let errors = validate_workflow(&workflow);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("Circular dependency detected"))
        );
    }

    #[test]
    fn test_iterate_without_iterate_over() {
        let mut phase = make_phase("a", vec![]);
        phase.execution.mode = ExecutionMode::Iterate;
        let workflow = make_workflow(vec![phase]);
        let errors = validate_workflow(&workflow);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("mode=iterate but no iterate_over"))
        );
    }

    #[test]
    fn test_nested_phase_missing() {
        let mut phase = make_phase("parent", vec![]);
        phase.nested_phases = vec!["nonexistent".to_string()];
        let workflow = make_workflow(vec![phase]);
        let errors = validate_workflow(&workflow);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("non-existent nested phase"))
        );
    }

    #[test]
    fn test_parent_child_consistency() {
        let mut parent = make_phase("parent", vec![]);
        parent.nested_phases = vec!["child".to_string()];
        let child = make_phase("child", vec![]);
        // Missing: child.parent = Some("parent".to_string());

        let workflow = make_workflow(vec![parent, child]);
        let errors = validate_workflow(&workflow);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("doesn't have it as parent"))
        );
    }
}
