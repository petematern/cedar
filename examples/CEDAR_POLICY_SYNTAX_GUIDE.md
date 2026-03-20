# Writing Cedar Policies with Custom Decision Types

This guide explains how to write Cedar policy files (`.cedar` files) that use custom decision types beyond the standard `permit` and `forbid`.

## Table of Contents

1. [Introduction](#introduction)
2. [Standard Policy Syntax](#standard-policy-syntax)
3. [Custom Decision Type Syntax](#custom-decision-type-syntax)
4. [Complete Examples](#complete-examples)
5. [Parsing Policies with Custom Decisions](#parsing-policies-with-custom-decisions)
6. [Best Practices](#best-practices)

## Introduction

Cedar's multi-valued decision system allows you to define custom decision types like `alert`, `validate`, and `audit` in addition to the built-in `permit` and `forbid`. Once configured, you can write policies using these custom decision types directly in your Cedar policy files.

## Standard Policy Syntax

Standard Cedar policies use `permit` and `forbid` as effects:

```cedar
// Grant access to admins
permit(principal, action, resource)
when { principal.role == "admin" };

// Deny access to archived resources
forbid(principal, action, resource)
when { resource.archived == true };
```

## Custom Decision Type Syntax

With custom decision types configured, you can use them just like `permit` and `forbid`:

```cedar
// Trigger an alert for sensitive resources
alert(principal, action, resource)
when { resource.classification == "sensitive" };

// Require validation for high-value transactions
validate(principal, action, resource)
when { resource.amount > 10000 };

// Log audit trail for PII access
audit(principal, action, resource)
when { resource.contains_pii == true };
```

The syntax is identical to standard policies:
- **Decision type name** (e.g., `alert`, `validate`, `audit`)
- **Scope** in parentheses: `(principal, action, resource)`
- **Conditions** using `when` and/or `unless` clauses

## Complete Examples

### Example 1: Security Monitoring

**Configuration** (`decision_config.yaml`):
```yaml
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

combination_rules:
  - when: [deny, "*"]
    then: exclusive
    result: [deny]
  - when: [allow, alert]
    then: merge
```

**Policy File** (`security_monitoring.cedar`):
```cedar
// Grant department access
permit(principal, action == Action::"read", resource)
when { principal.department == resource.department };

// Alert on sensitive resource access
alert(principal, action == Action::"read", resource)
when { resource.classification == "sensitive" };

// Deny external contractors
forbid(principal, action, resource)
when { principal.employment_type == "contractor" &&
       principal.location == "external" };
```

**Result**: When a department member accesses a sensitive resource, the system returns both `allow` and `alert` decisions, enabling concurrent authorization and monitoring.

### Example 2: Financial Validation

**Configuration** (`decision_config.yaml`):
```yaml
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

combination_rules:
  - when: [deny, "*"]
    then: exclusive
    result: [deny]
  - when: [allow, validate]
    then: merge
```

**Policy File** (`financial_validation.cedar`):
```cedar
// Allow finance staff to transfer funds
permit(principal, action == Action::"transfer", resource)
when { principal.role == "finance_staff" };

// Require 2FA for large transfers
validate(principal, action == Action::"transfer", resource)
when { resource.amount > 10000 };

// Require approval for international transfers
validate(principal, action == Action::"transfer", resource)
when { resource.destination_country != "US" };

// Deny transfers to sanctioned countries
forbid(principal, action == Action::"transfer", resource)
when { resource.destination_country in ["XX", "YY"] };
```

**Result**: Finance staff can transfer funds, but high-value or international transfers trigger the `validate` decision requiring additional verification.

### Example 3: Compliance Audit Trail

**Configuration** (`decision_config.yaml`):
```yaml
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

combination_rules:
  - when: [deny, "*"]
    then: exclusive
    result: [deny]
  - when: [audit, "*"]
    then: merge
```

**Policy File** (`compliance_audit.cedar`):
```cedar
// Standard access control
permit(principal, action, resource)
when { principal.clearance_level >= resource.required_clearance };

// Audit all PII access
audit(principal, action, resource)
when { resource.contains_pii == true };

// Audit all admin actions
audit(principal, action, resource)
when { principal.role == "admin" };

// Audit healthcare record access
audit(principal, action, resource)
when { resource.type == "HealthcareRecord" };
```

**Result**: Access is controlled by clearance levels, but all PII access, admin actions, and healthcare records trigger `audit` decisions for compliance logging, regardless of whether access is allowed or denied.

### Example 4: Multiple Custom Decisions

**Configuration** (`decision_config.yaml`):
```yaml
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

combination_rules:
  - when: [deny, "*"]
    then: exclusive
    result: [deny]
  - when: [allow, alert]
    then: merge
  - when: [allow, validate]
    then: merge
  - when: [audit, "*"]
    then: merge
```

**Policy File** (`comprehensive.cedar`):
```cedar
// Grant access to employees
permit(principal, action == Action::"access", resource)
when { principal.employee_id != null };

// Alert on after-hours access
alert(principal, action == Action::"access", resource)
when { context.time.hour < 6 || context.time.hour > 22 };

// Validate high-risk operations
validate(principal, action == Action::"delete", resource)
when { resource.criticality == "high" };

// Audit all admin operations
audit(principal, action, resource)
when { principal.role == "admin" };

// Audit all PII access
audit(principal, action, resource)
when { resource.contains_pii == true };

// Deny suspended users
forbid(principal, action, resource)
when { principal.account_status == "suspended" };
```

**Result**: A single authorization request can return multiple decisions:
- `allow` + `alert` (after-hours access)
- `allow` + `validate` (high-risk operation)
- `allow` + `alert` + `audit` (after-hours admin accessing PII)
- `deny` (suspended user)

## Parsing Policies with Custom Decisions

### Rust Code Example

```rust
use cedar_policy_core::{
    authorizer::Authorizer,
    config::DecisionConfig,
    entities::decision_registry::DecisionTypeRegistry,
    parser,
};

// 1. Load decision type configuration
let config = DecisionConfig::from_file("decision_config.yaml")?;
let registry = DecisionTypeRegistry::from_config(&config);

// 2. Parse policy file with custom decision support
let policy_text = std::fs::read_to_string("policies.cedar")?;
let policy_set = parser::parse_policyset_with_registry(&policy_text, &registry)?;

// 3. Perform multi-valued authorization
let authorizer = Authorizer::new();
let multi_response = authorizer.decisions(
    request,
    &policy_set,
    &entities,
);

// 4. Get decision type IDs for checking results
let allow_id = registry.get_id("allow").unwrap();
let alert_id = registry.get_id("alert").unwrap();
let validate_id = registry.get_id("validate").unwrap();
let audit_id = registry.get_id("audit").unwrap();

// 5. Handle multiple concurrent decisions
if multi_response.has_decision(allow_id) {
    // Grant access
    println!("Access granted");
}

if multi_response.has_decision(alert_id) {
    // Send security alert
    send_security_alert("Suspicious activity detected");
}

if multi_response.has_decision(validate_id) {
    // Require additional verification
    prompt_two_factor_auth();
}

if multi_response.has_decision(audit_id) {
    // Log to audit trail
    audit_log.record(request);
}
```

### Without Custom Decision Support

If you try to parse a policy with custom decision types without providing a registry, you'll get a parse error:

```rust
// This will FAIL if the policy uses custom effects like alert, validate, audit
let result = parser::parse_policyset(&policy_text);
assert!(result.is_err());
```

### Backward Compatibility

Standard `permit` and `forbid` policies work with or without a registry:

```rust
let standard_policies = r#"
    permit(principal, action, resource)
    when { principal.role == "admin" };

    forbid(principal, action, resource)
    when { resource.archived == true };
"#;

// Works without registry
let policy_set = parser::parse_policyset(standard_policies)?;

// Also works with registry
let policy_set = parser::parse_policyset_with_registry(standard_policies, &registry)?;
```

## Best Practices

### 1. Use Descriptive Names

Choose custom decision type names that clearly indicate their purpose:

✅ **Good**:
- `alert` - Triggers security monitoring
- `validate` - Requires additional verification
- `audit` - Logs to compliance trail
- `notify` - Sends user notification

❌ **Bad**:
- `type1`, `type2`, `type3` - Unclear purpose
- `x`, `y`, `z` - No semantic meaning

### 2. Document Decision Types

Add comments to your policy files explaining what each custom decision type does:

```cedar
// Alert decision: Triggers security monitoring system
// Used for: Sensitive data access, after-hours operations, privilege escalation
alert(principal, action, resource)
when { resource.classification == "sensitive" };

// Validate decision: Requires additional user verification (2FA, approval)
// Used for: High-value transactions, data deletion, configuration changes
validate(principal, action == Action::"delete", resource)
when { resource.value > 10000 };
```

### 3. Group Related Policies

Organize policies by decision type for maintainability:

```cedar
// ============================================================
// STANDARD ACCESS CONTROL (permit/forbid)
// ============================================================

permit(principal, action, resource)
when { principal.role == "admin" };

forbid(principal, action, resource)
when { resource.archived == true };

// ============================================================
// SECURITY ALERTS
// ============================================================

alert(principal, action, resource)
when { resource.classification == "top_secret" };

alert(principal, action, resource)
when { context.time.hour < 6 || context.time.hour > 22 };

// ============================================================
// COMPLIANCE AUDITING
// ============================================================

audit(principal, action, resource)
when { resource.contains_pii == true };

audit(principal, action, resource)
when { principal.role == "admin" };
```

### 4. Test Custom Decision Combinations

Verify that your combination rules produce expected results:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allow_plus_alert() {
        let result = evaluate_policies(admin_accessing_sensitive_resource);
        assert!(result.has_decision(allow_id));
        assert!(result.has_decision(alert_id));
    }

    #[test]
    fn test_deny_excludes_all() {
        let result = evaluate_policies(suspended_user_accessing_anything);
        assert!(result.has_decision(deny_id));
        assert!(!result.has_decision(allow_id));
        assert!(!result.has_decision(alert_id));
    }
}
```

### 5. Keep Policies Focused

Each policy should have a single, clear purpose:

✅ **Good**:
```cedar
// One condition per policy - clear and focused
alert(principal, action, resource)
when { resource.classification == "sensitive" };

alert(principal, action, resource)
when { context.time.hour < 6 || context.time.hour > 22 };
```

❌ **Bad**:
```cedar
// Too many unrelated conditions
alert(principal, action, resource)
when {
    resource.classification == "sensitive" ||
    context.time.hour < 6 ||
    principal.access_count > 100 ||
    resource.region == "restricted"
};
```

### 6. Version Your Configuration

Track changes to your decision type configuration:

```yaml
# decision_config.yaml
# Version: 2.1.0
# Last Updated: 2024-03-19
# Changes: Added 'notify' decision type for user notifications

decision_types:
  - name: allow
    precedence: 100
    combinable: true
    exclusive: false
  # ... rest of configuration
```

## Summary

Writing Cedar policies with custom decision types is straightforward:

1. **Configure** your decision types in `decision_config.yaml`
2. **Write** policies using custom decision names just like `permit`/`forbid`
3. **Parse** policies with `parse_policyset_with_registry()`
4. **Handle** multiple concurrent decisions in your application logic

Custom decision types enable powerful authorization patterns like security monitoring, conditional verification, and compliance auditing while maintaining Cedar's declarative policy syntax.
