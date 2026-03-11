//! JSON Schema validation for tool inputs.

use anyhow::{Context, Result};
use jsonschema::{Draft, JSONSchema};
use serde_json::Value;

use super::tools::ToolDefinition;

/// Validation error details.
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Path to the invalid field (e.g., "video_id").
    pub path: String,
    /// Error message.
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.path.is_empty() {
            write!(f, "{}", self.message)
        } else {
            write!(f, "{}: {}", self.path, self.message)
        }
    }
}

/// Validation result containing all errors.
#[derive(Debug)]
pub struct ValidationResult {
    /// Whether validation passed.
    pub valid: bool,
    /// List of validation errors.
    pub errors: Vec<ValidationError>,
}

impl ValidationResult {
    /// Create a successful validation result.
    pub fn ok() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
        }
    }

    /// Create a failed validation result with errors.
    pub fn err(errors: Vec<ValidationError>) -> Self {
        Self {
            valid: false,
            errors,
        }
    }

    /// Format errors as a human-readable string.
    pub fn format_errors(&self) -> String {
        self.errors
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("; ")
    }
}

/// Tool input validator using JSON Schema.
pub struct ToolValidator {
    /// Compiled JSON Schema.
    schema: JSONSchema,
    /// Tool name for error messages.
    tool_name: String,
}

impl ToolValidator {
    /// Create a validator from a tool definition.
    pub fn from_definition(definition: &ToolDefinition) -> Result<Self> {
        let schema_value = definition.to_input_schema();
        let schema = JSONSchema::options()
            .with_draft(Draft::Draft7)
            .compile(&schema_value)
            .map_err(|e| anyhow::anyhow!("Failed to compile schema for {}: {}", definition.name, e))?;

        Ok(Self {
            schema,
            tool_name: definition.name.clone(),
        })
    }

    /// Validate input against the schema.
    pub fn validate(&self, input: &Value) -> ValidationResult {
        match self.schema.validate(input) {
            Ok(_) => ValidationResult::ok(),
            Err(errors) => {
                let validation_errors: Vec<ValidationError> = errors
                    .map(|error| {
                        let path = format!("{}", error.instance_path);
                        ValidationError {
                            path,
                            message: error.to_string(),
                        }
                    })
                    .collect();
                ValidationResult::err(validation_errors)
            }
        }
    }

    /// Validate and return a Result for easier error handling.
    pub fn validate_or_err(&self, input: &Value) -> Result<()> {
        let result = self.validate(input);
        if result.valid {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Invalid input for tool '{}': {}",
                self.tool_name,
                result.format_errors()
            ))
        }
    }
}

impl ToolDefinition {
    /// Convert to JSON Schema for input validation.
    pub fn to_input_schema(&self) -> Value {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();

        for param in &self.parameters {
            let mut prop = serde_json::Map::new();
            prop.insert("type".to_string(), Value::String(param.param_type.clone()));
            prop.insert(
                "description".to_string(),
                Value::String(param.description.clone()),
            );

            if let Some(ref default) = param.default {
                prop.insert("default".to_string(), default.clone());
            }

            if let Some(ref enum_vals) = param.enum_values {
                prop.insert("enum".to_string(), Value::Array(enum_vals.clone()));
            }

            properties.insert(param.name.clone(), Value::Object(prop));

            if param.required {
                required.push(Value::String(param.name.clone()));
            }
        }

        serde_json::json!({
            "type": "object",
            "properties": properties,
            "required": required,
            "additionalProperties": false
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::tools::{ParameterDef, ToolDefinition};

    fn create_test_definition() -> ToolDefinition {
        ToolDefinition::new("test_tool", "A test tool")
            .with_param(ParameterDef::required_string("query", "Search query"))
            .with_param(ParameterDef::optional_integer("limit", "Max results", 10))
            .with_param(ParameterDef {
                name: "format".to_string(),
                param_type: "string".to_string(),
                description: "Output format".to_string(),
                required: false,
                default: Some(Value::String("json".to_string())),
                enum_values: Some(vec![
                    Value::String("json".to_string()),
                    Value::String("xml".to_string()),
                ]),
            })
    }

    #[test]
    fn test_validator_valid_input() {
        let def = create_test_definition();
        let validator = ToolValidator::from_definition(&def).unwrap();

        let input = serde_json::json!({
            "query": "test search",
            "limit": 5
        });

        let result = validator.validate(&input);
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validator_missing_required() {
        let def = create_test_definition();
        let validator = ToolValidator::from_definition(&def).unwrap();

        // Missing required 'query' field
        let input = serde_json::json!({
            "limit": 5
        });

        let result = validator.validate(&input);
        assert!(!result.valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_validator_wrong_type() {
        let def = create_test_definition();
        let validator = ToolValidator::from_definition(&def).unwrap();

        // 'limit' should be integer, not string
        let input = serde_json::json!({
            "query": "test",
            "limit": "not a number"
        });

        let result = validator.validate(&input);
        assert!(!result.valid);
    }

    #[test]
    fn test_validator_invalid_enum() {
        let def = create_test_definition();
        let validator = ToolValidator::from_definition(&def).unwrap();

        // 'format' must be "json" or "xml"
        let input = serde_json::json!({
            "query": "test",
            "format": "csv"
        });

        let result = validator.validate(&input);
        assert!(!result.valid);
    }

    #[test]
    fn test_validator_or_err() {
        let def = create_test_definition();
        let validator = ToolValidator::from_definition(&def).unwrap();

        let valid_input = serde_json::json!({"query": "test"});
        assert!(validator.validate_or_err(&valid_input).is_ok());

        let invalid_input = serde_json::json!({});
        assert!(validator.validate_or_err(&invalid_input).is_err());
    }

    #[test]
    fn test_input_schema_generation() {
        let def = create_test_definition();
        let schema = def.to_input_schema();

        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["query"].is_object());
        assert!(schema["required"].as_array().unwrap().contains(&Value::String("query".to_string())));
    }
}
