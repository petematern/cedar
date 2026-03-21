# Research & Technical Decisions: Cedar Multi-Valued Decisions

**Date**: 2026-03-18
**Feature**: Cedar Multi-Valued Authorization Decisions
**Purpose**: Document research findings and technical decisions for extending Cedar Policy Engine

## Overview

This document consolidates research findings that inform the implementation approach for multi-valued authorization decisions in Cedar. All technical uncertainties have been resolved through research and analysis of Cedar's architecture, Rust best practices, and similar authorization systems. **This revision incorporates operational clarifications about configuration file handling (fail-fast) and restart requirements.**

---

## Research Topics

### 1. Cedar Policy Engine Architecture

**Research Question**: How does Cedar's current architecture handle policy effects, and where should we inject multi-valued decision logic?

**Findings**:
- Cedar uses a pipeline: Parse → Validate → Evaluate → Authorize
- Effect enum is defined in `cedar-policy-core/src/ast/policy.rs` with two variants: `Permit` and `Forbid`
- Parser uses LALRPOP grammar generator with CST→AST conversion
- Evaluator in `cedar-policy-core/src/evaluator/` collects matching policies and applies precedence (forbid wins)
- Authorizer provides public API with `is_authorized()` returning binary `Decision`

**Decision**: **Hybrid Extension Approach**
- Extend Effect enum with `Custom(DecisionTypeId)` variant while preserving `Permit`/`Forbid`
- Add decision registry as new module in `entities/` directory
- Introduce `decisions()` as new API alongside legacy `is_authorized()`
- Leverage existing parser infrastructure with minimal grammar changes

**Rationale**:
- Minimizes breaking changes by keeping legacy variants
- Registry pattern provides runtime flexibility without code changes
- Separate API allows incremental adoption
- Follows Cedar's existing patterns (entities module for shared types)

**Alternatives Considered**:
- **Pure compile-time approach**: Rejected - would require recompiling Cedar for each decision type addition
- **String-based effects**: Rejected - loses type safety and performance
- **Completely separate evaluator**: Rejected - duplicates logic and increases maintenance burden

---

### 2. YAML Configuration Library Selection

**Research Question**: Which Rust YAML library provides the best balance of features, safety, and Cedar compatibility?

**Findings**:
- **serde_yaml**: Most popular, well-maintained, integrates with serde ecosystem
- **yaml-rust**: Lower-level, no serde integration
- **serde_yaml** v0.9+ supports safe parsing with size limits and recursion protection
- **serde_yaml** provides detailed error reporting with line/column information

**Decision**: **Use `serde_yaml` v0.9+**

**Rationale**:
- Cedar already uses `serde` for schema serialization (consistency)
- Type-safe deserialization into Rust structs
- **Excellent error messages** for configuration validation (supports clarification requirement)
- Active maintenance and security updates
- Supports both string and file loading with clear error propagation

**Configuration Schema** (detailed in Phase 1):
```yaml
decision_types:
  - name: "allow"
    precedence: 100
    combinable: true
    exclusive: false

combination_rules:
  - when: ["deny", "*"]
    then: "exclusive"
    result: ["deny"]

conflict_resolution: "precedence"
```

**Alternatives Considered**:
- **TOML (toml-rs)**: Rejected - less suitable for nested rules, less familiar to ops teams
- **JSON**: Rejected - less human-readable, no comments support for config documentation
- **Custom DSL**: Rejected - adds complexity, steeper learning curve, harder validation

---

### 3. Decision Type Identifier Design

**Research Question**: How should decision types be identified internally for performance and type safety?

**Findings**:
- String comparisons are O(n) per character
- Integer IDs provide O(1) equality checks
- Rust's `newtype` pattern provides type safety over raw integers
- BTreeSet/BTreeMap provide O(log n) operations with sorted iteration

**Decision**: **Use newtype `DecisionTypeId(u32)` with registry lookup**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DecisionTypeId(u32);

pub struct DecisionTypeRegistry {
    types: HashMap<String, DecisionTypeMetadata>,  // name → metadata
    id_to_name: Vec<String>,                       // id → name (indexed)
    precedence_order: Vec<DecisionTypeId>,         // sorted by precedence
}
```

**Rationale**:
- O(1) comparisons for decision type equality
- O(log n) precedence lookups via BTreeSet
- Type safety prevents mixing IDs with other integers
- Compact memory representation (4 bytes per ID)
- Easy debugging with Display trait showing name

**Alternatives Considered**:
- **String keys everywhere**: Rejected - poor performance for repeated comparisons
- **Enums with unknown variant**: Rejected - can't add types without recompiling
- **HashMap with hashed IDs**: Rejected - loses ordering needed for precedence

---

### 4. Parser Grammar Extension Strategy

**Research Question**: How should we extend Cedar's LALRPOP grammar to support `effect(name)` syntax without breaking existing policies?

**Findings**:
- LALRPOP supports alternation in grammar rules
- Cedar's current grammar: `Effect ::= 'permit' | 'forbid'`
- CST→AST conversion happens in `cst_to_ast.rs` with validation
- Parser errors are collected and reported with source locations

**Decision**: **Add effect function call syntax as third alternative**

```lalrpop
Effect: Effect = {
    "permit" => Effect::Permit,
    "forbid" => Effect::Forbid,
    "effect" "(" <name:IDENT> ")" => Effect::Custom(name),  // NEW
}
```

**Validation Strategy**:
- Parse phase: Accept any identifier in `effect(...)`
- AST construction phase: Validate against registry, error if unknown
- Error message includes suggestion for valid decision types

**Rationale**:
- Backward compatible - existing `permit`/`forbid` parse unchanged
- Clear distinction between legacy and extended syntax
- Function call syntax is familiar and extensible
- Validation deferred to AST phase allows better error messages

**Alternatives Considered**:
- **New keyword per type** (`alert`, `validate`, etc.): Rejected - requires parser changes for each type
- **Attribute syntax** `@effect(name)`: Rejected - doesn't fit Cedar's style
- **String literals** `effect("alert")`: Rejected - less clean, doesn't leverage IDENT tokenization

---

### 5. Combination Rules and Conflict Resolution

**Research Question**: How should multiple concurrent decisions be combined, and what happens when they conflict?

**Findings**:
- AWS IAM uses explicit deny precedence
- Open Policy Agent (OPA) supports custom conflict resolution
- XACML has policy combining algorithms (deny-overrides, permit-overrides, first-applicable)

**Decision**: **Configurable precedence-based resolution with combination rules**

**Default Behavior**:
1. Collect all matching decision types from policies
2. Check exclusivity: If exclusive type present, filter out incompatible types
3. Apply combination rules: Merge compatible types
4. Sort by precedence: Higher values win in conflicts
5. Return DecisionSet with all applicable decisions

**Configuration Example**:
```yaml
# Built-in: deny is exclusive and highest precedence
decision_types:
  - name: "deny"
    precedence: 200
    exclusive: true  # Removes "allow" when present

  - name: "allow"
    precedence: 100
    combinable: true  # Can coexist with alert, validate, audit

combination_rules:
  - when: ["allow", "alert"]
    then: "merge"  # Both present in result

  - when: ["deny", "*"]
    then: "exclusive"  # Only deny in result
```

**Rationale**:
- Precedence provides deterministic resolution
- Exclusivity handles deny-overrides pattern
- Combination rules support complex scenarios
- Default to precedence keeps simple cases simple
- Matches familiar authorization semantics

**Alternatives Considered**:
- **Always merge all**: Rejected - can't handle conflicting decisions (allow + deny)
- **First-match wins**: Rejected - policy order dependency is fragile
- **Hard-coded rules**: Rejected - not flexible enough for different domains

---

### 6. Performance Optimization Strategy

**Research Question**: How can we minimize performance overhead for both binary and multi-valued decisions?

**Findings**:
- Current Cedar binary decision: O(n) policy evaluation + O(1) precedence
- Multi-valued adds: O(m) decision tracking + O(m log m) sorting (m = decision types)
- Typical m = 2-5, so overhead is manageable
- Rust's zero-cost abstractions allow optimization without runtime cost

**Decision**: **Lazy evaluation with fast paths**

**Optimizations**:
1. **Fast path for binary**: Detect legacy-only policies, skip DecisionSet construction
2. **Inline decision tracking**: Use SmallVec or inline array for m ≤ 8 decisions
3. **Pre-sorted precedence**: Registry maintains sorted order, no runtime sorting
4. **Short-circuit exclusive**: Stop evaluating when exclusive decision found
5. **Benchmark-driven**: Use criterion for continuous performance validation

**Rust-Specific Techniques**:
- Use `BTreeSet` for automatic sorting during insertion
- `Arc<DecisionTypeRegistry>` for cheap clones across threads
- `Copy` trait for DecisionTypeId (4 bytes)
- Iterator chains to avoid intermediate allocations

**Rationale**:
- Fast path ensures <5% overhead for binary decisions
- Lazy evaluation only pays for multi-valued when needed
- Rust's compile-time optimizations eliminate abstraction cost
- Benchmarking ensures performance targets are met

**Alternatives Considered**:
- **Always full DecisionSet**: Rejected - unnecessary cost for binary cases
- **Separate code paths**: Rejected - duplicates logic, hard to maintain
- **Caching**: Rejected - authorization depends on dynamic context, cache hit rate would be low

---

### 7. Backward Compatibility Strategy

**Research Question**: How can we guarantee 100% compatibility with existing Cedar policies and APIs?

**Findings**:
- Cedar has extensive test suite and public API contracts
- Breaking changes require major version bump
- Existing applications rely on `is_authorized()` returning `Response { decision: Decision, diagnostics: Diagnostics }`

**Decision**: **Dual API with internal conversion layer**

**Implementation**:
```rust
// Legacy API (unchanged signature)
pub fn is_authorized(&self, request: &Request, entities: &Entities, schema: &Schema) -> Response {
    let multi_response = self.decisions(request, entities, schema);
    Response {
        decision: multi_response.decision_set.to_decision(),  // Convert to binary
        diagnostics: multi_response.diagnostics,
    }
}

// Extended API (new)
pub fn decisions(&self, request: &Request, entities: &Entities, schema: &Schema) -> MultiResponse {
    // Full multi-valued evaluation
}
```

**to_decision() Logic**:
```rust
impl DecisionSet {
    pub fn to_decision(&self) -> Decision {
        if self.has("allow") && !self.has("deny") {
            Decision::Allow
        } else {
            Decision::Deny  // Default deny (Cedar semantics)
        }
    }
}
```

**Testing Strategy**:
- Run entire Cedar test suite against modified codebase
- Add regression tests for binary API behavior
- Fuzz testing with existing policies

**Rationale**:
- Legacy API signature completely unchanged
- Internal implementation can be shared (DRY)
- Applications can adopt multi-valued at their own pace
- No migration required for existing users

**Alternatives Considered**:
- **Feature flag**: Rejected - runtime branching adds complexity
- **Separate crate**: Rejected - duplicates too much Cedar code
- **Deprecate old API**: Rejected - breaks existing users

---

### 8. Thread Safety and Concurrency

**Research Question**: How should the decision registry and evaluation handle concurrent access?

**Findings**:
- Cedar's Authorizer is designed for concurrent use (shared references)
- Registry is immutable after initialization
- Rust's ownership system enforces thread safety at compile time

**Decision**: **Immutable registry with Arc sharing**

```rust
pub struct Authorizer {
    policies: PolicySet,
    registry: Arc<DecisionTypeRegistry>,  // Shared immutable reference
}

impl DecisionTypeRegistry {
    // All methods take &self (shared reference)
    pub fn get_id(&self, name: &str) -> Option<DecisionTypeId> { ... }
}
```

**Thread Safety Guarantees**:
- Registry is `Send + Sync` (can be shared across threads)
- No interior mutability (no Mutex/RwLock overhead)
- **Initialized once at authorizer creation** (incorporates clarification)
- Cheap to clone Authorizer (Arc is atomic pointer)

**Rationale**:
- Zero runtime synchronization cost
- Compile-time enforcement of safety
- Matches Cedar's existing concurrency model
- Simple mental model (immutable after init)
- **Aligns with restart-based config updates** (no race conditions from hot-reload)

**Alternatives Considered**:
- **RwLock for hot-reload**: Rejected - ✅ **Clarification: no hot-reload needed**
- **Thread-local registries**: Rejected - wastes memory, complicates initialization
- **Global static registry**: Rejected - testing becomes harder, can't have multiple configs

---

### 9. Configuration Error Handling (Incorporates Clarification)

**Research Question**: How should the system behave when configuration is invalid, missing, or inaccessible?

**Clarification Answer**: **Fail startup with error** - system refuses to initialize without valid configuration file

**Decision**: **Fail-Fast Configuration Validation**

**Implementation Strategy**:
```rust
impl DecisionConfig {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();

        // Clear error if file doesn't exist
        let contents = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::FileNotFound {
                path: path.to_path_buf(),
                cause: e,
                message: format!(
                    "Configuration file not found: {}\n\
                     Ensure the file exists and is readable.\n\
                     Configuration is required for multi-valued decision support.",
                    path.display()
                ),
            })?;

        // Parse YAML with clear errors
        let config: DecisionConfig = serde_yaml::from_str(&contents)
            .map_err(|e| ConfigError::ParseError {
                path: path.to_path_buf(),
                cause: e,
                line: e.location().map(|l| l.line()),
                column: e.location().map(|l| l.column()),
            })?;

        // Validate semantic rules
        config.validate()?;

        Ok(config)
    }
}

impl DecisionTypeRegistry {
    pub fn from_config(config: &DecisionConfig) -> Result<Self, ConfigError> {
        // Validate config structure
        // Build registry
        // Return error on any validation failure
    }
}

impl Authorizer {
    pub fn new(policies: PolicySet, registry: Arc<DecisionTypeRegistry>) -> Self {
        // Registry must be valid (validated at construction)
        // No runtime config checks needed
    }
}
```

**Error Message Design**:
```
Error: Configuration file not found: /etc/cedar/decision_config.yaml
  Cause: No such file or directory (os error 2)

Configuration is required for multi-valued decision support.

Suggested actions:
  1. Create configuration file at /etc/cedar/decision_config.yaml
  2. Provide a different path using --config option
  3. See examples/decision_config.yaml for reference

Documentation: https://docs.cedarpolicy.com/multi-valued-decisions/configuration
```

**Rationale**:
- ✅ **Clear feedback** - operators know immediately what's wrong
- ✅ **Prevents silent failures** - no runtime surprises from missing config
- ✅ **Simple deployment model** - config presence is a deployment requirement
- ✅ **Testable** - can verify error messages and failure modes
- ✅ **Consistent with library patterns** - many Rust libraries fail-fast on config errors

**Testing Requirements**:
- Unit test: missing file → FileNotFound error with path
- Unit test: unreadable file → permission error
- Unit test: invalid YAML → ParseError with line/column
- Unit test: semantic errors → ValidationError with details
- Integration test: startup without config → initialization failure

**Alternatives Considered** (from clarification):
- **Fall back to default (allow/deny only)**: Rejected by user - want explicit config
- **Search multiple locations**: Rejected by user - adds complexity
- **Require explicit path**: Partial - constructor takes config explicitly

---

### 10. Operational Model: Configuration Updates (Incorporates Clarification)

**Research Question**: How should operators update configuration in production environments?

**Clarification Answer**: **Restart required** - configuration changes require restarting the application/service

**Decision**: **Restart-Based Configuration Updates**

**Operational Model**:
1. **Development**: Edit `decision_config.yaml` file
2. **Validation**: Test config with `cargo test` (including config validation tests)
3. **Deployment**: Deploy updated config file to production
4. **Update**: Restart application/service (rolling restart for zero-downtime)
5. **Verification**: Check logs for successful config load

**Implementation Implications**:
- Registry is `Arc<DecisionTypeRegistry>` - **immutable after creation**
- No setter methods on registry
- No file watchers or reload APIs
- No version tracking or hot-swap logic
- Simple lifetime model: registry lives as long as Authorizer

**Documentation Requirements**:
- `quickstart.md`: Document restart requirement prominently
- `README.md`: Include deployment section with restart instructions
- Error messages: Suggest restart when config issues detected
- Examples: Show proper lifecycle management

**Deployment Best Practices**:
```yaml
# Kubernetes rolling update example
apiVersion: apps/v1
kind: Deployment
spec:
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxUnavailable: 1
      maxSurge: 1
  template:
    spec:
      containers:
      - name: authorization-service
        volumeMounts:
        - name: config
          mountPath: /etc/cedar/decision_config.yaml
          subPath: decision_config.yaml
      volumes:
      - name: config
        configMap:
          name: cedar-decision-config

# Update: kubectl apply -f decision-config.yaml && kubectl rollout restart deployment/auth-service
```

**Rationale**:
- ✅ **Simplicity** - no hot-reload complexity, no config versioning
- ✅ **Predictability** - restart = clean state, no partial updates
- ✅ **Standard practice** - common for library-level configuration
- ✅ **Reduced error surface** - no race conditions from concurrent config updates
- ✅ **Testability** - deterministic initialization, easy to test
- ✅ **Operational clarity** - clear contract with deployment teams

**Trade-offs Accepted**:
- **Brief downtime during update**: Mitigated by rolling restarts in production
- **Cannot A/B test configs**: Config changes are deterministic, not experimental
- **No runtime experimentation**: Feature flags at application level if needed

**Alternatives Considered** (from clarification):
- **In-place reload via API/signal**: Rejected by user - adds complexity
- **Versioned configuration files**: Rejected by user - filesystem coupling
- **External config service**: Rejected by user - external dependency

---

## Summary of Key Decisions

| Category | Decision | Primary Rationale | Clarification Impact |
|----------|----------|-------------------|---------------------|
| Architecture | Hybrid extension (preserve + extend) | Backward compatibility + type safety | - |
| Configuration | YAML with serde_yaml | Human-readable, integrates with Cedar ecosystem | ✅ Excellent error reporting |
| Config Errors | Fail-fast on missing/invalid config | Clear feedback, prevents runtime surprises | ✅ From clarification Q1 |
| Config Updates | Restart required (immutable registry) | Simplicity, predictability, standard practice | ✅ From clarification Q2 |
| Identifiers | Newtype DecisionTypeId(u32) | Performance + type safety | - |
| Parser | effect(name) function syntax | Extensible, doesn't require parser changes per type | - |
| Combination | Precedence + exclusivity rules | Deterministic, handles conflicts, configurable | - |
| Performance | Lazy evaluation with fast paths | <5% binary overhead, <15% multi-valued | - |
| Compatibility | Dual API (legacy + extended) | Zero migration burden, gradual adoption | - |
| Concurrency | Immutable Arc<Registry> | Zero-cost sharing, compile-time safety | ✅ Aligns with restart model |

---

## Implementation Readiness

All technical uncertainties have been resolved, including operational requirements. The design leverages:
- ✅ Cedar's existing architecture patterns
- ✅ Rust's type system for safety and performance
- ✅ Industry-proven authorization patterns
- ✅ Minimal API surface expansion
- ✅ **Clear operational contracts (fail-fast, restart-based)**
- ✅ **Explicit error handling for deployment issues**

**Clarifications Incorporated**:
1. Configuration file presence is **mandatory** - system fails to start without it
2. Configuration updates require **application restart** - no hot-reload complexity
3. Operational model is **simple and predictable** - restart = clean state

**Next Phase**: Design detailed data models and contracts (Phase 1)
