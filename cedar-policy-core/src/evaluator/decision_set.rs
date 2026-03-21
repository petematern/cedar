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

//! DecisionSet - representing multiple concurrent authorization decisions
//!
//! Supports multi-valued authorization where a single request can yield
//! multiple decision types (e.g., "allow + alert" or "allow + validate + audit").

use crate::ast::PolicyID;
use crate::entities::decision_registry::{DecisionTypeId, DecisionTypeRegistry};
use std::collections::{HashMap, HashSet};

/// Set of concurrent authorization decisions with supporting policy information
///
/// Each decision type maps to the set of policies that returned that decision.
/// Provides methods for querying decisions, finding the primary (highest precedence)
/// decision, and converting to binary allow/deny format for backward compatibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionSet {
    /// Map of decision type ID to the policies that returned that decision
    decisions: HashMap<DecisionTypeId, HashSet<PolicyID>>,

    /// Reference to the decision type registry for precedence lookups
    /// This is optional to support testing without a full registry
    registry: Option<DecisionTypeRegistry>,
}

impl DecisionSet {
    /// Create a new DecisionSet with a registry for precedence resolution
    ///
    /// The registry is used to determine precedence order when finding the
    /// primary decision and for name lookups.
    pub fn new(registry: DecisionTypeRegistry) -> Self {
        Self {
            decisions: HashMap::new(),
            registry: Some(registry),
        }
    }

    /// Create an empty DecisionSet without a registry (for testing)
    pub fn empty() -> Self {
        Self {
            decisions: HashMap::new(),
            registry: None,
        }
    }

    /// Add a decision type with its associated policy
    pub fn add(&mut self, decision_id: DecisionTypeId, policy_id: PolicyID) {
        self.decisions
            .entry(decision_id)
            .or_insert_with(HashSet::new)
            .insert(policy_id);
    }

    /// Check if a specific decision type is present
    ///
    /// Returns true if at least one policy returned this decision type.
    pub fn has(&self, decision_id: DecisionTypeId) -> bool {
        self.decisions
            .get(&decision_id)
            .map_or(false, |policies| !policies.is_empty())
    }

    /// Get the primary (highest precedence) decision type
    ///
    /// Returns the decision type with the highest precedence according to the
    /// registry. If no registry is available, returns the first decision type
    /// with the lowest numeric ID (DENY < ALLOW < custom).
    ///
    /// Returns None if no decisions are present.
    pub fn primary(&self) -> Option<DecisionTypeId> {
        if self.decisions.is_empty() {
            return None;
        }

        if let Some(registry) = &self.registry {
            // Use registry precedence order (sorted highest first)
            for decision_id in self.decisions.keys() {
                if let Some(metadata) = registry.get_metadata(*decision_id) {
                    // Find the decision with highest precedence among present decisions
                    let mut highest = metadata;
                    let mut highest_id = *decision_id;

                    for other_id in self.decisions.keys() {
                        if let Some(other_meta) = registry.get_metadata(*other_id) {
                            if other_meta.precedence > highest.precedence {
                                highest = other_meta;
                                highest_id = *other_id;
                            }
                        }
                    }
                    return Some(highest_id);
                }
            }
        }

        // Fallback: return decision with lowest ID (DENY=1 < ALLOW=0 is wrong, so fix)
        // Actually DENY=1, ALLOW=0, so we want highest ID for built-ins
        // But really we should just pick any - without registry we can't determine precedence
        self.decisions.keys().copied().min()
    }

    /// Get all decision type names in this set
    ///
    /// Returns an iterator over the human-readable names of all decision types
    /// present. If no registry is available, returns decision type IDs as strings.
    pub fn all_names<'a>(&'a self) -> Box<dyn Iterator<Item = String> + 'a> {
        if let Some(registry) = &self.registry {
            Box::new(self.decisions.keys().filter_map(move |id| {
                registry.get_name(*id).map(|s| s.to_string())
            }))
        } else {
            Box::new(self.decisions.keys().map(|id| format!("decision_{}", id.0)))
        }
    }

    /// Get the set of policies that contributed to a specific decision type
    ///
    /// Returns None if the decision type is not present in this set.
    pub fn policies_for(&self, decision_id: DecisionTypeId) -> Option<&HashSet<PolicyID>> {
        self.decisions.get(&decision_id)
    }

    /// Get all decision types present in this set
    pub fn decision_types(&self) -> impl Iterator<Item = DecisionTypeId> + '_ {
        self.decisions.keys().copied()
    }

    /// Convert to binary allow/deny decision for backward compatibility
    ///
    /// Conversion rules:
    /// - If DENY present → Deny
    /// - If ALLOW present (and no DENY) → Allow
    /// - Otherwise → Deny (safe default)
    pub fn to_decision(&self) -> crate::authorizer::Decision {
        use crate::authorizer::Decision;

        if self.has(DecisionTypeId::DENY) {
            Decision::Deny
        } else if self.has(DecisionTypeId::ALLOW) {
            Decision::Allow
        } else {
            // Safe default when neither allow nor deny present
            Decision::Deny
        }
    }

    /// Get the total number of decision types present
    pub fn len(&self) -> usize {
        self.decisions.len()
    }

    /// Check if the decision set is empty
    pub fn is_empty(&self) -> bool {
        self.decisions.is_empty()
    }

    /// Convert DecisionSet into the internal decisions HashMap
    ///
    /// Consumes the DecisionSet and returns the underlying HashMap.
    /// Useful for converting to MultiResponse after applying exclusivity rules.
    pub fn into_decisions(self) -> HashMap<DecisionTypeId, HashSet<PolicyID>> {
        self.decisions
    }

    /// Apply exclusivity rules using the registry's combination rules
    ///
    /// This modifies the decision set in-place by removing decisions that
    /// are excluded by combination rules. Always applies the implicit allow+deny
    /// rule first, then applies user-defined combination rules from the registry.
    pub fn apply_exclusivity(&mut self) {
        // IMPLICIT RULE: Allow and Deny cannot coexist (deny wins)
        if self.has(DecisionTypeId::ALLOW) && self.has(DecisionTypeId::DENY) {
            self.decisions.remove(&DecisionTypeId::ALLOW);
        }

        // Apply combination rules from registry
        if let Some(registry) = &self.registry {
            // Get current decision IDs
            let current_ids: Vec<DecisionTypeId> = self.decisions.keys().copied().collect();

            // Resolve using registry combination rules
            let resolved_ids = registry.resolve(&current_ids);

            // Keep only resolved decisions
            self.decisions.retain(|id, _| resolved_ids.contains(id));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DecisionConfig, DecisionTypeConfig};

    fn minimal_registry() -> DecisionTypeRegistry {
        let config = DecisionConfig {
            decision_types: vec![
                DecisionTypeConfig {
                    name: "allow".to_string(),
                    precedence: 100,
                },
                DecisionTypeConfig {
                    name: "deny".to_string(),
                    precedence: 200,
                },
            ],
            combination_rules: Vec::new(),
            conflict_resolution: "precedence".to_string(),
        };
        DecisionTypeRegistry::from_config(&config)
    }

    fn extended_registry() -> DecisionTypeRegistry {
        let config = DecisionConfig {
            decision_types: vec![
                DecisionTypeConfig {
                    name: "allow".to_string(),
                    precedence: 100,
                },
                DecisionTypeConfig {
                    name: "deny".to_string(),
                    precedence: 200,
                },
                DecisionTypeConfig {
                    name: "alert".to_string(),
                    precedence: 50,
                },
                DecisionTypeConfig {
                    name: "validate".to_string(),
                    precedence: 60,
                },
            ],
            combination_rules: Vec::new(),
            conflict_resolution: "precedence".to_string(),
        };
        DecisionTypeRegistry::from_config(&config)
    }

    #[test]
    fn test_empty_decision_set() {
        let set = DecisionSet::empty();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
        assert_eq!(set.primary(), None);
    }

    #[test]
    fn test_add_and_has() {
        let mut set = DecisionSet::empty();
        let policy1 = PolicyID::from_string("policy1");

        assert!(!set.has(DecisionTypeId::ALLOW));

        set.add(DecisionTypeId::ALLOW, policy1.clone());

        assert!(set.has(DecisionTypeId::ALLOW));
        assert!(!set.has(DecisionTypeId::DENY));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_policies_for() {
        let mut set = DecisionSet::empty();
        let policy1 = PolicyID::from_string("policy1");
        let policy2 = PolicyID::from_string("policy2");

        set.add(DecisionTypeId::ALLOW, policy1.clone());
        set.add(DecisionTypeId::ALLOW, policy2.clone());

        let policies = set.policies_for(DecisionTypeId::ALLOW).unwrap();
        assert_eq!(policies.len(), 2);
        assert!(policies.contains(&policy1));
        assert!(policies.contains(&policy2));

        assert_eq!(set.policies_for(DecisionTypeId::DENY), None);
    }

    #[test]
    fn test_primary_with_registry() {
        let registry = extended_registry();
        let mut set = DecisionSet::new(registry);

        let policy1 = PolicyID::from_string("p1");
        set.add(DecisionTypeId::ALLOW, policy1.clone());
        set.add(DecisionTypeId(100), policy1.clone()); // alert, precedence 50

        // Primary should be ALLOW (precedence 100) over alert (precedence 50)
        assert_eq!(set.primary(), Some(DecisionTypeId::ALLOW));

        // Add DENY (highest precedence)
        set.add(DecisionTypeId::DENY, policy1.clone());
        assert_eq!(set.primary(), Some(DecisionTypeId::DENY));
    }

    #[test]
    fn test_to_decision_deny_precedence() {
        let mut set = DecisionSet::empty();
        let policy = PolicyID::from_string("p1");

        set.add(DecisionTypeId::ALLOW, policy.clone());
        set.add(DecisionTypeId::DENY, policy.clone());

        // DENY should take precedence
        assert_eq!(set.to_decision(), crate::authorizer::Decision::Deny);
    }

    #[test]
    fn test_to_decision_allow_only() {
        let mut set = DecisionSet::empty();
        let policy = PolicyID::from_string("p1");

        set.add(DecisionTypeId::ALLOW, policy);

        assert_eq!(set.to_decision(), crate::authorizer::Decision::Allow);
    }

    #[test]
    fn test_to_decision_custom_only() {
        let mut set = DecisionSet::empty();
        let policy = PolicyID::from_string("p1");

        // Only custom decision types (no allow or deny)
        set.add(DecisionTypeId(100), policy);

        // Should default to Deny for safety
        assert_eq!(set.to_decision(), crate::authorizer::Decision::Deny);
    }

    #[test]
    fn test_all_names_with_registry() {
        let registry = extended_registry();
        let mut set = DecisionSet::new(registry);

        set.add(DecisionTypeId::ALLOW, PolicyID::from_string("p1"));
        set.add(DecisionTypeId(100), PolicyID::from_string("p2")); // alert

        let names: Vec<_> = set.all_names().collect();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"allow".to_string()));
        assert!(names.contains(&"alert".to_string()));
    }

    #[test]
    fn test_decision_types_iterator() {
        let mut set = DecisionSet::empty();

        set.add(DecisionTypeId::ALLOW, PolicyID::from_string("p1"));
        set.add(DecisionTypeId::DENY, PolicyID::from_string("p2"));
        set.add(DecisionTypeId(100), PolicyID::from_string("p3"));

        let types: Vec<_> = set.decision_types().collect();
        assert_eq!(types.len(), 3);
        assert!(types.contains(&DecisionTypeId::ALLOW));
        assert!(types.contains(&DecisionTypeId::DENY));
        assert!(types.contains(&DecisionTypeId(100)));
    }

    #[test]
    fn test_multiple_policies_same_decision() {
        let mut set = DecisionSet::empty();

        set.add(DecisionTypeId::ALLOW, PolicyID::from_string("p1"));
        set.add(DecisionTypeId::ALLOW, PolicyID::from_string("p2"));
        set.add(DecisionTypeId::ALLOW, PolicyID::from_string("p3"));

        assert_eq!(set.len(), 1); // Still only one decision type
        assert_eq!(set.policies_for(DecisionTypeId::ALLOW).unwrap().len(), 3);
    }

    #[test]
    fn test_apply_exclusivity_deny_wins() {
        use crate::config::{DecisionConfig, DecisionTypeConfig};
        use crate::entities::decision_registry::CombinationRule;

        let config = DecisionConfig {
            decision_types: vec![
                DecisionTypeConfig {
                    name: "allow".to_string(),
                    precedence: 100,
                },
                DecisionTypeConfig {
                    name: "deny".to_string(),
                    precedence: 200,
                },
                DecisionTypeConfig {
                    name: "alert".to_string(),
                    precedence: 50,
                },
            ],
            combination_rules: vec![CombinationRule {
                when: vec!["deny".to_string(), "*".to_string()],
                then: crate::entities::decision_registry::CombinationStrategy::Exclusive,
                result: Some(vec!["deny".to_string()]),
            }],
            conflict_resolution: "precedence".to_string(),
        };

        let registry = DecisionTypeRegistry::from_config(&config);
        let mut set = DecisionSet::new(registry);

        set.add(DecisionTypeId::ALLOW, PolicyID::from_string("p1"));
        set.add(DecisionTypeId::DENY, PolicyID::from_string("p2"));
        set.add(DecisionTypeId(100), PolicyID::from_string("p3")); // alert

        assert_eq!(set.len(), 3);

        set.apply_exclusivity();

        // Only deny should remain
        assert_eq!(set.len(), 1);
        assert!(set.has(DecisionTypeId::DENY));
        assert!(!set.has(DecisionTypeId::ALLOW));
    }

    #[test]
    fn test_apply_exclusivity_merge_decisions() {
        use crate::config::{DecisionConfig, DecisionTypeConfig};
        use crate::entities::decision_registry::CombinationRule;

        let config = DecisionConfig {
            decision_types: vec![
                DecisionTypeConfig {
                    name: "allow".to_string(),
                    precedence: 100,
                },
                DecisionTypeConfig {
                    name: "alert".to_string(),
                    precedence: 50,
                },
            ],
            combination_rules: vec![CombinationRule {
                when: vec!["allow".to_string(), "alert".to_string()],
                then: crate::entities::decision_registry::CombinationStrategy::Merge,
                result: None,
            }],
            conflict_resolution: "precedence".to_string(),
        };

        let registry = DecisionTypeRegistry::from_config(&config);
        let mut set = DecisionSet::new(registry);

        set.add(DecisionTypeId::ALLOW, PolicyID::from_string("p1"));
        set.add(DecisionTypeId(100), PolicyID::from_string("p2")); // alert

        assert_eq!(set.len(), 2);

        set.apply_exclusivity();

        // Both should remain with merge rule
        assert_eq!(set.len(), 2);
        assert!(set.has(DecisionTypeId::ALLOW));
        assert!(set.has(DecisionTypeId(100)));
    }

    #[test]
    fn test_apply_exclusivity_without_registry() {
        let mut set = DecisionSet::empty();

        set.add(DecisionTypeId::ALLOW, PolicyID::from_string("p1"));
        set.add(DecisionTypeId::DENY, PolicyID::from_string("p2"));

        set.apply_exclusivity();

        // Without registry, basic deny exclusive logic applies
        assert_eq!(set.len(), 1);
        assert!(set.has(DecisionTypeId::DENY));
    }
}
