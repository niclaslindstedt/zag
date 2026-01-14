use std::collections::HashMap;

use super::template::TemplateEngine;
use super::types::DefinitionValue;

/// Formats workflow definitions into a markdown string for system prompt injection.
///
/// Returns `None` if definitions is empty.
/// Flat definitions are listed first, then sections (both alphabetically sorted).
/// Template variables in definition values are expanded.
pub fn format_definitions(
    definitions: &HashMap<String, DefinitionValue>,
    template: &TemplateEngine,
) -> Option<String> {
    if definitions.is_empty() {
        return None;
    }

    let mut output = String::from("## Definitions\n\n");
    let mut sections: Vec<(&String, &HashMap<String, String>)> = Vec::new();

    // Collect and sort keys
    let mut keys: Vec<&String> = definitions.keys().collect();
    keys.sort();

    // First pass: collect flat definitions and sections
    for key in &keys {
        match definitions.get(*key).unwrap() {
            DefinitionValue::Simple(value) => {
                let expanded = template.expand(value);
                output.push_str(&format!("**{}**: {}\n\n", key, expanded));
            }
            DefinitionValue::Section(section) => {
                sections.push((key, section));
            }
        }
    }

    // Second pass: output sections
    for (section_name, section_defs) in sections {
        // Convert section name to title case for header
        let title = to_title_case(section_name);
        output.push_str(&format!("### {}\n\n", title));

        // Sort section keys
        let mut section_keys: Vec<&String> = section_defs.keys().collect();
        section_keys.sort();

        for key in section_keys {
            let value = section_defs.get(key).unwrap();
            let expanded = template.expand(value);
            output.push_str(&format!("**{}**: {}\n\n", key, expanded));
        }
    }

    // Trim trailing newlines
    let result = output.trim_end().to_string();

    if result == "## Definitions" {
        // Only header, no actual definitions
        None
    } else {
        Some(result)
    }
}

/// Converts a snake_case or kebab-case string to Title Case.
fn to_title_case(s: &str) -> String {
    s.split(['_', '-'])
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_title_case() {
        assert_eq!(to_title_case("hello_world"), "Hello World");
        assert_eq!(to_title_case("code-style"), "Code Style");
        assert_eq!(to_title_case("simple"), "Simple");
        assert_eq!(to_title_case("UPPER"), "UPPER");
    }

    #[test]
    fn test_empty_definitions() {
        let defs: HashMap<String, DefinitionValue> = HashMap::new();
        let template = TemplateEngine::new();
        assert!(format_definitions(&defs, &template).is_none());
    }

    #[test]
    fn test_flat_definitions() {
        let mut defs = HashMap::new();
        defs.insert(
            "epic".to_string(),
            DefinitionValue::Simple("A large feature".to_string()),
        );
        defs.insert(
            "ticket".to_string(),
            DefinitionValue::Simple("A small unit of work".to_string()),
        );

        let template = TemplateEngine::new();
        let result = format_definitions(&defs, &template).unwrap();

        assert!(result.contains("## Definitions"));
        assert!(result.contains("**epic**: A large feature"));
        assert!(result.contains("**ticket**: A small unit of work"));
    }

    #[test]
    fn test_nested_definitions() {
        let mut section = HashMap::new();
        section.insert("nested_term".to_string(), "A nested definition".to_string());

        let mut defs = HashMap::new();
        defs.insert("terms".to_string(), DefinitionValue::Section(section));

        let template = TemplateEngine::new();
        let result = format_definitions(&defs, &template).unwrap();

        assert!(result.contains("### Terms"));
        assert!(result.contains("**nested_term**: A nested definition"));
    }

    #[test]
    fn test_mixed_definitions() {
        let mut section = HashMap::new();
        section.insert("nested".to_string(), "Nested value".to_string());

        let mut defs = HashMap::new();
        defs.insert(
            "flat".to_string(),
            DefinitionValue::Simple("Flat value".to_string()),
        );
        defs.insert(
            "section_name".to_string(),
            DefinitionValue::Section(section),
        );

        let template = TemplateEngine::new();
        let result = format_definitions(&defs, &template).unwrap();

        // Flat definitions come first
        let flat_pos = result.find("**flat**").unwrap();
        let section_pos = result.find("### Section Name").unwrap();
        assert!(
            flat_pos < section_pos,
            "Flat definitions should come before sections"
        );
    }

    #[test]
    fn test_template_expansion() {
        let mut defs = HashMap::new();
        defs.insert(
            "path".to_string(),
            DefinitionValue::Simple("Output to {{state_dir}}".to_string()),
        );

        let mut template = TemplateEngine::new();
        template.set("state_dir", "/tmp/test".to_string());

        let result = format_definitions(&defs, &template).unwrap();
        assert!(result.contains("**path**: Output to /tmp/test"));
    }
}
