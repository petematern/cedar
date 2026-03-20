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

//! Tests for custom decision type parsing

use crate::ast::Effect;
use crate::config::DecisionConfig;
use crate::entities::decision_registry::DecisionTypeRegistry;
use crate::parser;

#[test]
fn test_custom_effect_alert() {
    let config_yaml = r#"
decision_types:
  - name: allow
    precedence: 100
    combinable: true
    exclusive: false
  - name: deny
    precedence: 200
    combinable: false
    exclusive: true
  - name: alert
    precedence: 50
    combinable: true
    exclusive: false
"#;

    let config = DecisionConfig::from_str(config_yaml).expect("config should parse");
    let registry = DecisionTypeRegistry::from_config(&config);

    let policy_text = r#"
        alert(principal, action, resource)
        when { resource.classification == "sensitive" };
    "#;

    let policy_set = parser::parse_policyset_with_registry(policy_text, &registry)
        .expect("policy should parse");

    assert_eq!(policy_set.policies().count(), 1);
    let policy = policy_set.policies().next().expect("should have one policy");

    match policy.effect() {
        Effect::Custom(id) => {
            assert_eq!(
                registry.get_name(id),
                Some("alert"),
                "effect should be alert"
            );
        }
        _ => panic!("effect should be Custom, not Permit or Forbid"),
    }
}

#[test]
fn test_custom_effect_validate() {
    let config_yaml = r#"
decision_types:
  - name: allow
    precedence: 100
    combinable: true
    exclusive: false
  - name: deny
    precedence: 200
    combinable: false
    exclusive: true
  - name: validate
    precedence: 60
    combinable: true
    exclusive: false
"#;

    let config = DecisionConfig::from_str(config_yaml).expect("config should parse");
    let registry = DecisionTypeRegistry::from_config(&config);

    let policy_text = r#"
        validate(principal, action, resource)
        when { resource.amount > 10000 };
    "#;

    let policy_set = parser::parse_policyset_with_registry(policy_text, &registry)
        .expect("policy should parse");

    assert_eq!(policy_set.policies().count(), 1);
    let policy = policy_set.policies().next().expect("should have one policy");

    match policy.effect() {
        Effect::Custom(id) => {
            assert_eq!(
                registry.get_name(id),
                Some("validate"),
                "effect should be validate"
            );
        }
        _ => panic!("effect should be Custom"),
    }
}

#[test]
fn test_custom_effect_audit() {
    let config_yaml = r#"
decision_types:
  - name: allow
    precedence: 100
    combinable: true
    exclusive: false
  - name: deny
    precedence: 200
    combinable: false
    exclusive: true
  - name: audit
    precedence: 40
    combinable: true
    exclusive: false
"#;

    let config = DecisionConfig::from_str(config_yaml).expect("config should parse");
    let registry = DecisionTypeRegistry::from_config(&config);

    let policy_text = r#"
        audit(principal, action, resource)
        when { resource.contains_pii == true };
    "#;

    let policy_set = parser::parse_policyset_with_registry(policy_text, &registry)
        .expect("policy should parse");

    assert_eq!(policy_set.policies().count(), 1);
    let policy = policy_set.policies().next().expect("should have one policy");

    match policy.effect() {
        Effect::Custom(id) => {
            assert_eq!(
                registry.get_name(id),
                Some("audit"),
                "effect should be audit"
            );
        }
        _ => panic!("effect should be Custom"),
    }
}

#[test]
fn test_multiple_custom_effects() {
    let config_yaml = r#"
decision_types:
  - name: allow
    precedence: 100
    combinable: true
    exclusive: false
  - name: deny
    precedence: 200
    combinable: false
    exclusive: true
  - name: alert
    precedence: 50
    combinable: true
    exclusive: false
  - name: validate
    precedence: 60
    combinable: true
    exclusive: false
  - name: audit
    precedence: 40
    combinable: true
    exclusive: false
"#;

    let config = DecisionConfig::from_str(config_yaml).expect("config should parse");
    let registry = DecisionTypeRegistry::from_config(&config);

    let policy_text = r#"
        permit(principal, action, resource)
        when { principal.role == "admin" };

        alert(principal, action, resource)
        when { resource.classification == "sensitive" };

        validate(principal, action, resource)
        when { resource.amount > 10000 };

        audit(principal, action, resource)
        when { resource.contains_pii == true };
    "#;

    let policy_set = parser::parse_policyset_with_registry(policy_text, &registry)
        .expect("policies should parse");

    assert_eq!(policy_set.policies().count(), 4);

    let policies: Vec<_> = policy_set.policies().collect();

    // Check first policy is permit (Effect::Permit)
    assert!(matches!(policies[0].effect(), Effect::Permit));

    // Check remaining are custom effects
    let effects: Vec<_> = policies[1..]
        .iter()
        .map(|p| match p.effect() {
            Effect::Custom(id) => registry
                .get_name(id)
                .expect("should have name")
                .to_string(),
            _ => panic!("expected custom effect"),
        })
        .collect();

    assert_eq!(effects, vec!["alert", "validate", "audit"]);
}

#[test]
fn test_unknown_custom_effect_fails() {
    let config_yaml = r#"
decision_types:
  - name: allow
    precedence: 100
    combinable: true
    exclusive: false
  - name: deny
    precedence: 200
    combinable: false
    exclusive: true
"#;

    let config = DecisionConfig::from_str(config_yaml).expect("config should parse");
    let registry = DecisionTypeRegistry::from_config(&config);

    let policy_text = r#"
        unknown(principal, action, resource)
        when { true };
    "#;

    let result = parser::parse_policyset_with_registry(policy_text, &registry);
    assert!(result.is_err(), "unknown effect should fail to parse");
}

#[test]
fn test_legacy_permit_forbid_still_work() {
    let config_yaml = r#"
decision_types:
  - name: allow
    precedence: 100
    combinable: true
    exclusive: false
  - name: deny
    precedence: 200
    combinable: false
    exclusive: true
"#;

    let config = DecisionConfig::from_str(config_yaml).expect("config should parse");
    let registry = DecisionTypeRegistry::from_config(&config);

    let policy_text = r#"
        permit(principal, action, resource);
        forbid(principal, action, resource) when { false };
    "#;

    let policy_set = parser::parse_policyset_with_registry(policy_text, &registry)
        .expect("legacy policies should parse");

    assert_eq!(policy_set.policies().count(), 2);

    let policies: Vec<_> = policy_set.policies().collect();
    assert!(matches!(policies[0].effect(), Effect::Permit));
    assert!(matches!(policies[1].effect(), Effect::Forbid));
}

#[test]
fn test_without_registry_custom_effects_fail() {
    let policy_text = r#"
        alert(principal, action, resource)
        when { resource.classification == "sensitive" };
    "#;

    // Without registry, custom effects should fail
    let result = parser::parse_policyset(policy_text);
    assert!(
        result.is_err(),
        "custom effect should fail without registry"
    );
}

#[test]
fn test_without_registry_permit_forbid_work() {
    let policy_text = r#"
        permit(principal, action, resource);
        forbid(principal, action, resource) when { false };
    "#;

    // Without registry, permit/forbid should still work
    let policy_set = parser::parse_policyset(policy_text).expect("permit/forbid should parse");

    assert_eq!(policy_set.policies().count(), 2);
}
