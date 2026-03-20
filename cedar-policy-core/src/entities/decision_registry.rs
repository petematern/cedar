// Multi-Valued Decision Registry
// Manages custom decision types beyond binary permit/forbid

use crate::config::DecisionConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for a decision type
/// Uses newtype pattern for type safety over raw u32
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DecisionTypeId(pub(crate) u32);

impl DecisionTypeId {
    /// Reserved ID for built-in "allow" decision type
    pub const ALLOW: DecisionTypeId = DecisionTypeId(0);

    /// Reserved ID for built-in "deny" decision type
    pub const DENY: DecisionTypeId = DecisionTypeId(1);

    /// First ID available for custom decision types
    pub const CUSTOM_START: u32 = 100;
}

/// Metadata describing a decision type's properties and behavior
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionTypeMetadata {
    /// Unique identifier
    pub id: DecisionTypeId,

    /// Human-readable name (e.g., "allow", "deny", "alert", "validate", "audit")
    pub name: String,

    /// Priority for conflict resolution (higher = higher priority)
    pub precedence: u32,

    /// Whether this decision can coexist with other decisions
    pub combinable: bool,

    /// Whether this decision excludes other decisions when present
    pub exclusive: bool,
}

/// Pattern for matching decision types in combination rules
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DecisionPattern {
    /// Match a specific decision type by name
    Specific(String),
    /// Wildcard matching any decision type
    Wildcard,
}

impl DecisionPattern {
    /// Check if this pattern matches a decision type name
    pub fn matches(&self, name: &str) -> bool {
        match self {
            DecisionPattern::Specific(pattern_name) => pattern_name == name,
            DecisionPattern::Wildcard => true,
        }
    }
}

/// Strategy for combining multiple decision types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CombinationStrategy {
    /// Merge decisions - both can coexist
    Merge,
    /// Exclusive - one decision excludes others, only result decisions remain
    Exclusive,
    /// Override - one decision overrides another
    Override,
}

/// Rule defining how decision types combine or conflict
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombinationRule {
    /// Patterns matching the decision types this rule applies to
    pub when: Vec<String>,

    /// Strategy for combining these decisions
    pub then: CombinationStrategy,

    /// Resulting decisions (for Exclusive strategy)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Vec<String>>,
}

impl CombinationRule {
    /// Check if this rule matches a set of decision type names
    ///
    /// A rule matches if all patterns in the `when` clause match at least
    /// one decision type in the provided set.
    pub fn matches(&self, decision_names: &[&str]) -> bool {
        if self.when.is_empty() {
            return false;
        }

        // Check if all patterns in the rule match at least one decision
        self.when.iter().all(|pattern_str| {
            let pattern = if pattern_str == "*" {
                DecisionPattern::Wildcard
            } else {
                DecisionPattern::Specific(pattern_str.clone())
            };

            decision_names.iter().any(|name| pattern.matches(name))
        })
    }

    /// Apply this combination rule to a set of decision types
    ///
    /// Returns the resulting decision type names after applying the rule.
    /// For Exclusive rules, returns only the `result` decisions.
    /// For Merge rules, returns all input decisions unchanged.
    pub fn apply(&self, decision_names: Vec<String>) -> Vec<String> {
        match self.then {
            CombinationStrategy::Exclusive => {
                // Return only the result decisions
                self.result.clone().unwrap_or_default()
            }
            CombinationStrategy::Merge => {
                // Keep all decisions
                decision_names
            }
            CombinationStrategy::Override => {
                // For now, treat Override like Exclusive
                // TODO: Implement proper override semantics in Phase 5
                self.result.clone().unwrap_or(decision_names)
            }
        }
    }
}

/// Central registry managing all configured decision types
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionTypeRegistry {
    /// Name → metadata lookup (O(1))
    types: HashMap<String, DecisionTypeMetadata>,

    /// ID → name mapping via indexing (O(1))
    id_to_name: Vec<String>,

    /// Decision types sorted by precedence (highest first)
    precedence_order: Vec<DecisionTypeId>,

    /// Combination rules defining how decision types interact
    combination_rules: Vec<CombinationRule>,
}

impl DecisionTypeRegistry {
    /// Create registry from configuration
    ///
    /// Assumes configuration has already been validated via `DecisionConfig::validate()`.
    /// Builds internal lookup structures for O(1) access by name or ID.
    pub fn from_config(config: &DecisionConfig) -> Self {
        let mut types = HashMap::new();
        let mut id_to_name = Vec::new();
        let mut next_id = DecisionTypeId::CUSTOM_START;

        // Reserve IDs for built-in types
        id_to_name.push("allow".to_string());
        id_to_name.push("deny".to_string());

        // Process decision types from config (validation already done)
        for dt_config in &config.decision_types {
            // Assign ID (built-in types get reserved IDs, custom types get sequential)
            let id = match dt_config.name.as_str() {
                "allow" => DecisionTypeId::ALLOW,
                "deny" => DecisionTypeId::DENY,
                _ => {
                    let id = DecisionTypeId(next_id);
                    next_id += 1;
                    // Grow id_to_name vec as needed
                    while id_to_name.len() <= id.0 as usize {
                        id_to_name.push(String::new());
                    }
                    id_to_name[id.0 as usize] = dt_config.name.clone();
                    id
                }
            };

            let metadata = DecisionTypeMetadata {
                id,
                name: dt_config.name.clone(),
                precedence: dt_config.precedence,
                combinable: dt_config.combinable,
                exclusive: dt_config.exclusive,
            };

            types.insert(dt_config.name.clone(), metadata);
        }

        // Build precedence-sorted order (highest precedence first)
        let mut precedence_order: Vec<DecisionTypeId> = types.values()
            .map(|m| m.id)
            .collect();
        precedence_order.sort_by(|a, b| {
            let meta_a = types.values().find(|m| m.id == *a).unwrap();
            let meta_b = types.values().find(|m| m.id == *b).unwrap();
            meta_b.precedence.cmp(&meta_a.precedence) // Descending order
        });

        Self {
            types,
            id_to_name,
            precedence_order,
            combination_rules: config.combination_rules.clone(),
        }
    }

    /// Create minimal registry with only allow/deny (for testing)
    pub fn default() -> Self {
        let mut types = HashMap::new();

        types.insert("allow".to_string(), DecisionTypeMetadata {
            id: DecisionTypeId::ALLOW,
            name: "allow".to_string(),
            precedence: 100,
            combinable: true,
            exclusive: false,
        });

        types.insert("deny".to_string(), DecisionTypeMetadata {
            id: DecisionTypeId::DENY,
            name: "deny".to_string(),
            precedence: 200,
            combinable: false,
            exclusive: true,
        });

        let id_to_name = vec!["allow".to_string(), "deny".to_string()];
        let precedence_order = vec![DecisionTypeId::DENY, DecisionTypeId::ALLOW];

        Self {
            types,
            id_to_name,
            precedence_order,
            combination_rules: Vec::new(),
        }
    }

    /// Resolve a set of decision types by applying combination rules
    ///
    /// Takes a set of decision type IDs and applies the configured combination
    /// rules to determine which decisions should remain in the final result.
    ///
    /// Returns the filtered set of decision type IDs after applying all rules.
    pub fn resolve(&self, decision_ids: &[DecisionTypeId]) -> Vec<DecisionTypeId> {
        if decision_ids.is_empty() {
            return Vec::new();
        }

        // Convert IDs to names for rule matching
        let mut decision_names: Vec<String> = decision_ids
            .iter()
            .filter_map(|id| self.get_name(*id).map(|s| s.to_string()))
            .collect();

        // Apply combination rules in order
        for rule in &self.combination_rules {
            let name_refs: Vec<&str> = decision_names.iter().map(|s| s.as_str()).collect();
            if rule.matches(&name_refs) {
                decision_names = rule.apply(decision_names);
            }
        }

        // Convert names back to IDs
        decision_names
            .iter()
            .filter_map(|name| self.get_id(name))
            .collect()
    }

    /// Check if two decision types can be combined
    ///
    /// Returns true if both decisions can coexist according to their metadata
    /// and the configured combination rules.
    pub fn can_combine(&self, id1: DecisionTypeId, id2: DecisionTypeId) -> bool {
        let meta1 = match self.get_metadata(id1) {
            Some(m) => m,
            None => return false,
        };

        let meta2 = match self.get_metadata(id2) {
            Some(m) => m,
            None => return false,
        };

        // If either is exclusive, they cannot combine
        if meta1.exclusive || meta2.exclusive {
            return false;
        }

        // Both must be combinable
        if !meta1.combinable || !meta2.combinable {
            return false;
        }

        // Check combination rules for explicit merge or exclusive directives
        let names = [meta1.name.as_str(), meta2.name.as_str()];
        for rule in &self.combination_rules {
            if rule.matches(&names) {
                match rule.then {
                    CombinationStrategy::Merge => return true,
                    CombinationStrategy::Exclusive => return false,
                    CombinationStrategy::Override => return false,
                }
            }
        }

        // Default: allow combination if both are combinable
        true
    }

    /// Get decision type ID by name
    pub fn get_id(&self, name: &str) -> Option<DecisionTypeId> {
        self.types.get(name).map(|m| m.id)
    }

    /// Get decision type name by ID
    pub fn get_name(&self, id: DecisionTypeId) -> Option<&str> {
        self.id_to_name.get(id.0 as usize).map(|s| s.as_str())
    }

    /// Get complete metadata for a decision type
    pub fn get_metadata(&self, id: DecisionTypeId) -> Option<&DecisionTypeMetadata> {
        let name = self.get_name(id)?;
        self.types.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DecisionTypeConfig;

    fn minimal_config() -> DecisionConfig {
        DecisionConfig {
            decision_types: vec![
                DecisionTypeConfig {
                    name: "allow".to_string(),
                    precedence: 100,
                    combinable: true,
                    exclusive: false,
                },
                DecisionTypeConfig {
                    name: "deny".to_string(),
                    precedence: 200,
                    combinable: false,
                    exclusive: true,
                },
            ],
            combination_rules: Vec::new(),
            conflict_resolution: "precedence".to_string(),
        }
    }

    fn extended_config() -> DecisionConfig {
        DecisionConfig {
            decision_types: vec![
                DecisionTypeConfig {
                    name: "allow".to_string(),
                    precedence: 100,
                    combinable: true,
                    exclusive: false,
                },
                DecisionTypeConfig {
                    name: "deny".to_string(),
                    precedence: 200,
                    combinable: false,
                    exclusive: true,
                },
                DecisionTypeConfig {
                    name: "alert".to_string(),
                    precedence: 50,
                    combinable: true,
                    exclusive: false,
                },
                DecisionTypeConfig {
                    name: "validate".to_string(),
                    precedence: 60,
                    combinable: true,
                    exclusive: false,
                },
            ],
            combination_rules: Vec::new(),
            conflict_resolution: "precedence".to_string(),
        }
    }

    #[test]
    fn test_default_registry() {
        let registry = DecisionTypeRegistry::default();

        // Check built-in types exist
        assert_eq!(registry.get_id("allow"), Some(DecisionTypeId::ALLOW));
        assert_eq!(registry.get_id("deny"), Some(DecisionTypeId::DENY));

        // Check reverse lookup
        assert_eq!(registry.get_name(DecisionTypeId::ALLOW), Some("allow"));
        assert_eq!(registry.get_name(DecisionTypeId::DENY), Some("deny"));

        // Check metadata
        let allow_meta = registry.get_metadata(DecisionTypeId::ALLOW).unwrap();
        assert_eq!(allow_meta.name, "allow");
        assert_eq!(allow_meta.precedence, 100);
        assert!(allow_meta.combinable);
        assert!(!allow_meta.exclusive);

        let deny_meta = registry.get_metadata(DecisionTypeId::DENY).unwrap();
        assert_eq!(deny_meta.name, "deny");
        assert_eq!(deny_meta.precedence, 200);
        assert!(!deny_meta.combinable);
        assert!(deny_meta.exclusive);
    }

    #[test]
    fn test_from_config_minimal() {
        let config = minimal_config();
        let registry = DecisionTypeRegistry::from_config(&config);

        // Verify both required types present
        assert!(registry.get_id("allow").is_some());
        assert!(registry.get_id("deny").is_some());

        // Verify precedence order (deny should come first - highest precedence)
        assert_eq!(registry.precedence_order.len(), 2);
        assert_eq!(registry.precedence_order[0], DecisionTypeId::DENY);
        assert_eq!(registry.precedence_order[1], DecisionTypeId::ALLOW);
    }

    #[test]
    fn test_from_config_with_custom_types() {
        let config = extended_config();
        let registry = DecisionTypeRegistry::from_config(&config);

        // Check all decision types registered
        assert!(registry.get_id("allow").is_some());
        assert!(registry.get_id("deny").is_some());
        assert!(registry.get_id("alert").is_some());
        assert!(registry.get_id("validate").is_some());

        // Check custom types got IDs >= CUSTOM_START
        let alert_id = registry.get_id("alert").unwrap();
        let validate_id = registry.get_id("validate").unwrap();
        assert!(alert_id.0 >= DecisionTypeId::CUSTOM_START);
        assert!(validate_id.0 >= DecisionTypeId::CUSTOM_START);

        // Check reverse lookup works for custom types
        assert_eq!(registry.get_name(alert_id), Some("alert"));
        assert_eq!(registry.get_name(validate_id), Some("validate"));

        // Check precedence ordering: deny(200) > allow(100) > validate(60) > alert(50)
        assert_eq!(registry.precedence_order.len(), 4);
        assert_eq!(registry.precedence_order[0], DecisionTypeId::DENY);
        assert_eq!(registry.precedence_order[1], DecisionTypeId::ALLOW);
        assert_eq!(registry.precedence_order[2], validate_id);
        assert_eq!(registry.precedence_order[3], alert_id);
    }

    #[test]
    fn test_get_metadata_for_custom_type() {
        let config = extended_config();
        let registry = DecisionTypeRegistry::from_config(&config);

        let alert_id = registry.get_id("alert").unwrap();
        let alert_meta = registry.get_metadata(alert_id).unwrap();

        assert_eq!(alert_meta.name, "alert");
        assert_eq!(alert_meta.precedence, 50);
        assert!(alert_meta.combinable);
        assert!(!alert_meta.exclusive);
    }

    #[test]
    fn test_nonexistent_decision_type() {
        let registry = DecisionTypeRegistry::default();

        assert_eq!(registry.get_id("nonexistent"), None);
        assert_eq!(registry.get_name(DecisionTypeId(999)), None);
        assert_eq!(registry.get_metadata(DecisionTypeId(999)), None);
    }

    #[test]
    fn test_reserved_ids_consistent() {
        let config = minimal_config();
        let registry = DecisionTypeRegistry::from_config(&config);

        // Built-in types should always get their reserved IDs
        assert_eq!(
            registry.get_id("allow"),
            Some(DecisionTypeId::ALLOW)
        );
        assert_eq!(
            registry.get_id("deny"),
            Some(DecisionTypeId::DENY)
        );
    }

    // Combination rule tests
    #[test]
    fn test_decision_pattern_specific_match() {
        let pattern = DecisionPattern::Specific("allow".to_string());
        assert!(pattern.matches("allow"));
        assert!(!pattern.matches("deny"));
    }

    #[test]
    fn test_decision_pattern_wildcard_match() {
        let pattern = DecisionPattern::Wildcard;
        assert!(pattern.matches("allow"));
        assert!(pattern.matches("deny"));
        assert!(pattern.matches("anything"));
    }

    #[test]
    fn test_combination_rule_matches_specific() {
        let rule = CombinationRule {
            when: vec!["allow".to_string(), "alert".to_string()],
            then: CombinationStrategy::Merge,
            result: None,
        };

        assert!(rule.matches(&["allow", "alert"]));
        assert!(rule.matches(&["allow", "alert", "audit"]));
        assert!(!rule.matches(&["allow"]));
        assert!(!rule.matches(&["deny", "alert"]));
    }

    #[test]
    fn test_combination_rule_matches_wildcard() {
        let rule = CombinationRule {
            when: vec!["deny".to_string(), "*".to_string()],
            then: CombinationStrategy::Exclusive,
            result: Some(vec!["deny".to_string()]),
        };

        assert!(rule.matches(&["deny", "allow"]));
        assert!(rule.matches(&["deny", "alert"]));
        assert!(rule.matches(&["deny", "anything"]));
        assert!(!rule.matches(&["allow"]));
    }

    #[test]
    fn test_combination_rule_apply_merge() {
        let rule = CombinationRule {
            when: vec!["allow".to_string(), "alert".to_string()],
            then: CombinationStrategy::Merge,
            result: None,
        };

        let decisions = vec!["allow".to_string(), "alert".to_string()];
        let result = rule.apply(decisions.clone());
        assert_eq!(result, decisions);
    }

    #[test]
    fn test_combination_rule_apply_exclusive() {
        let rule = CombinationRule {
            when: vec!["deny".to_string(), "*".to_string()],
            then: CombinationStrategy::Exclusive,
            result: Some(vec!["deny".to_string()]),
        };

        let decisions = vec!["deny".to_string(), "allow".to_string(), "alert".to_string()];
        let result = rule.apply(decisions);
        assert_eq!(result, vec!["deny".to_string()]);
    }

    #[test]
    fn test_registry_resolve_with_exclusive_rule() {
        let config = DecisionConfig {
            decision_types: vec![
                DecisionTypeConfig {
                    name: "allow".to_string(),
                    precedence: 100,
                    combinable: true,
                    exclusive: false,
                },
                DecisionTypeConfig {
                    name: "deny".to_string(),
                    precedence: 200,
                    combinable: false,
                    exclusive: true,
                },
                DecisionTypeConfig {
                    name: "alert".to_string(),
                    precedence: 50,
                    combinable: true,
                    exclusive: false,
                },
            ],
            combination_rules: vec![
                CombinationRule {
                    when: vec!["deny".to_string(), "*".to_string()],
                    then: CombinationStrategy::Exclusive,
                    result: Some(vec!["deny".to_string()]),
                },
            ],
            conflict_resolution: "precedence".to_string(),
        };

        let registry = DecisionTypeRegistry::from_config(&config);

        // When deny is present, only deny should remain
        let allow_id = registry.get_id("allow").unwrap();
        let deny_id = registry.get_id("deny").unwrap();
        let alert_id = registry.get_id("alert").unwrap();

        let resolved = registry.resolve(&[allow_id, deny_id, alert_id]);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0], deny_id);
    }

    #[test]
    fn test_registry_resolve_merge_decisions() {
        let config = DecisionConfig {
            decision_types: vec![
                DecisionTypeConfig {
                    name: "allow".to_string(),
                    precedence: 100,
                    combinable: true,
                    exclusive: false,
                },
                DecisionTypeConfig {
                    name: "alert".to_string(),
                    precedence: 50,
                    combinable: true,
                    exclusive: false,
                },
            ],
            combination_rules: vec![
                CombinationRule {
                    when: vec!["allow".to_string(), "alert".to_string()],
                    then: CombinationStrategy::Merge,
                    result: None,
                },
            ],
            conflict_resolution: "precedence".to_string(),
        };

        let registry = DecisionTypeRegistry::from_config(&config);

        let allow_id = registry.get_id("allow").unwrap();
        let alert_id = registry.get_id("alert").unwrap();

        // Allow and alert should both remain
        let resolved = registry.resolve(&[allow_id, alert_id]);
        assert_eq!(resolved.len(), 2);
        assert!(resolved.contains(&allow_id));
        assert!(resolved.contains(&alert_id));
    }

    #[test]
    fn test_registry_can_combine_compatible() {
        let config = DecisionConfig {
            decision_types: vec![
                DecisionTypeConfig {
                    name: "allow".to_string(),
                    precedence: 100,
                    combinable: true,
                    exclusive: false,
                },
                DecisionTypeConfig {
                    name: "alert".to_string(),
                    precedence: 50,
                    combinable: true,
                    exclusive: false,
                },
            ],
            combination_rules: vec![
                CombinationRule {
                    when: vec!["allow".to_string(), "alert".to_string()],
                    then: CombinationStrategy::Merge,
                    result: None,
                },
            ],
            conflict_resolution: "precedence".to_string(),
        };

        let registry = DecisionTypeRegistry::from_config(&config);

        let allow_id = registry.get_id("allow").unwrap();
        let alert_id = registry.get_id("alert").unwrap();

        assert!(registry.can_combine(allow_id, alert_id));
    }

    #[test]
    fn test_registry_can_combine_exclusive() {
        let config = DecisionConfig {
            decision_types: vec![
                DecisionTypeConfig {
                    name: "allow".to_string(),
                    precedence: 100,
                    combinable: true,
                    exclusive: false,
                },
                DecisionTypeConfig {
                    name: "deny".to_string(),
                    precedence: 200,
                    combinable: false,
                    exclusive: true,
                },
            ],
            combination_rules: vec![],
            conflict_resolution: "precedence".to_string(),
        };

        let registry = DecisionTypeRegistry::from_config(&config);

        let allow_id = registry.get_id("allow").unwrap();
        let deny_id = registry.get_id("deny").unwrap();

        // Deny is exclusive, cannot combine
        assert!(!registry.can_combine(allow_id, deny_id));
    }

    #[test]
    fn test_registry_can_combine_explicit_exclusive_rule() {
        let config = DecisionConfig {
            decision_types: vec![
                DecisionTypeConfig {
                    name: "allow".to_string(),
                    precedence: 100,
                    combinable: true,
                    exclusive: false,
                },
                DecisionTypeConfig {
                    name: "validate".to_string(),
                    precedence: 60,
                    combinable: true,
                    exclusive: false,
                },
            ],
            combination_rules: vec![
                CombinationRule {
                    when: vec!["allow".to_string(), "validate".to_string()],
                    then: CombinationStrategy::Exclusive,
                    result: Some(vec!["allow".to_string()]),
                },
            ],
            conflict_resolution: "precedence".to_string(),
        };

        let registry = DecisionTypeRegistry::from_config(&config);

        let allow_id = registry.get_id("allow").unwrap();
        let validate_id = registry.get_id("validate").unwrap();

        // Explicit exclusive rule, cannot combine
        assert!(!registry.can_combine(allow_id, validate_id));
    }
}
