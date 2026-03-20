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

//! Integration example demonstrating multi-valued authorization decisions
//!
//! This example shows how to use Cedar's extended authorization API to get
//! multiple concurrent decision types from a single authorization request.
//!
//! Use case: Security monitoring where access is allowed but alerts are
//! triggered for sensitive resources.

use cedar_policy_core::{
    ast::{Context, EntityUID, PolicySet, Request},
    authorizer::Authorizer,
    entities::{decision_registry::DecisionTypeId, Entities},
    extensions::Extensions,
    parser,
};

fn main() {
    println!("Cedar Multi-Valued Authorization Example\n");
    println!("==========================================\n");

    // Create the authorizer
    let authorizer = Authorizer::new();

    // Define policies
    let policy1_src = r#"
        // Allow department members to read resources in their department
        permit(
            principal,
            action == Action::"read",
            resource
        ) when {
            principal.department == resource.department
        };
    "#;

    let policy2_src = r#"
        // Forbid access to archived resources
        forbid(
            principal,
            action,
            resource
        ) when {
            resource.archived == true
        };
    "#;

    // Parse and add policies to policy set
    let mut policy_set = PolicySet::new();
    policy_set
        .add_static(
            parser::parse_policy(
                Some(cedar_policy_core::ast::PolicyID::from_string("allow_dept")),
                policy1_src,
            )
            .expect("Failed to parse policy1"),
        )
        .expect("Failed to add policy1");

    policy_set
        .add_static(
            parser::parse_policy(
                Some(cedar_policy_core::ast::PolicyID::from_string("forbid_archived")),
                policy2_src,
            )
            .expect("Failed to parse policy2"),
        )
        .expect("Failed to add policy2");

    // Create entities
    let entities = Entities::new();

    println!("Scenario 1: Department member accessing own department's resource");
    println!("----------------------------------------------------------------");

    // Create authorization request
    let request1 = Request::new(
        (
            EntityUID::with_eid_and_type("User", "alice")
                .expect("Failed to create principal"),
            None,
        ),
        (
            EntityUID::with_eid_and_type("Action", "read").expect("Failed to create action"),
            None,
        ),
        (
            EntityUID::with_eid_and_type("Resource", "doc1")
                .expect("Failed to create resource"),
            None,
        ),
        Context::empty(),
        None::<&cedar_policy_core::entities::json::NoEntitiesSchema>,
        Extensions::none(),
    )
    .expect("Failed to create request");

    // Use the extended multi-valued API
    let multi_response = authorizer.decisions(request1.clone(), &policy_set, &entities);

    println!("Multi-valued response:");
    println!("  Decision types present: {}", multi_response.decision_types().count());

    if multi_response.has_decision(DecisionTypeId::ALLOW) {
        println!("  ✓ ALLOW decision present");
        if let Some(policies) = multi_response.policies_for(DecisionTypeId::ALLOW) {
            println!("    Policies: {:?}", policies);
        }
    }

    if multi_response.has_decision(DecisionTypeId::DENY) {
        println!("  ✗ DENY decision present");
        if let Some(policies) = multi_response.policies_for(DecisionTypeId::DENY) {
            println!("    Policies: {:?}", policies);
        }
    }

    // Convert to legacy binary response for backward compatibility
    let legacy_response = multi_response.into_legacy();
    println!("\nLegacy binary response:");
    println!("  Decision: {:?}", legacy_response.decision);
    println!("  Reason policies: {:?}", legacy_response.diagnostics.reason);

    println!("\n\nScenario 2: Accessing archived resource (deny should win)");
    println!("----------------------------------------------------------");

    let request2 = Request::new(
        (
            EntityUID::with_eid_and_type("User", "bob").expect("Failed to create principal"),
            None,
        ),
        (
            EntityUID::with_eid_and_type("Action", "read").expect("Failed to create action"),
            None,
        ),
        (
            EntityUID::with_eid_and_type("Resource", "archived_doc")
                .expect("Failed to create resource"),
            None,
        ),
        Context::empty(),
        None::<&cedar_policy_core::entities::json::NoEntitiesSchema>,
        Extensions::none(),
    )
    .expect("Failed to create request");

    let multi_response2 = authorizer.decisions(request2, &policy_set, &entities);

    println!("Multi-valued response:");
    println!("  Decision types present: {}", multi_response2.decision_types().count());

    for decision_id in multi_response2.decision_types() {
        if decision_id == DecisionTypeId::ALLOW {
            println!("  ✓ ALLOW decision present");
        } else if decision_id == DecisionTypeId::DENY {
            println!("  ✗ DENY decision present (takes precedence)");
        }
    }

    let legacy_response2 = multi_response2.into_legacy();
    println!("\nLegacy binary response:");
    println!("  Decision: {:?} (deny wins by precedence)", legacy_response2.decision);

    println!("\n\nScenario 3: Conditional Validation (US2 - Future)");
    println!("--------------------------------------------------");
    println!("Use case: Require additional verification for high-value transactions");
    println!();
    println!("When parser support for effect(name) syntax is complete, policies like:");
    println!("  permit(principal, action == Action::\"transfer\", resource)");
    println!("  when {{ principal.role == \"finance_staff\" }};");
    println!();
    println!("  effect(validate)(principal, action == Action::\"transfer\", resource)");
    println!("  when {{ resource.amount > 10000 }};");
    println!();
    println!("Would return: {{ allow, validate }}");
    println!("Meaning: Allow the transfer, but require additional validation (e.g., 2FA)");
    println!();
    println!("Benefits:");
    println!("  - Risk-based authorization without blocking legitimate requests");
    println!("  - Seamless escalation to additional verification when needed");
    println!("  - Policy-driven rather than hardcoded in application logic");

    println!("\n\nScenario 4: Independent Audit Trail (US3 - Future)");
    println!("---------------------------------------------------");
    println!("Use case: Log all access to PII resources regardless of allow/deny");
    println!();
    println!("When parser support is complete, policies like:");
    println!("  effect(audit)(principal, action, resource)");
    println!("  when {{ resource.contains_pii == true }};");
    println!();
    println!("Would return audit decision alongside allow/deny decisions.");
    println!("For example:");
    println!("  - Access allowed + audit triggered: {{ allow, audit }}");
    println!("  - Access denied + audit triggered: {{ deny, audit }}");
    println!();
    println!("Benefits:");
    println!("  - Complete audit trail for compliance");
    println!("  - Audit decisions fire independently of authorization outcome");
    println!("  - Centralized audit policy management");

    println!("\n\nScenario 5: Security Monitoring with Alerts (US1 - Future)");
    println!("------------------------------------------------------------");
    println!("Use case: Grant access but trigger security alerts for sensitive resources");
    println!();
    println!("When parser support is complete, policies like:");
    println!("  permit(principal, action == Action::\"read\", resource)");
    println!("  when {{ principal.department == resource.department }};");
    println!();
    println!("  effect(alert)(principal, action == Action::\"read\", resource)");
    println!("  when {{ resource.classification == \"sensitive\" }};");
    println!();
    println!("Would return: {{ allow, alert }}");
    println!("Meaning: Grant access, but alert security team about sensitive resource access");
    println!();
    println!("Benefits:");
    println!("  - Real-time security monitoring without blocking users");
    println!("  - Anomaly detection through policy-driven alerts");
    println!("  - Balances security with usability");

    println!("\n\nKey Takeaways:");
    println!("--------------");
    println!("1. decisions() returns all decision types from matching policies");
    println!("2. Multiple decisions can be present concurrently (e.g., allow + alert)");
    println!("3. into_legacy() converts to binary Allow/Deny with precedence rules");
    println!("4. Existing is_authorized() API delegates to decisions()");
    println!("5. Full backward compatibility maintained for legacy Cedar policies");
    println!();
    println!("Future Enhancements (Parser Support Required):");
    println!("  - effect(alert) syntax for security monitoring");
    println!("  - effect(validate) syntax for conditional verification");
    println!("  - effect(audit) syntax for compliance logging");
    println!("  - Custom decision types via YAML configuration");
}
