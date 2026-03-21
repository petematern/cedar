# Data Model: Cedar Multi-Valued Decisions

**Date**: 2026-03-18
**Feature**: Cedar Multi-Valued Authorization Decisions
**Purpose**: Define core entities, types, and their relationships
**Revision**: Incorporates operational clarifications (fail-fast config, restart required)

## Overview

This document defines the data structures and types that support multi-valued authorization decisions. All types are designed for the Rust implementation with consideration for performance, type safety, backward compatibility, and clear operational contracts.

---

## Core Entities

### 1. DecisionTypeId

**Purpose**: Type-safe unique identifier for decision types

**Definition**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DecisionTypeId(u32);
```

**Traits**:
- `Copy`: Cheap to pass by value (4 bytes)
- `Ord`: Enables sorting for precedence
- `Hash`: Enables use in HashMap/HashSet

**Validation Rules**:
- Valid range: 0 to u32::MAX
- 0-99 reserved for built-in types (allow=0, deny=1)
- 100+ for custom types

**Relationships**:
- Owned by: DecisionTypeRegistry (maps to metadata)
- Used in: Effect::Custom, DecisionSet, CombinationRule

---

### 2. DecisionTypeMetadata

**Purpose**: Describes properties and behavior of a decision type

**Definition**:
```rust
pub struct DecisionTypeMetadata {
    pub id: DecisionTypeId,
    pub name: String,
    pub precedence: u32,
    pub combinable: bool,
    pub exclusive: bool,
}
```

**Fields**:
- **id**: Unique identifier (immutable after creation)
- **name**: Human-readable name (e.g., "allow", "deny", "alert")
  - Validation: lowercase alphanumeric + underscore, 1-32 chars
  - Must be unique within registry
- **precedence**: Conflict resolution priority (higher wins)
  - Built-in: deny=200, allow=100
  - Custom: operator-defined (typically 50-150)
- **combinable**: Can coexist with other decision types
  - true: Can appear with others (alert, audit, validate)
  - false: Standalone decision (rare)
- **exclusive**: Excludes other decisions when present
  - true: Only this decision in final set (deny behavior)
  - false: Can coexist per combination rules

**Invariants**:
- `exclusive=true` implies `combinable=false`
- `precedence` values should be spaced (e.g., by 10) for future insertions

**State Transitions**: None (immutable after creation)

---

### 3. DecisionTypeRegistry

**Purpose**: Central registry managing all decision types and their interactions

**Definition**:
```rust
pub struct DecisionTypeRegistry {
    types: HashMap<String, DecisionTypeMetadata>,
    id_to_name: Vec<String>,
    precedence_order: Vec<DecisionTypeId>,
}
```

**Fields**:
- **types**: Name → metadata lookup (O(1))
- **id_to_name**: ID → name mapping via indexing (O(1))
- **precedence_order**: Pre-sorted IDs by precedence (highest first)

**Invariants**:
- All IDs are unique
- All names are unique (case-insensitive check)
- Built-in types (allow, deny) always present
- precedence_order matches metadata precedence values
- **Registry is immutable after construction** (supports restart-based updates)

**Initialization** (incorporates fail-fast requirement):
```rust
impl DecisionTypeRegistry {
    /// Create registry from configuration
    ///
    /// # Errors
    /// Returns ConfigError if:
    /// - Config file missing or inaccessible (incorporates clarification)
    /// - Invalid YAML syntax
    /// - Semantic validation failures (duplicates, missing required types, etc.)
    pub fn from_config(config: &DecisionConfig) -> Result<Self, ConfigError> {
        // 1. Validate config (no duplicates, valid names, allow/deny present)
        // 2. Reserve IDs: allow=0, deny=1, then sequential
        // 3. Build hash maps
        // 4. Sort precedence_order
        // 5. Validate combination rules reference existing types
        // 6. Return error on any failure (fail-fast)
    }

    /// Create minimal registry (allow/deny only) - for testing
    pub fn default() -> Self {
        // Used only in test contexts
        // Production code must use from_config()
    }
}
```

**Key Methods**:
```rust
// Lookup operations (immutable, thread-safe)
pub fn get_id(&self, name: &str) -> Option<DecisionTypeId>;
pub fn get_name(&self, id: DecisionTypeId) -> Option<&str>;
pub fn get_metadata(&self, id: DecisionTypeId) -> Option<&DecisionTypeMetadata>;

// Combination logic
pub fn can_combine(&self, a: DecisionTypeId, b: DecisionTypeId) -> bool;
pub fn resolve(&self, decisions: Vec<DecisionTypeId>) -> DecisionSet;

// Validation
pub fn validate_effect_name(&self, name: &str) -> Result<DecisionTypeId, ValidationError>;
```

**Thread Safety**:
- Immutable after construction
- Wrapped in `Arc<DecisionTypeRegistry>` for sharing across threads
- **No hot-reload support** (incorporates clarification)

---

### 4. DecisionSet

**Purpose**: Result of authorization evaluation containing multiple concurrent decisions

**Definition**:
```rust
pub struct DecisionSet {
    decisions: BTreeSet<DecisionTypeId>,
    policies: HashMap<DecisionTypeId, Vec<PolicyId>>,
    registry: Arc<DecisionTypeRegistry>,
}
```

**Fields**:
- **decisions**: Set of active decision types (automatically sorted by Ord)
- **policies**: Maps each decision to contributing policy IDs (diagnostics)
- **registry**: Reference to registry for name lookups

**Invariants**:
- All DecisionTypeIds in `decisions` exist in `registry`
- All DecisionTypeIds in `policies` are also in `decisions`
- If exclusive decision present, no incompatible decisions

**Key Methods**:
```rust
// Query operations
pub fn has(&self, name: &str) -> bool;
pub fn primary(&self) -> DecisionTypeId;  // Highest precedence
pub fn all(&self) -> &BTreeSet<DecisionTypeId>;
pub fn policies_for(&self, name: &str) -> Option<&[PolicyId]>;

// Conversion for backward compatibility
pub fn to_decision(&self) -> Decision {
    if self.has("allow") && !self.has("deny") {
        Decision::Allow
    } else {
        Decision::Deny
    }
}

// Combination
pub fn merge(&mut self, other: DecisionSet);
pub fn apply_exclusivity(&mut self);
```

**Construction**:
```rust
impl DecisionSet {
    pub fn new(registry: Arc<DecisionTypeRegistry>) -> Self;

    pub fn from_effects(
        effects: Vec<Effect>,
        policies: Vec<PolicyId>,
        registry: Arc<DecisionTypeRegistry>,
    ) -> Self {
        // 1. Convert effects to DecisionTypeIds
        // 2. Group by decision type
        // 3. Apply combination rules
        // 4. Resolve precedence conflicts
    }
}
```

---

### 5. Effect (Extended)

**Purpose**: Represents the outcome specified by a policy

**Definition**:
```rust
pub enum Effect {
    /// Legacy permit (maps to "allow")
    Permit,

    /// Legacy forbid (maps to "deny")
    Forbid,

    /// Custom decision type
    Custom(DecisionTypeId),
}
```

**Conversion**:
```rust
impl Effect {
    pub fn decision_type(&self, registry: &DecisionTypeRegistry) -> DecisionTypeId {
        match self {
            Effect::Permit => registry.get_id("allow").unwrap(),
            Effect::Forbid => registry.get_id("deny").unwrap(),
            Effect::Custom(id) => *id,
        }
    }

    pub fn is_legacy(&self) -> bool {
        matches!(self, Effect::Permit | Effect::Forbid)
    }
}
```

**Validation Rules**:
- `Custom(id)` must reference valid ID in registry
- Validated during CST→AST conversion

**State Transitions**: Immutable (part of AST)

---

### 6. CombinationRule

**Purpose**: Defines how decision types interact when multiple match

**Definition**:
```rust
pub struct CombinationRule {
    pub when: Vec<DecisionPattern>,
    pub then: CombinationStrategy,
    pub result: Option<Vec<DecisionTypeId>>,
}

pub enum DecisionPattern {
    Specific(DecisionTypeId),
    Wildcard,  // Matches any decision type
}

pub enum CombinationStrategy {
    Merge,       // Include all matching decisions
    Exclusive,   // Only include decisions from result field
    Override,    // Replace with result field
}
```

**Evaluation**:
```rust
impl CombinationRule {
    pub fn matches(&self, decisions: &BTreeSet<DecisionTypeId>) -> bool {
        // Check if all patterns in `when` match decisions
    }

    pub fn apply(&self, decisions: &mut BTreeSet<DecisionTypeId>) {
        match self.then {
            Merge => { /* no-op, already merged */ }
            Exclusive => {
                decisions.clear();
                decisions.extend(self.result.as_ref().unwrap());
            }
            Override => {
                decisions.clear();
                decisions.extend(self.result.as_ref().unwrap());
            }
        }
    }
}
```

**Validation Rules**:
- `Exclusive` and `Override` require non-empty `result` field
- All DecisionTypeIds in patterns and results must exist in registry
- No circular dependencies (e.g., rule A → rule B → rule A)

---

### 7. DecisionConfig (Incorporates Fail-Fast Behavior)

**Purpose**: Configuration structure loaded from YAML with strict validation

**Definition**:
```rust
#[derive(Deserialize)]
pub struct DecisionConfig {
    pub decision_types: Vec<DecisionTypeConfig>,
    pub combination_rules: Option<Vec<CombinationRuleConfig>>,
    pub conflict_resolution: Option<ConflictResolution>,
}

#[derive(Deserialize)]
pub struct DecisionTypeConfig {
    pub name: String,
    pub precedence: u32,
    pub combinable: bool,
    pub exclusive: bool,
}

#[derive(Deserialize)]
pub struct CombinationRuleConfig {
    pub when: Vec<String>,  // decision names or "*"
    pub then: String,       // "merge" | "exclusive" | "override"
    pub result: Option<Vec<String>>,  // decision names
}

#[derive(Deserialize)]
pub enum ConflictResolution {
    Precedence,  // Default
    Error,       // Fail if conflict detected
    Merge,       // Include all decisions
}
```

**Loading** (incorporates clarification):
```rust
impl DecisionConfig {
    /// Load configuration from file (fail-fast on errors)
    ///
    /// # Errors
    /// - FileNotFound: Config file doesn't exist (with helpful message)
    /// - ParseError: Invalid YAML syntax (with line/column)
    /// - ValidationError: Semantic errors (with details)
    ///
    /// # Example
    /// ```rust
    /// let config = DecisionConfig::from_file("decision_config.yaml")?;
    /// // If file missing, returns Err with:
    /// // "Configuration file not found: decision_config.yaml
    /// //  Configuration is required for multi-valued decision support.
    /// //  Ensure the file exists and is readable."
    /// ```
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();

        // Fail immediately if file missing
        let contents = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::FileNotFound {
                path: path.to_path_buf(),
                cause: e,
            })?;

        // Parse YAML (fail on syntax errors)
        let config: Self = serde_yaml::from_str(&contents)
            .map_err(|e| ConfigError::ParseError {
                path: path.to_path_buf(),
                cause: e,
            })?;

        // Validate semantics (fail on validation errors)
        config.validate()?;

        Ok(config)
    }

    pub fn from_str(yaml: &str) -> Result<Self, ConfigError> {
        let config: Self = serde_yaml::from_str(yaml)?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        // Check: unique names, valid precedence values
        // Check: allow and deny are present
        // Check: exclusive implies not combinable
        // Check: combination rules reference existing types
        // Return first error found (fail-fast)
    }
}
```

**Error Types** (incorporates clarification requirements):
```rust
pub enum ConfigError {
    /// File not found or inaccessible
    FileNotFound {
        path: PathBuf,
        cause: std::io::Error,
    },

    /// YAML parsing error
    ParseError {
        path: PathBuf,
        cause: serde_yaml::Error,
    },

    /// Validation error
    ValidationError {
        kind: ValidationErrorKind,
        message: String,
    },
}

pub enum ValidationErrorKind {
    DuplicateName { name: String },
    MissingRequiredType { name: String },
    InvalidName { name: String, reason: String },
    InvalidCombinationRule { rule_index: usize, reason: String },
    CircularDependency { cycle: Vec<String> },
}
```

**Error Message Examples**:
```
Error: Configuration file not found: /etc/cedar/decision_config.yaml
  Cause: No such file or directory (os error 2)

Configuration is required for multi-valued decision support.

To resolve:
  1. Create /etc/cedar/decision_config.yaml
  2. Or provide path: --config /path/to/config.yaml
  3. See examples/decision_config.yaml for template

Documentation: https://docs.cedarpolicy.com/multi-valued/configuration
```

---

### 8. MultiResponse

**Purpose**: Extended authorization response with multi-valued results

**Definition**:
```rust
pub struct MultiResponse {
    pub decision_set: DecisionSet,
    pub diagnostics: Diagnostics,
}
```

**Fields**:
- **decision_set**: All applicable decisions
- **diagnostics**: Same as legacy Response (reasons, errors)

**Conversion to Legacy**:
```rust
impl From<MultiResponse> for Response {
    fn from(multi: MultiResponse) -> Self {
        Response {
            decision: multi.decision_set.to_decision(),
            diagnostics: multi.diagnostics,
        }
    }
}
```

---

## Entity Relationships

```
DecisionConfig (YAML file - MUST exist per clarification)
    ↓ from_file() with fail-fast validation
DecisionTypeRegistry (immutable after init, no hot-reload per clarification)
    ├─ contains → DecisionTypeMetadata
    ├─ contains → CombinationRule
    └─ referenced by → Effect::Custom(DecisionTypeId)

Policy
    └─ has → Effect (Permit | Forbid | Custom)

Authorization Evaluation
    ├─ inputs → Request, Entities, Policy Set
    ├─ uses → DecisionTypeRegistry (Arc-wrapped, immutable)
    ├─ produces → DecisionSet
    │   ├─ contains → Set<DecisionTypeId>
    │   └─ maps → DecisionTypeId → Vec<PolicyId>
    └─ outputs → MultiResponse
        ├─ contains → DecisionSet
        └─ contains → Diagnostics

MultiResponse
    └─ converts to → Response (legacy)
```

---

## Data Flow

### Initialization Flow (Incorporates Clarifications)
1. **Config file MUST exist** → `DecisionConfig::from_file(path)`
2. **Fail immediately if missing** → `Err(ConfigError::FileNotFound { ... })`
3. Parse YAML → `DecisionConfig` struct
4. Validate semantic rules → `config.validate()`
5. Build registry → `DecisionTypeRegistry::from_config(&config)?`
6. **Any error = startup failure** (fail-fast per clarification)
7. Wrap registry → `Arc::new(registry)`
8. Create authorizer → `Authorizer::new(policies, registry)`
9. **Registry immutable for lifetime** (restart required for updates per clarification)

### Authorization Flow
1. Application calls `decisions(request, entities, schema)`
2. Evaluator matches policies → `Vec<(PolicyId, Effect)>`
3. Convert effects to decision IDs → `Vec<DecisionTypeId>`
4. Apply combination rules → `DecisionSet`
5. Resolve precedence → Sort and filter by exclusivity
6. Return `MultiResponse { decision_set, diagnostics }`

### Backward Compatibility Flow
1. Application calls legacy `is_authorized(request, entities, schema)`
2. Internally calls `decisions()` → `MultiResponse`
3. Convert to binary: `decision_set.to_decision()` → `Decision`
4. Return legacy `Response { decision, diagnostics }`

### Configuration Update Flow (Incorporates Clarification)
1. Operator edits `decision_config.yaml` file
2. **Application restart required** (no hot-reload per clarification)
3. On startup: Load config → Build new registry
4. New registry used for all evaluations
5. **No in-place updates** - clean state guaranteed

---

## Performance Characteristics

| Operation | Complexity | Notes |
|-----------|------------|-------|
| Config loading (startup only) | O(n × log n) | Parse + validate + sort, n = decision types |
| Registry lookup (name → id) | O(1) | HashMap |
| Registry lookup (id → name) | O(1) | Vec indexing |
| Decision set query (`has`) | O(log m) | BTreeSet, m = decision types |
| Decision set primary | O(1) | First element in sorted set |
| Combination rule matching | O(r × m) | r = rules, m = decisions |
| Precedence resolution | O(m log m) | BTreeSet maintains order |
| Binary conversion | O(log m) | Two `has()` checks |

**Expected Scale**:
- n (decision types in config) = 2-10 (typically 5)
- m (decisions in result) = 1-5 (typically 2-3)
- r (combination rules) = 0-5
- Total overhead for multi-valued: <15% vs binary
- **Config loading overhead**: One-time at startup only (restart-based updates)

---

## Validation Summary

### DecisionConfig Validation (Fail-Fast)
- ✅ **File existence checked first** (incorporates clarification)
- ✅ Name: lowercase alphanumeric + underscore, 1-32 chars, unique
- ✅ Precedence: u32, spaced values recommended
- ✅ exclusive=true implies combinable=false
- ✅ "allow" and "deny" must be present
- ✅ **Any validation failure = startup error** (fail-fast)

### CombinationRule Validation
- ✅ All decision names in `when` exist in registry
- ✅ All decision names in `result` exist in registry
- ✅ `exclusive`/`override` strategies require non-empty `result`
- ✅ No circular rule dependencies

### Effect Validation
- ✅ Custom(id) references valid DecisionTypeId
- ✅ Permit/Forbid map to allow/deny

### DecisionSet Invariants
- ✅ All IDs exist in registry
- ✅ Exclusive decision enforced
- ✅ policies map matches decisions set

---

## Operational Contracts (From Clarifications)

### Configuration File Requirements
**Contract**: Configuration file MUST exist at specified path before startup

**Enforcement**:
- `DecisionConfig::from_file()` returns `Err(FileNotFound)` if missing
- Error message includes: expected path, suggestion, documentation link
- No fallback to default configuration
- No silent failures

**Testing**:
- Unit test: missing file → clear error message
- Integration test: startup without config → initialization failure
- Documentation: quickstart.md emphasizes config requirement

### Configuration Update Requirements
**Contract**: Configuration updates require application restart

**Enforcement**:
- `DecisionTypeRegistry` has no setter methods
- Registry wrapped in `Arc` (immutable reference)
- No file watchers or reload APIs
- Documentation explicitly states restart requirement

**Deployment**:
- Rolling restart for zero-downtime updates
- Config changes are design-time decisions (infrequent)
- Standard library configuration pattern

---

## Migration Considerations

**From Binary to Multi-Valued**:
- Legacy `Response.decision` maps to `DecisionSet.to_decision()`
- **New: Config file required** (create decision_config.yaml)
- No data model changes required for existing policies
- Configuration addition is explicit (fail-fast ensures correctness)

**Adding New Decision Types**:
1. Update YAML configuration file
2. **Restart application** (per clarification)
3. Deploy policies using new decision types
4. No code changes required

---

## Next Steps

Proceed to Phase 1 contracts definition:
- Policy syntax grammar (extended Cedar syntax)
- API contracts (function signatures with config error handling)
- Configuration schema (YAML structure with fail-fast requirements)
