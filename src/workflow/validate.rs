//! Workflow validation.
//!
//! Validates workflow JSON files for structural and semantic correctness.

use anyhow::{Context, Result, bail};
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

    // Read and parse JSON
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;

    let workflow: Workflow = match serde_json::from_str(&content) {
        Ok(w) => {
            println!("  [ok] JSON syntax valid");
            w
        }
        Err(e) => {
            println!("  [FAIL] JSON syntax error: {}", e);
            bail!("Validation failed: invalid JSON");
        }
    };

    // Run validations
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
