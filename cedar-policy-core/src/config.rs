/*
 * Copyright Cedar Contributors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      https://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

//! Configuration module for multi-valued decision types
//!
//! This module provides configuration loading and validation for custom
//! decision types beyond Cedar's built-in permit/forbid (allow/deny).

use serde::Deserialize;
use std::fs;
use std::io;
use std::path::Path;
use thiserror::Error;

/// Configuration structure loaded from YAML
///
/// Defines all decision types available in the authorization system.
/// Must include at minimum "allow" and "deny" decision types.
#[derive(Debug, Clone, Deserialize)]
pub struct DecisionConfig {
    /// List of decision type configurations
    pub decision_types: Vec<DecisionTypeConfig>,

    /// Combination rules defining how decision types interact
    #[serde(default)]
    pub combination_rules: Vec<CombinationRule>,

    /// Conflict resolution strategy (currently only "precedence" supported)
    #[serde(default = "default_conflict_resolution")]
    pub conflict_resolution: String,
}

fn default_conflict_resolution() -> String {
    "precedence".to_string()
}

/// Re-export CombinationRule from decision_registry for use in config
pub use crate::entities::decision_registry::CombinationRule;

/// Individual decision type configuration
///
/// Each decision type has properties that control:
/// - Precedence: Higher values win in conflict resolution
#[derive(Debug, Clone, Deserialize)]
pub struct DecisionTypeConfig {
    /// Name of the decision type (e.g., "allow", "deny", "alert")
    /// Must be lowercase alphanumeric + underscore, 1-32 characters
    pub name: String,

    /// Priority for conflict resolution (higher = higher priority)
    pub precedence: u32,
}

impl DecisionConfig {
    /// Load configuration from a YAML file
    ///
    /// Performs fail-fast validation - any error results in immediate failure.
    /// The configuration file MUST exist at startup.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` if:
    /// - File does not exist or cannot be read
    /// - YAML parsing fails
    /// - Validation fails (duplicate names, missing required types, invalid names)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use cedar_policy_core::config::DecisionConfig;
    ///
    /// let config = DecisionConfig::from_file("decision_config.yaml")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| match e.kind() {
                io::ErrorKind::NotFound => ConfigError::FileNotFound {
                    path: path.as_ref().to_string_lossy().to_string(),
                },
                _ => ConfigError::Io(e),
            })?;

        Self::from_str(&content)
    }

    /// Load configuration from a YAML string
    ///
    /// Performs fail-fast validation on the parsed configuration.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` if:
    /// - YAML parsing fails
    /// - Validation fails
    pub fn from_str(yaml: &str) -> Result<Self, ConfigError> {
        let config: DecisionConfig = serde_yaml::from_str(yaml)
            .map_err(ConfigError::YamlParse)?;

        config.validate()?;
        Ok(config)
    }

    /// Validate configuration semantics
    ///
    /// Checks:
    /// - All decision type names are valid format
    /// - No duplicate names
    /// - Required "allow" and "deny" types are present
    /// - No decision is both exclusive and combinable
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` with specific validation failure
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Check for empty config
        if self.decision_types.is_empty() {
            return Err(ConfigError::ValidationError {
                message: "Configuration must define at least one decision type".to_string(),
            });
        }

        // Track seen names to detect duplicates
        let mut seen_names = std::collections::HashSet::new();

        // Validate each decision type
        for dt in &self.decision_types {
            // Check name format
            Self::validate_name(&dt.name)?;

            // Check for duplicates
            if !seen_names.insert(dt.name.clone()) {
                return Err(ConfigError::DuplicateName {
                    name: dt.name.clone(),
                });
            }
        }

        // Verify required built-in types are present
        if !seen_names.contains("allow") {
            return Err(ConfigError::MissingRequiredType {
                name: "allow".to_string(),
            });
        }
        if !seen_names.contains("deny") {
            return Err(ConfigError::MissingRequiredType {
                name: "deny".to_string(),
            });
        }

        Ok(())
    }

    /// Validate decision type name format
    ///
    /// Rules:
    /// - 1-32 characters
    /// - Lowercase letters, digits, underscores only
    /// - Must start with a lowercase letter
    fn validate_name(name: &str) -> Result<(), ConfigError> {
        if name.is_empty() || name.len() > 32 {
            return Err(ConfigError::InvalidName {
                name: name.to_string(),
                reason: "Name must be 1-32 characters".to_string(),
            });
        }

        if !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
            return Err(ConfigError::InvalidName {
                name: name.to_string(),
                reason: "Name must contain only lowercase letters, digits, and underscores".to_string(),
            });
        }

        if !name.chars().next().unwrap().is_ascii_lowercase() {
            return Err(ConfigError::InvalidName {
                name: name.to_string(),
                reason: "Name must start with a lowercase letter".to_string(),
            });
        }

        Ok(())
    }
}

/// Configuration and validation errors
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Configuration file not found at specified path
    #[error("Configuration file not found: {path}")]
    FileNotFound {
        /// Path that was attempted
        path: String,
    },

    /// Decision type name is duplicated in configuration
    #[error("Decision type name '{name}' is duplicated")]
    DuplicateName {
        /// The duplicate name
        name: String,
    },

    /// Decision type name has invalid format
    #[error("Invalid decision type name '{name}': {reason}")]
    InvalidName {
        /// The invalid name
        name: String,
        /// Reason for invalidity
        reason: String,
    },

    /// Required decision type is missing from configuration
    #[error("Missing required decision type '{name}'")]
    MissingRequiredType {
        /// Name of the missing required type
        name: String,
    },

    /// Generic validation error
    #[error("Validation error: {message}")]
    ValidationError {
        /// Description of the validation failure
        message: String,
    },

    /// IO error reading configuration file
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// YAML parsing error
    #[error("YAML parse error: {0}")]
    YamlParse(#[from] serde_yaml::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_minimal_config() {
        let yaml = r#"
decision_types:
  - name: allow
    precedence: 100
  - name: deny
    precedence: 200
"#;

        let result = DecisionConfig::from_str(yaml);
        assert!(result.is_ok());
    }

    #[test]
    fn test_missing_required_allow() {
        let yaml = r#"
decision_types:
  - name: deny
    precedence: 200
"#;

        let result = DecisionConfig::from_str(yaml);
        assert!(matches!(result, Err(ConfigError::MissingRequiredType { .. })));
    }

    #[test]
    fn test_duplicate_name() {
        let yaml = r#"
decision_types:
  - name: allow
    precedence: 100
  - name: deny
    precedence: 200
  - name: allow
    precedence: 50
"#;

        let result = DecisionConfig::from_str(yaml);
        assert!(matches!(result, Err(ConfigError::DuplicateName { .. })));
    }

    #[test]
    fn test_invalid_name_uppercase() {
        let yaml = r#"
decision_types:
  - name: allow
    precedence: 100
  - name: deny
    precedence: 200
  - name: Alert
    precedence: 50
"#;

        let result = DecisionConfig::from_str(yaml);
        assert!(matches!(result, Err(ConfigError::InvalidName { .. })));
    }

    #[test]
    fn test_file_not_found() {
        let result = DecisionConfig::from_file("/nonexistent/path/config.yaml");
        assert!(matches!(result, Err(ConfigError::FileNotFound { .. })));

        if let Err(ConfigError::FileNotFound { path }) = result {
            assert!(path.contains("nonexistent"));
        }
    }

    #[test]
    fn test_invalid_yaml() {
        let yaml = r#"
decision_types:
  - name: allow
    precedence: not_a_number
    combinable: true
"#;

        let result = DecisionConfig::from_str(yaml);
        assert!(matches!(result, Err(ConfigError::YamlParse(..))));
    }

    #[test]
    fn test_empty_config() {
        let yaml = r#"
decision_types: []
"#;

        let result = DecisionConfig::from_str(yaml);
        assert!(matches!(result, Err(ConfigError::ValidationError { .. })));
    }

    #[test]
    fn test_name_validation_too_long() {
        let long_name = "a".repeat(33);
        let yaml = format!(
            r#"
decision_types:
  - name: allow
    precedence: 100
  - name: deny
    precedence: 200
  - name: {}
    precedence: 50
"#,
            long_name
        );

        let result = DecisionConfig::from_str(&yaml);
        assert!(matches!(result, Err(ConfigError::InvalidName { .. })));
    }

    #[test]
    fn test_name_validation_starts_with_digit() {
        let yaml = r#"
decision_types:
  - name: allow
    precedence: 100
  - name: deny
    precedence: 200
  - name: 9alert
    precedence: 50
"#;

        let result = DecisionConfig::from_str(yaml);
        assert!(matches!(result, Err(ConfigError::InvalidName { .. })));
    }

    #[test]
    fn test_name_validation_special_chars() {
        let yaml = r#"
decision_types:
  - name: allow
    precedence: 100
  - name: deny
    precedence: 200
  - name: alert-notify
    precedence: 50
"#;

        let result = DecisionConfig::from_str(yaml);
        assert!(matches!(result, Err(ConfigError::InvalidName { .. })));
    }

    #[test]
    fn test_valid_custom_decision_types() {
        let yaml = r#"
decision_types:
  - name: allow
    precedence: 100
  - name: deny
    precedence: 200
  - name: alert
    precedence: 50
  - name: validate
    precedence: 60
  - name: audit_log
    precedence: 40
"#;

        let result = DecisionConfig::from_str(yaml);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.decision_types.len(), 5);
    }

    #[test]
    fn test_missing_deny() {
        let yaml = r#"
decision_types:
  - name: allow
    precedence: 100
"#;

        let result = DecisionConfig::from_str(yaml);
        assert!(matches!(result, Err(ConfigError::MissingRequiredType { .. })));
    }

    #[test]
    fn test_config_with_combination_rules() {
        let yaml = r#"
decision_types:
  - name: allow
    precedence: 100
  - name: deny
    precedence: 200
  - name: alert
    precedence: 50

combination_rules:
  - when: [deny, "*"]
    then: exclusive
    result: [deny]
  - when: [allow, alert]
    then: merge

conflict_resolution: precedence
"#;

        let result = DecisionConfig::from_str(yaml);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.decision_types.len(), 3);
        assert_eq!(config.combination_rules.len(), 2);
        assert_eq!(config.conflict_resolution, "precedence");

        // Verify first rule (deny exclusive)
        assert_eq!(config.combination_rules[0].when, vec!["deny", "*"]);
        assert_eq!(
            config.combination_rules[0].then,
            crate::entities::decision_registry::CombinationStrategy::Exclusive
        );
        assert_eq!(
            config.combination_rules[0].result,
            Some(vec!["deny".to_string()])
        );

        // Verify second rule (allow + alert merge)
        assert_eq!(config.combination_rules[1].when, vec!["allow", "alert"]);
        assert_eq!(
            config.combination_rules[1].then,
            crate::entities::decision_registry::CombinationStrategy::Merge
        );
        assert_eq!(config.combination_rules[1].result, None);
    }

    #[test]
    fn test_config_precedence_resolution() {
        use crate::entities::decision_registry::DecisionTypeRegistry;

        let yaml = r#"
decision_types:
  - name: allow
    precedence: 100
  - name: deny
    precedence: 200
  - name: alert
    precedence: 50
  - name: validate
    precedence: 60

combination_rules:
  - when: [deny, "*"]
    then: exclusive
    result: [deny]

conflict_resolution: precedence
"#;

        let config = DecisionConfig::from_str(yaml).expect("Config should parse");
        let registry = DecisionTypeRegistry::from_config(&config);

        // Test that deny (precedence 200) > allow (100) > validate (60) > alert (50)
        let allow_id = registry.get_id("allow").unwrap();
        let deny_id = registry.get_id("deny").unwrap();
        let alert_id = registry.get_id("alert").unwrap();
        let validate_id = registry.get_id("validate").unwrap();

        // When deny is present with others, only deny should remain after resolve
        let resolved = registry.resolve(&[allow_id, deny_id, alert_id, validate_id]);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0], deny_id);

        // Without deny, all should remain
        let resolved = registry.resolve(&[allow_id, alert_id, validate_id]);
        assert_eq!(resolved.len(), 3);
    }

    #[test]
    fn test_config_defaults() {
        let yaml = r#"
decision_types:
  - name: allow
    precedence: 100
  - name: deny
    precedence: 200
"#;

        let config = DecisionConfig::from_str(yaml).expect("Config should parse");

        // combination_rules should default to empty vec
        assert_eq!(config.combination_rules.len(), 0);

        // conflict_resolution should default to "precedence"
        assert_eq!(config.conflict_resolution, "precedence");
    }
}
