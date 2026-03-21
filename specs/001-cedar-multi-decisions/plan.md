# Implementation Plan: Cedar Multi-Valued Authorization Decisions

**Branch**: `001-cedar-multi-decisions` | **Date**: 2026-03-18 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-cedar-multi-decisions/spec.md`

**Note**: This plan incorporates clarifications about configuration file handling (fail-fast on missing) and restart requirements for config updates.

## Summary

Extend Cedar Policy Engine to support multi-valued authorization decisions beyond binary permit/forbid. The system will support configurable decision types (allow, deny, alert, validate, audit) where a single authorization evaluation can return multiple concurrent decision types. This enables production authorization systems to trigger side effects (alerting, auditing, validation) through policy-driven decisions while maintaining 100% backward compatibility with existing Cedar policies. The implementation follows a hybrid configuration + core extension approach with YAML-based decision type configuration, Rust type-safe registry, extended policy syntax, and strict operational contracts (fail-fast on config errors, restart required for updates).

## Technical Context

**Language/Version**: Rust 1.75+ (Cedar Policy Engine is implemented in Rust)
**Primary Dependencies**:
- Cedar Policy Engine core (`cedar-policy-core`)
- YAML parsing library (`serde_yaml` for type-safe deserialization)
- Cedar parser infrastructure (LALRPOP for grammar extension)
- Cedar validator and schema system
**Storage**: Configuration files (YAML for decision types), no runtime persistence required
**Testing**: `cargo test` (Rust native testing), benchmarking suite with `criterion` for performance validation
**Target Platform**: Cross-platform library (Linux, macOS, Windows) with WASM support
**Project Type**: Library extension (extending Cedar Policy Engine core)
**Performance Goals**:
- Binary decisions: <5% overhead vs current Cedar
- Multi-valued (2-5 decisions): <15% overhead
- 10,000+ authorization requests per second
**Constraints**:
- 100% backward compatibility with existing Cedar policies and APIs
- Type-safe Rust implementation with compile-time guarantees
- Zero runtime dependencies beyond Cedar core
- Thread-safe for concurrent evaluation
- **Configuration must exist at startup** (fail-fast, no graceful fallback)
- **Configuration updates require restart** (no hot-reloading)
**Scale/Scope**:
- Support 5+ custom decision types concurrently
- Handle up to 10 decision types per configuration
- Minimal memory overhead (registry loaded once at initialization)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Status**: ⚠️ Constitution template not yet populated - no project-specific principles defined

Since the constitution file contains only template placeholders, no specific gate violations can be identified. Once the project constitution is established, the following areas should be reviewed:

- **Backward Compatibility**: This feature maintains 100% compatibility with existing Cedar APIs (no breaking changes)
- **Testing Strategy**: TDD approach with comprehensive unit, integration, and performance tests
- **Simplicity**: Hybrid configuration approach balances flexibility with type safety
- **Documentation**: Contracts, data models, and quickstart guide will be generated in Phase 1
- **Operational Clarity**: Explicit fail-fast behavior for config errors and restart requirements documented

**Post-Phase 1 Re-check**: Will validate that design artifacts comply with any established constitution principles.

## Project Structure

### Documentation (this feature)

```text
specs/001-cedar-multi-decisions/
├── spec.md              # Feature specification (completed, includes clarifications)
├── plan.md              # This file (implementation plan)
├── research.md          # Phase 0: Research findings and technical decisions
├── data-model.md        # Phase 1: Entity models and type definitions
├── quickstart.md        # Phase 1: Developer quick start guide
├── contracts/           # Phase 1: API contracts and policy syntax
│   ├── policy-syntax.md      # Extended Cedar policy grammar
│   ├── api-contracts.md      # Authorization API signatures
│   └── config-schema.yaml    # Decision type configuration schema
├── checklists/          # Quality validation checklists
│   └── requirements.md  # Specification quality checklist (completed)
└── tasks.md             # Phase 2: Implementation tasks (/speckit.tasks - NOT created yet)
```

### Source Code (Cedar Policy Engine Extension)

```text
cedar-policy-core/src/
├── entities/
│   ├── decision_registry.rs      # NEW: Decision type registry and metadata
│   └── tests/
│       └── decision_registry_tests.rs
├── ast/
│   ├── policy.rs                 # MODIFY: Effect enum with Custom variant
│   └── tests/
│       └── extended_effect_tests.rs
├── parser/
│   ├── grammar.lalrpop           # MODIFY: Add effect(name) syntax
│   ├── text_to_cst.rs           # MODIFY: Handle new token sequences
│   ├── cst_to_ast.rs            # MODIFY: Validate against registry
│   └── tests/
│       └── extended_effect_tests.rs
├── evaluator/
│   ├── evaluator.rs             # MODIFY: Multi-decision evaluation
│   ├── decision_set.rs          # NEW: DecisionSet type
│   └── tests/
│       └── multi_decision_tests.rs
├── authorizer/
│   ├── mod.rs                   # MODIFY: Extended API with decisions()
│   └── tests/
│       └── multi_decision_tests.rs
└── config/
    ├── decision_config.rs       # NEW: Configuration loader with fail-fast validation
    └── tests/
        └── config_tests.rs      # Including missing file error tests

cedar-policy-validator/src/
├── validator.rs                 # MODIFY: Validate custom effects
└── tests/
    └── custom_effect_validation_tests.rs

benches/
└── multi_decision_bench.rs      # NEW: Performance benchmarks

examples/
├── decision_config.yaml         # Example configuration
├── basic_multi_decision.cedar   # Example policies
└── integration_example.rs       # End-to-end usage example

tests/
├── integration/
│   ├── backward_compat_test.rs  # Legacy policy compatibility
│   ├── multi_decision_e2e.rs    # Full workflow tests
│   ├── config_loading_test.rs   # Configuration validation
│   └── config_error_test.rs     # NEW: Missing config file behavior
└── contract/
    ├── policy_syntax_test.rs    # Parser contract tests
    └── api_contract_test.rs     # API signature tests
```

**Structure Decision**: This is a library extension to the existing Cedar Policy Engine. We're modifying core Cedar files (Effect enum, parser, evaluator, authorizer) and adding new modules (decision registry, decision set, configuration). The structure follows Cedar's existing organization with clear separation between core types (`ast/`), parsing (`parser/`), evaluation (`evaluator/`), and public API (`authorizer/`). New functionality is isolated in the `entities/decision_registry.rs` module and `config/` directory to minimize impact on existing Cedar code. The config module implements strict validation with fail-fast semantics as specified in clarifications.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

**Status**: N/A - No constitution violations identified. Constitution template not yet populated with project-specific principles.

Once constitution is established, any complexity additions (new abstractions, pattern introductions, etc.) will be documented here with justification.

---

## Phase 0: Research (To Be Generated)

**Status**: ⏳ Pending

Research will resolve the following technical decisions:

1. **Cedar Architecture Integration**: How to extend Cedar's evaluation pipeline
2. **Configuration Library Selection**: Which YAML parser (serde_yaml vs alternatives)
3. **Decision Type Identifier Design**: Performance vs type-safety tradeoffs
4. **Parser Extension Strategy**: LALRPOP grammar modification approach
5. **Combination Rules Logic**: Precedence and conflict resolution algorithms
6. **Performance Optimization**: Lazy evaluation and fast-path strategies
7. **Backward Compatibility**: Dual API implementation patterns
8. **Thread Safety**: Immutable registry with Arc sharing
9. **Configuration Error Handling**: Fail-fast implementation (incorporates clarification)
10. **Operational Model**: Restart-based updates (incorporates clarification)

**Output**: `research.md` with all technical decisions documented

---

## Phase 1: Design & Contracts (To Be Generated)

**Status**: ⏳ Pending (requires Phase 0 completion)

Will generate:

1. **Data Model** (`data-model.md`):
   - DecisionTypeId, DecisionTypeMetadata, DecisionTypeRegistry
   - DecisionSet, Effect (extended), CombinationRule
   - DecisionConfig with strict validation
   - MultiResponse
   - Complete entity relationships and data flow

2. **Contracts** (`contracts/`):
   - **Policy Syntax** (`policy-syntax.md`): Extended Cedar grammar with effect(name) syntax
   - **API Contracts** (`api-contracts.md`): Public API signatures including config error handling
   - **Configuration Schema** (`config-schema.yaml`): YAML structure with validation rules

3. **Developer Guide** (`quickstart.md`):
   - 5-minute quick start
   - Configuration file setup (required)
   - Development workflow
   - Testing strategy
   - Deployment practices (restart requirements)

4. **Agent Context Update**: Update `CLAUDE.md` with technology stack

**Outputs**: data-model.md, contracts/, quickstart.md, updated CLAUDE.md

---

## Phase 2: Implementation Tasks (Next Step After Planning)

**Status**: ⏳ Ready for `/speckit.tasks`

After Phase 0-1 complete, run `/speckit.tasks` to generate detailed implementation task breakdown.

---

## Operational Requirements (From Clarifications)

### Configuration File Handling

**Requirement**: System MUST fail at startup if configuration file is missing or inaccessible (FR-011, A-002, C-006)

**Implementation Impact**:
- `DecisionConfig::from_file()` must return clear error with file path
- `DecisionTypeRegistry::from_config()` fails if config invalid
- `Authorizer::new()` propagates config errors to caller
- No fallback to default registry when explicit config path provided
- Error messages must include: expected file path, error cause, remedy suggestion

**Testing Requirements**:
- Test missing config file → startup error
- Test unreadable config file → startup error
- Test invalid YAML → parse error with location
- Test invalid decision types → validation error with details

### Configuration Update Strategy

**Requirement**: Configuration changes require application restart (A-002, C-006)

**Implementation Impact**:
- Registry immutable after initialization (no setter methods)
- No hot-reload API or file watcher
- Documentation must explicitly state restart requirement
- Deployment guides must cover rolling restart procedures

**Operational Considerations**:
- Standard practice for library-level config
- Compatible with load-balanced deployments
- Configuration changes are infrequent (design-time decisions)
- Restart ensures consistency across all evaluations

---

## Key Technical Decisions Summary

| Aspect | Decision | Incorporates Clarification |
|--------|----------|---------------------------|
| Architecture | Hybrid extension (preserve + extend) | - |
| Configuration | YAML with serde_yaml | ✅ Fail-fast on missing |
| Config Updates | Restart required (immutable registry) | ✅ No hot-reload |
| Performance | Lazy evaluation, fast paths | - |
| Parser | `effect(name)` syntax | - |
| API | Dual (legacy + extended) | - |
| Concurrency | Immutable Arc<Registry> | ✅ Loaded once at startup |
| Error Handling | Fail-fast with actionable messages | ✅ Config errors fatal |

---

## Success Criteria Alignment

The implementation plan directly addresses all success criteria from spec.md:

- **SC-001**: Performance targets (<15% multi-valued) → Phase 0 research, benchmarking strategy
- **SC-002**: 100% backward compatibility → Dual API, legacy tests
- **SC-003**: Identical legacy results → Conversion layer, comparison tests
- **SC-004**: Clear validation errors → Config module design, error types
- **SC-005**: Config-driven deployment → ✅ **Fail-fast ensures correct config**
- **SC-006**: 10k+ req/s → Performance goals, optimization strategies
- **SC-007**: Query API → DecisionSet methods in contracts
- **SC-008**: Deterministic resolution → Precedence algorithm in research
- **SC-009**: Init-time error detection → ✅ **Startup validation, no runtime surprises**
- **SC-010**: Incremental adoption → Dual API design

---

## Next Steps

1. ✅ Planning initiated
2. ⏳ **Generate Phase 0 research.md** (next action in this command)
3. ⏳ **Generate Phase 1 artifacts** (data-model, contracts, quickstart)
4. ⏳ **Update agent context**
5. ⏳ Report completion
6. Then user runs: `/speckit.tasks` to generate implementation tasks

---

## Notes

This plan incorporates operational clarifications from `/speckit.clarify` session:
- Configuration file presence is mandatory (no fallback)
- Configuration updates require restart (no hot-reload complexity)
- These decisions simplify implementation, testing, and operational procedures
- Fail-fast approach provides clear feedback for deployment errors

The strict operational model trades some runtime flexibility for:
- Simpler implementation (no dynamic config reload complexity)
- Clearer operational contracts (restart = clean state)
- Reduced error surface (no partial config update states)
- Better testability (deterministic initialization)

---

## Phase 0: Research (Completed)

**Status**: ✅ Complete

All technical unknowns have been resolved through research and analysis. **This revision incorporates operational clarifications about configuration file handling (fail-fast on missing) and restart requirements for updates.**

Research topics addressed:
1. Cedar Architecture Integration - Hybrid extension approach
2. Configuration Library Selection - serde_yaml with excellent error reporting  
3. Decision Type Identifier Design - Newtype DecisionTypeId(u32)
4. Parser Extension Strategy - LALRPOP with effect(name) syntax
5. Combination Rules Logic - Precedence + exclusivity
6. Performance Optimization - Lazy evaluation with fast paths
7. Backward Compatibility - Dual API (legacy + extended)
8. Thread Safety - Immutable Arc<Registry>
9. **Configuration Error Handling** - Fail-fast on missing/invalid (clarification)
10. **Operational Model** - Restart-based updates (clarification)

**Artifacts**: [`research.md`](./research.md)

---

## Phase 1: Design & Contracts (Completed)

**Status**: ✅ Complete

All design artifacts have been generated, incorporating operational clarifications:

1. **Data Model** ([`data-model.md`](./data-model.md)):
   - 8 core entities with complete specifications
   - **Fail-fast config loading** with clear error messages
   - **Immutable registry** (no hot-reload)
   - Complete entity relationships and data flow
   - Performance characteristics and validation rules

2. **Contracts** ([`contracts/`](./contracts/)):
   - **Policy Syntax** ([`policy-syntax.md`](./contracts/policy-syntax.md)): Extended Cedar grammar
   - **API Contracts** ([`api-contracts.md`](./contracts/api-contracts.md)): Public APIs with config error handling
   - **Configuration Schema** ([`config-schema.yaml`](./contracts/config-schema.yaml)): YAML structure with fail-fast requirements
   - **README** ([`README.md`](./contracts/README.md)): Contract principles

3. **Developer Guide** ([`quickstart.md`](./quickstart.md)):
   - 5-minute quick start with config requirement prominently featured
   - Development workflow
   - **Deployment practices** (restart requirements)
   - Testing strategy
   - **Troubleshooting** (config errors, restart procedures)

4. **Agent Context Update**:
   - Updated `CLAUDE.md` with Rust 1.75+, YAML config, library extension type

---

## Phase 2: Implementation Tasks (Next Step)

**Status**: ⏳ Ready for `/speckit.tasks`

Run `/speckit.tasks` to generate detailed implementation task breakdown based on these design artifacts.

After tasks generated, use `/speckit.implement` to execute the implementation plan.

---

## Planning Summary

**Status**: ✅ Complete (Phases 0-1)

**Clarifications Incorporated**:
- ✅ Configuration file MUST exist at startup (fail-fast, no fallback)
- ✅ Configuration updates require restart (immutable registry, no hot-reload)
- ✅ Clear error messages with actionable remediation steps
- ✅ Operational contracts explicitly documented

**Artifacts Generated**:
- ✅ plan.md - Implementation plan (this file)
- ✅ research.md - Technical decisions (10 topics, incorporates clarifications)
- ✅ data-model.md - Entity definitions (8 entities, fail-fast behavior)
- ✅ contracts/policy-syntax.md - Grammar specification
- ✅ contracts/api-contracts.md - Public API definitions
- ✅ contracts/config-schema.yaml - Configuration documentation
- ✅ contracts/README.md - Contract principles
- ✅ quickstart.md - Developer guide (emphasizes config requirements)
- ✅ CLAUDE.md - Updated agent context

**Key Design Decisions**:
| Aspect | Decision | Clarification Impact |
|--------|----------|---------------------|
| Architecture | Hybrid extension | - |
| Configuration | YAML with serde_yaml | ✅ Excellent error reporting |
| Config Errors | Fail-fast on missing/invalid | ✅ From clarification Q1 |
| Config Updates | Restart required | ✅ From clarification Q2 |
| Identifiers | DecisionTypeId(u32) | - |
| Parser | effect(name) syntax | - |
| Performance | <5% binary, <15% multi | - |
| Compatibility | Dual API | - |
| Concurrency | Immutable Arc<Registry> | ✅ Aligns with restart model |

**Next Commands**:
1. `/speckit.tasks` - Generate implementation tasks
2. `/speckit.implement` - Execute implementation
3. `/speckit.analyze` - Post-implementation quality analysis

**Implementation Readiness**: ✅ Yes - All design complete, clarifications integrated, no blockers
