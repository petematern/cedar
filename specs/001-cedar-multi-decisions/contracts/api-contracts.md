# API Contracts

**Feature**: Cedar Multi-Valued Authorization Decisions  
**Date**: 2026-03-18  
**Incorporates**: Operational clarifications (fail-fast config, restart required)

## Core Types

### DecisionSet
```rust
pub struct DecisionSet { /* opaque */ }

impl DecisionSet {
    pub fn has(&self, name: &str) -> bool;
    pub fn primary(&self) -> DecisionTypeId;
    pub fn primary_name(&self) -> &str;
    pub fn all_names(&self) -> impl Iterator<Item = &str>;
    pub fn policies_for(&self, name: &str) -> Option<&[PolicyId]>;
    pub fn to_decision(&self) -> Decision;
    pub fn is_allow(&self) -> bool;
    pub fn is_deny(&self) -> bool;
}
```

### MultiResponse
```rust
pub struct MultiResponse {
    pub decision_set: DecisionSet,
    pub diagnostics: Diagnostics,
}

impl From<MultiResponse> for Response { ... }
```

## Authorizer API

### Constructors (Incorporates Clarifications)
```rust
impl Authorizer {
    /// Create with explicit configuration (REQUIRED for multi-valued support)
    /// 
    /// # Errors
    /// - ConfigError::FileNotFound if config file missing (fail-fast per clarification)
    /// - ConfigError::ParseError if invalid YAML
    /// - ConfigError::ValidationError if semantic errors
    ///
    /// # Note
    /// Configuration updates require restart (no hot-reload per clarification)
    pub fn new(
        policies: PolicySet,
        registry: Arc<DecisionTypeRegistry>,
    ) -> Self;
}
```

### Authorization Methods

#### Extended API (New)
```rust
/// Evaluate with multi-valued decisions
pub fn decisions(
    &self,
    request: &Request,
    entities: &Entities,
    schema: &Schema,
) -> Result<MultiResponse, EvaluationError>;
```

#### Legacy API (Preserved)
```rust
/// Binary authorization (backward compatible)
pub fn is_authorized(
    &self,
    request: &Request,
    entities: &Entities,
    schema: &Schema,
) -> Result<Response, EvaluationError>;
```

## Configuration API (Incorporates Clarifications)

### DecisionConfig
```rust
impl DecisionConfig {
    /// Load from file (FAIL-FAST on missing/invalid per clarification)
    ///
    /// # Errors
    /// - FileNotFound: Config file doesn't exist (with path and remedy)
    /// - ParseError: Invalid YAML (with line/column)
    /// - ValidationError: Semantic errors (with details)
    ///
    /// # Error Example
    /// ```text
    /// Error: Configuration file not found: decision_config.yaml
    ///   Configuration is required for multi-valued decision support.
    ///   Create the file or provide path with --config option.
    /// ```
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError>;

    pub fn from_str(yaml: &str) -> Result<Self, ConfigError>;
    pub fn validate(&self) -> Result<(), ConfigError>;
}
```

### DecisionTypeRegistry
```rust
impl DecisionTypeRegistry {
    /// Build from config (fail-fast on errors)
    pub fn from_config(config: &DecisionConfig) -> Result<Self, ConfigError>;

    // Lookup methods (immutable - no hot-reload per clarification)
    pub fn get_id(&self, name: &str) -> Option<DecisionTypeId>;
    pub fn get_name(&self, id: DecisionTypeId) -> Option<&str>;
    pub fn all_names(&self) -> impl Iterator<Item = &str>;
    pub fn validate_name(&self, name: &str) -> Result<DecisionTypeId, ValidationError>;
}
```

## Error Types

### ConfigError (Incorporates Fail-Fast Requirements)
```rust
pub enum ConfigError {
    /// Configuration file not found or inaccessible
    FileNotFound {
        path: PathBuf,
        cause: std::io::Error,
    },

    /// Invalid YAML syntax
    ParseError {
        path: PathBuf,
        cause: serde_yaml::Error,
    },

    /// Semantic validation failure
    ValidationError {
        kind: ValidationErrorKind,
        message: String,
    },
}
```

## Usage Examples

### Multi-Valued Authorization
```rust
let config = DecisionConfig::from_file("decision_config.yaml")?;
let registry = Arc::new(DecisionTypeRegistry::from_config(&config)?);
let authorizer = Authorizer::new(policies, registry);

let response = authorizer.decisions(&request, &entities, &schema)?;

if response.decision_set.has("allow") {
    grant_access();

    if response.decision_set.has("alert") {
        security_monitor.log(&request);
    }

    if response.decision_set.has("validate") {
        require_two_factor(&request)?;
    }
} else {
    deny_access();
}
```

### Configuration Update Pattern (Per Clarification)
```rust
// 1. Edit decision_config.yaml
// 2. Restart application (REQUIRED - no hot-reload)
// 3. On startup, new config is loaded:

let config = DecisionConfig::from_file("decision_config.yaml")?;
let registry = Arc::new(DecisionTypeRegistry::from_config(&config)?);
let authorizer = Authorizer::new(policies, registry);

// Registry is immutable for application lifetime
// Config changes = restart required
```

## Performance Contracts

| Operation | Requirement | Notes |
|-----------|-------------|-------|
| Legacy `is_authorized()` | <5% overhead vs Cedar 3.x | Fast path for binary |
| Multi-valued (2-5 decisions) | <15% overhead | Lazy evaluation |
| Registry lookup | O(1) | HashMap |
| Decision set query | O(log m) | BTreeSet, m<10 |
| **Config load (startup)** | O(n log n) | One-time cost, n=decision types |

## Operational Contracts (From Clarifications)

### Configuration File Requirement
- **Contract**: Configuration file MUST exist before startup
- **Enforcement**: `from_file()` returns `Err(FileNotFound)` if missing
- **No fallback**: Explicit config required for multi-valued support
- **Error includes**: Path, cause, remedy suggestions

### Configuration Updates
- **Contract**: Updates require application restart
- **Enforcement**: Registry immutable after creation
- **No hot-reload**: No APIs or mechanisms for in-place updates
- **Documentation**: quickstart.md and README explicitly state requirement

## Backward Compatibility Guarantees

✅ Legacy API signature unchanged  
✅ Legacy policies produce identical results  
✅ No migration required for existing Cedar users  
✅ Performance regression <5% for binary decisions  
✅ Diagnostics preserved in both APIs
