# Cedar Multi-Valued Authorization Decision Guide

## Overview

Cedar's multi-valued authorization decision system extends the traditional binary permit/forbid model to support multiple concurrent decision types. This enables use cases like:

- **Security Monitoring**: Grant access while triggering alerts for sensitive resources
- **Conditional Verification**: Allow actions but require additional validation (e.g., 2FA)
- **Audit Trail**: Log access attempts independently of authorization outcome
- **Custom Workflows**: Define application-specific decision types via configuration

## Quick Start

### 1. Configuration

Create a `decision_config.yaml` file defining your decision types:

```yaml
decision_types:
  # Built-in types (required)
  - name: allow
    precedence: 100

  - name: deny
    precedence: 200

  # Custom types
  - name: alert
    precedence: 50

  - name: validate
    precedence: 60

  - name: audit
    precedence: 40

# Define how decision types interact
combination_rules:
  # Deny always wins
  - when: [deny, "*"]
    then: exclusive
    result: [deny]

  # Allow and alert can coexist
  - when: [allow, alert]
    then: merge

  # Audit can combine with anything
  - when: [audit, "*"]
    then: merge

conflict_resolution: precedence

# IMPLICIT RULE (always applied, cannot be overridden):
# Allow and Deny cannot coexist - if both are present, Allow is removed.
# When no other rules match, the default behavior is MERGE.
```

### 2. Load Configuration

```rust
use cedar_policy_core::{
    config::DecisionConfig,
    entities::decision_registry::DecisionTypeRegistry,
};

// Load configuration from file
let config = DecisionConfig::from_file("decision_config.yaml")
    .expect("Failed to load configuration");

// Create registry
let registry = DecisionTypeRegistry::from_config(&config);
```

### 3. Use Multi-Valued Authorization API

```rust
use cedar_policy_core::{
    authorizer::Authorizer,
    ast::{PolicySet, Request},
    entities::Entities,
};

let authorizer = Authorizer::new();
let policy_set = /* your policies */;
let entities = Entities::new();
let request = /* your authorization request */;

// Call extended API with registry
let multi_response = authorizer.decisions(
    request,
    &policy_set,
    &entities,
    &registry,
);

// Check for specific decision types
if multi_response.has_decision(allow_id) {
    // Grant access
}

if multi_response.has_decision(alert_id) {
    // Trigger security alert
}

if multi_response.has_decision(validate_id) {
    // Require additional verification
}

if multi_response.has_decision(audit_id) {
    // Log to audit trail
}

// Or convert to legacy binary response
let legacy_response = multi_response.into_legacy();
match legacy_response.decision {
    Decision::Allow => { /* allow */ }
    Decision::Deny => { /* deny */ }
}
```

## Decision Type Properties

Each decision type has two properties:

### Precedence

Higher precedence values take priority in conflict resolution:

```yaml
- name: deny
  precedence: 200  # Highest - always wins

- name: allow
  precedence: 100

- name: alert
  precedence: 50   # Lowest among these examples
```

### Name

The unique identifier for the decision type (lowercase alphanumeric + underscore).

## Default Behavior

**Implicit Rule (Always Applied)**:
- **Allow and Deny cannot coexist**: If both are present, Allow is automatically removed
- This rule is hardcoded and cannot be overridden

**When No Rules Match**:
- **Default strategy**: MERGE (all decisions coexist)
- This forces explicit configuration of exclusions via combination rules

## Combination Rules

### Merge Strategy

Both decisions remain in the result:

```yaml
combination_rules:
  - when: [allow, alert]
    then: merge
```

Result: `{ allow, alert }`

### Exclusive Strategy

Only specified result decisions remain:

```yaml
combination_rules:
  - when: [deny, "*"]
    then: exclusive
    result: [deny]
```

Result: `{ deny }` (all others removed)

### Wildcard Matching

Use `"*"` to match any decision type:

```yaml
combination_rules:
  - when: [audit, "*"]
    then: merge
```

This allows audit to combine with any other decision.

## Use Cases

### Security Monitoring (US1)

**Scenario**: Grant access to department resources while alerting on sensitive items

**Configuration**:
```yaml
decision_types:
  - name: allow
    precedence: 100

  - name: alert
    precedence: 50

combination_rules:
  - when: [allow, alert]
    then: merge
```

**Policies** (future - requires parser support):
```cedar
// Grant department access
permit(principal, action == Action::"read", resource)
when { principal.department == resource.department };

// Alert on sensitive resources
effect(alert)(principal, action == Action::"read", resource)
when { resource.classification == "sensitive" };
```

**Result**: `{ allow, alert }` - User can access, and security team is notified

### Conditional Verification (US2)

**Scenario**: Allow financial transfers but require 2FA for high-value amounts

**Configuration**:
```yaml
decision_types:
  - name: allow
    precedence: 100

  - name: validate
    precedence: 60

combination_rules:
  - when: [allow, validate]
    then: merge
```

**Policies** (future):
```cedar
// Allow finance staff to transfer
permit(principal, action == Action::"transfer", resource)
when { principal.role == "finance_staff" };

// Require validation for large amounts
effect(validate)(principal, action == Action::"transfer", resource)
when { resource.amount > 10000 };
```

**Result**: `{ allow, validate }` - Transfer permitted, but 2FA required

### Audit Trail (US3)

**Scenario**: Log all PII access regardless of allow/deny

**Configuration**:
```yaml
decision_types:
  - name: allow
    precedence: 100

  - name: deny
    precedence: 200

  - name: audit
    precedence: 40

combination_rules:
  - when: [deny, "*"]
    then: exclusive
    result: [deny]
  - when: [audit, "*"]
    then: merge
```

**Policies** (future):
```cedar
// Audit all PII access
effect(audit)(principal, action, resource)
when { resource.contains_pii == true };
```

**Result**:
- If access allowed: `{ allow, audit }`
- If access denied: `{ deny, audit }`

Audit fires regardless of authorization outcome.

## Backward Compatibility

The multi-valued system maintains 100% backward compatibility:

### Legacy API

```rust
// Existing code continues to work unchanged
let response = authorizer.is_authorized(request, &policy_set, &entities);

match response.decision {
    Decision::Allow => { /* allow */ }
    Decision::Deny => { /* deny */ }
}
```

### Legacy Policies

Existing `permit` and `forbid` policies work identically:

```cedar
permit(principal, action, resource)
when { principal.role == "admin" };

forbid(principal, action, resource)
when { resource.archived == true };
```

These automatically map to `allow` and `deny` decision types.

### Conversion Rules

When converting multi-valued responses to binary:

1. If `deny` present → `Decision::Deny`
2. If `allow` present (no deny) → `Decision::Allow`
3. Otherwise → `Decision::Deny` (safe default)

All other decision types (alert, validate, audit) are discarded in legacy conversion.

## Configuration Lifecycle

### Loading

Configuration is loaded at application startup:

```rust
let config = DecisionConfig::from_file("decision_config.yaml")?;
let registry = DecisionTypeRegistry::from_config(&config);
```

### Validation

Configuration is validated on load:

- **Required types**: `allow` and `deny` must be present
- **Name format**: Lowercase alphanumeric + underscore, 1-32 characters
- **No duplicates**: Decision type names must be unique

Any validation failure causes immediate startup failure (fail-fast).

### Updates

Configuration changes require application restart:

1. Update `decision_config.yaml`
2. Restart the application
3. New configuration takes effect

**No hot-reload** - ensures consistency and predictability.

## Performance

The multi-valued decision system is designed for production use:

### Targets

- **Binary authorization overhead**: < 5%
- **Multi-valued authorization overhead**: < 15%
- **Throughput**: > 10,000 requests/second

### Benchmarking

Run performance benchmarks:

```bash
cargo bench --bench multi_decision_bench
```

This measures:
- Binary vs multi-valued authorization
- Registry operations
- Decision set operations
- Configuration loading
- Scaling with number of decision types

## Troubleshooting

### Configuration Not Found

**Error**: `Configuration file not found: decision_config.yaml`

**Solution**: Ensure the config file exists at the specified path. Use absolute paths or verify working directory.

### Missing Required Types

**Error**: `Missing required decision type 'allow'` or `'deny'`

**Solution**: Add both `allow` and `deny` to your configuration:

```yaml
decision_types:
  - name: allow
    precedence: 100

  - name: deny
    precedence: 200
```

### Invalid Name Format

**Error**: `Invalid decision type name 'Alert': Name must contain only lowercase letters`

**Solution**: Use lowercase names: `alert`, `validate`, `audit_log`

### Parser Support

**Note**: The `effect(name)` syntax for custom decision types requires parser modifications (LALRPOP grammar) which are not yet complete.

**Current**: Use the multi-valued API programmatically with `Effect::Custom(id)`

**Future**: Full parser support for `effect(alert)`, `effect(validate)`, etc.

## Migration Guide

### From Binary to Multi-Valued

1. **Add configuration file** with at least `allow` and `deny`
2. **Load configuration** at startup
3. **Gradually adopt** multi-valued API where beneficial
4. **Keep legacy API** for existing code - it continues to work

### Adding Custom Decision Types

1. **Add to configuration**:
   ```yaml
   - name: my_custom_type
     precedence: 70
   ```

2. **Define combination rules** to control interactions:
   ```yaml
   combination_rules:
     - when: [allow, my_custom_type]
       then: merge
     # Add exclusive rules if needed
     - when: [my_custom_type, some_other_type]
       then: exclusive
       result: [my_custom_type]
   ```

3. **Check for custom decisions** in application code:
   ```rust
   let custom_id = registry.get_id("my_custom_type").unwrap();
   if multi_response.has_decision(custom_id) {
       // Handle custom decision
   }
   ```

4. **Restart application** to load new configuration

**Note**: By default, decisions that don't have explicit combination rules will MERGE (coexist). Define explicit rules to create exclusive behaviors.

## Best Practices

### 1. Start Simple

Begin with basic `allow` and `deny`, add custom types as needed.

### 2. Use Descriptive Names

Choose clear names that indicate purpose: `audit`, `validate`, `alert`, not `type1`, `type2`.

### 3. Set Appropriate Precedence

- Critical blocking decisions (deny): High precedence (200+)
- Primary authorization (allow): Medium precedence (100)
- Supplementary (alert, audit): Low precedence (< 100)

### 4. Test Combination Rules

Verify rules produce expected results:

```rust
let resolved = registry.resolve(&[allow_id, deny_id, alert_id]);
assert_eq!(resolved, vec![deny_id]); // Deny exclusive
```

### 5. Document Custom Types

Maintain documentation explaining what each custom decision type means and how applications should respond.

### 6. Monitor Performance

Use benchmarks to ensure performance targets are met after configuration changes.

## Additional Resources

- **Example Configuration**: `examples/decision_config.yaml`
- **Example Policies**: `examples/basic_multi_decision.cedar`
- **Integration Example**: `examples/integration_example.rs`
- **Performance Benchmarks**: `benches/multi_decision_bench.rs`
- **Test Suite**: Run `cargo test --lib` to see examples in action

## Support

For issues, questions, or feature requests, please refer to the Cedar Policy Engine documentation and community resources.
