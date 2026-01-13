use std::collections::HashMap;

/// Simple template engine for variable substitution in prompts.
///
/// Supports:
/// - `{{variable}}` - Simple variable substitution
/// - `{{object.field}}` - Nested field access for JSON objects
///
/// # Example
/// ```
/// let mut engine = TemplateEngine::new();
/// engine.set("state_dir", "/path/to/state");
/// engine.set("epic.id", "epic-001");
///
/// let result = engine.expand("Read {{state_dir}}/epics/{{epic.id}}/tickets.json");
/// // Returns: "Read /path/to/state/epics/epic-001/tickets.json"
/// ```
#[derive(Debug, Clone, Default)]
pub struct TemplateEngine {
    variables: HashMap<String, String>,
}

impl TemplateEngine {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    /// Set a template variable
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.variables.insert(key.into(), value.into());
    }

    /// Convenience method to set the state_dir variable
    pub fn set_state_dir(&mut self, path: &str) {
        self.set("state_dir", path);
    }

    /// Set variables from a JSON value, prefixed with a namespace.
    ///
    /// For a JSON object like `{"id": "T001", "title": "Add login"}`,
    /// calling `set_from_json("ticket", &value)` will set:
    /// - `ticket.id` = "T001"
    /// - `ticket.title` = "Add login"
    pub fn set_from_json(&mut self, prefix: &str, value: &serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                for (key, val) in map {
                    let full_key = format!("{}.{}", prefix, key);
                    self.set_json_value(&full_key, val);
                }
            }
            serde_json::Value::String(s) => self.set(prefix, s.clone()),
            serde_json::Value::Number(n) => self.set(prefix, n.to_string()),
            serde_json::Value::Bool(b) => self.set(prefix, b.to_string()),
            serde_json::Value::Null => self.set(prefix, "null"),
            serde_json::Value::Array(_) => {
                // Arrays are serialized as JSON strings
                if let Ok(s) = serde_json::to_string(value) {
                    self.set(prefix, s);
                }
            }
        }
    }

    fn set_json_value(&mut self, key: &str, value: &serde_json::Value) {
        match value {
            serde_json::Value::String(s) => self.set(key, s.clone()),
            serde_json::Value::Number(n) => self.set(key, n.to_string()),
            serde_json::Value::Bool(b) => self.set(key, b.to_string()),
            serde_json::Value::Null => self.set(key, "null"),
            serde_json::Value::Object(map) => {
                // Recursively handle nested objects
                for (nested_key, nested_val) in map {
                    let full_key = format!("{}.{}", key, nested_key);
                    self.set_json_value(&full_key, nested_val);
                }
            }
            serde_json::Value::Array(_) => {
                if let Ok(s) = serde_json::to_string(value) {
                    self.set(key, s);
                }
            }
        }
    }

    /// Merge another template engine's variables into this one
    pub fn merge(&mut self, other: &TemplateEngine) {
        for (key, value) in &other.variables {
            self.variables.insert(key.clone(), value.clone());
        }
    }

    /// Expand template variables in a string.
    ///
    /// Variables are specified as `{{variable_name}}` and will be replaced
    /// with their values. Unknown variables are left unchanged.
    pub fn expand(&self, template: &str) -> String {
        let mut result = template.to_string();

        // Sort keys by length (longest first) to handle nested keys correctly
        // e.g., {{ticket.id}} should be replaced before {{ticket}}
        let mut keys: Vec<_> = self.variables.keys().collect();
        keys.sort_by(|a, b| b.len().cmp(&a.len()));

        for key in keys {
            if let Some(value) = self.variables.get(key) {
                let pattern = format!("{{{{{}}}}}", key);
                result = result.replace(&pattern, value);
            }
        }

        result
    }

    /// Get a variable value by key
    pub fn get(&self, key: &str) -> Option<&String> {
        self.variables.get(key)
    }

    /// Check if a variable exists
    pub fn contains(&self, key: &str) -> bool {
        self.variables.contains_key(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_expansion() {
        let mut engine = TemplateEngine::new();
        engine.set("state_dir", "/path/to/state");

        let result = engine.expand("Read file at {{state_dir}}/spec.md");
        assert_eq!(result, "Read file at /path/to/state/spec.md");
    }

    #[test]
    fn test_multiple_variables() {
        let mut engine = TemplateEngine::new();
        engine.set("state_dir", "/state");
        engine.set("epic_id", "epic-001");
        engine.set("ticket_id", "T001");

        let result = engine.expand("{{state_dir}}/epics/{{epic_id}}/tickets/{{ticket_id}}");
        assert_eq!(result, "/state/epics/epic-001/tickets/T001");
    }

    #[test]
    fn test_nested_json_expansion() {
        let mut engine = TemplateEngine::new();
        let json: serde_json::Value = serde_json::json!({
            "id": "T001",
            "title": "Add login feature"
        });
        engine.set_from_json("ticket", &json);

        let result = engine.expand("Implement {{ticket.id}}: {{ticket.title}}");
        assert_eq!(result, "Implement T001: Add login feature");
    }

    #[test]
    fn test_unknown_variable_unchanged() {
        let engine = TemplateEngine::new();
        let result = engine.expand("Value is {{unknown}}");
        assert_eq!(result, "Value is {{unknown}}");
    }

    #[test]
    fn test_merge() {
        let mut engine1 = TemplateEngine::new();
        engine1.set("a", "1");

        let mut engine2 = TemplateEngine::new();
        engine2.set("b", "2");

        engine1.merge(&engine2);

        assert_eq!(engine1.get("a"), Some(&"1".to_string()));
        assert_eq!(engine1.get("b"), Some(&"2".to_string()));
    }
}
