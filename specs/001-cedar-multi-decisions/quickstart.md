# Quickstart Guide: Cedar Multi-Valued Decisions

**Date**: 2026-03-18
**Incorporates**: Operational clarifications (fail-fast config, restart required)

## Quick Start (5 Minutes)

### 1. Create Configuration File (REQUIRED)

⚠️ **Configuration file MUST exist before startup** (fail-fast per clarification)

Create `decision_config.yaml`:
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

conflict_resolution: precedence
```

### 2. Write Multi-Valued Policies

Create `policies.cedar`:
```cedar
// Basic access control
permit(principal, action == Action::"read", resource)
    when { principal.department == resource.department };

// Alert on sensitive access
effect(alert)(principal, action == Action::"read", resource)
    when { resource.classification == "sensitive" };
```

### 3. Use the API

```rust
use cedar_policy_core::{Authorizer, DecisionConfig, DecisionTypeRegistry};
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load config (FAILS if file missing per clarification)
    let config = DecisionConfig::from_file("decision_config.yaml")?;
    let registry = Arc::new(DecisionTypeRegistry::from_config(&config)?);

    // Create authorizer
    let policies = PolicySet::from_file("policies.cedar")?;
    let authorizer = Authorizer::new(policies, registry);

    // Evaluate
    let request = Request::new(principal, action, resource, context);
    let response = authorizer.decisions(&request, &entities, &schema)?;

    // Handle decisions
    if response.decision_set.has("allow") {
        println!("✓ Access granted");
        if response.decision_set.has("alert") {
            println!("⚠️  Security alert triggered");
        }
    } else {
        println!("✗ Access denied");
    }

    Ok(())
}
```

## Development Workflow

### Build and Test
```bash
cargo build
cargo test
cargo bench  # Performance validation
```

### Configuration Updates (Per Clarification)

⚠️ **Configuration changes require restart** (no hot-reload per clarification)

1. Edit `decision_config.yaml`
2. Validate: `cargo test config_validation`
3. **Restart application** (required per clarification)
4. Verify startup logs show successful config load

### Deployment (Zero-Downtime)

```bash
# Kubernetes example
kubectl apply -f decision-config.yaml
kubectl rollout restart deployment/auth-service

# Docker compose example
docker-compose up -d --force-recreate auth-service
```

## Key Implementation Tasks

### 1. Decision Registry
**Location**: `cedar-policy-core/src/entities/decision_registry.rs`

Implement fail-fast configuration loading:
```rust
impl DecisionTypeRegistry {
    pub fn from_config(config: &DecisionConfig) -> Result<Self, ConfigError> {
        // Validate config (fail-fast on errors per clarification)
        // Build registry
        // No hot-reload support (immutable per clarification)
    }
}
```

### 2. Effect Enum Extension
**Location**: `cedar-policy-core/src/ast/policy.rs`

Add Custom variant:
```rust
pub enum Effect {
    Permit,
    Forbid,
    Custom(DecisionTypeId),  // NEW
}
```

### 3. Parser Grammar
**Location**: `cedar-policy-core/src/parser/grammar.lalrpop`

Add effect(name) syntax:
```lalrpop
Effect: Effect = {
    "permit" => Effect::Permit,
    "forbid" => Effect::Forbid,
    "effect" "(" <name:Ident> ")" => Effect::CustomName(name),  // NEW
};
```

### 4. Evaluator
**Location**: `cedar-policy-core/src/evaluator/evaluator.rs`

Implement multi-decision evaluation:
```rust
pub fn evaluate_multi(...) -> Result<DecisionSet, EvaluationError> {
    // Collect matching effects
    // Apply combination rules
    // Resolve precedence
}
```

### 5. Authorizer API
**Location**: `cedar-policy-core/src/authorizer/mod.rs`

Add extended API:
```rust
pub fn decisions(...) -> Result<MultiResponse, EvaluationError> {
    // Full multi-valued evaluation
}

pub fn is_authorized(...) -> Result<Response, EvaluationError> {
    // Legacy: calls decisions() and converts
}
```

## Testing Strategy

### Unit Tests
```bash
cargo test decision_registry      # Registry tests
cargo test config_loading          # Config tests (including missing file)
cargo test decision_set            # DecisionSet tests
cargo test extended_effect         # Parser tests
```

### Integration Tests
```bash
cargo test --test multi_decision_e2e
cargo test --test backward_compat_test
cargo test --test config_error_test  # Test fail-fast behavior
```

### Performance Benchmarks
```bash
cargo bench
# Validate: <5% binary overhead, <15% multi-valued overhead
```

## Common Patterns

### Pattern 1: Allow with Monitoring
```cedar
permit(principal, action, resource);
effect(alert)(principal, action, resource) when { resource.sensitive };
```

```rust
if response.decision_set.has("allow") {
    grant_access();
    if response.decision_set.has("alert") {
        monitor_access();
    }
}
```

### Pattern 2: Conditional Validation
```cedar
permit(principal, action == Action::"transfer", resource);
effect(validate)(principal, action == Action::"transfer", resource)
    when { resource.amount > 10000 };
```

```rust
if response.decision_set.has("allow") {
    if response.decision_set.has("validate") {
        require_2fa()?;
    }
    process_transfer();
}
```

## Troubleshooting

### Error: Configuration file not found
```
Error: Configuration file not found: decision_config.yaml
  Configuration is required for multi-valued decision support.
```

**Solution**: Create `decision_config.yaml` before starting application (fail-fast per clarification)

### Configuration Changes Not Applied
**Solution**: Restart application (updates require restart per clarification)

### Unknown Decision Type in Policy
```
Error: Unknown decision type 'alrt'
  Available: allow, deny, alert, validate, audit
```

**Solution**: Fix typo in policy or add decision type to config + restart

## Operational Notes (From Clarifications)

1. **Config file is REQUIRED** - System fails to start if missing (fail-fast)
2. **Updates require restart** - No hot-reload support (immutable registry)
3. **Use rolling restarts** - For zero-downtime config updates
4. **Validate before deploy** - Run `cargo test config_validation` first

## Next Steps

1. Review [`spec.md`](../spec.md) for requirements
2. Review [`data-model.md`](../data-model.md) for entity definitions
3. Review [`contracts/`](../contracts/) for API and syntax details
4. Run `/speckit.tasks` to generate implementation task list
5. Run `/speckit.implement` to execute tasks

## Resources

- Cedar Documentation: https://docs.cedarpolicy.com/
- Rust Book: https://doc.rust-lang.org/book/
- LALRPOP Guide: https://lalrpop.github.io/lalrpop/
- Criterion Benchmarking: https://bheisler.github.io/criterion.rs/book/
