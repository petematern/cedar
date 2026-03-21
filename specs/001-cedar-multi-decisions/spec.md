# Feature Specification: Cedar Multi-Valued Authorization Decisions

**Feature Branch**: `001-cedar-multi-decisions`
**Created**: 2026-03-18
**Status**: Draft
**Input**: User description: "Create a feature specification for extending Cedar Policy Engine to support multi-valued authorization decisions beyond binary permit/forbid. The feature should support configurable decision types (allow, deny, alert, validate, audit), multi-valued results where a single authorization can yield multiple concurrent decisions, combination rules for how decisions interact, and full backward compatibility with existing Cedar policies. This is for a production authorization system where access decisions need to trigger side effects like alerting, auditing, and validation."

## Clarifications

### Session 2026-03-18

- Q: What happens when the configuration file is missing or inaccessible at system initialization? → A: Fail startup with error (system refuses to initialize without valid configuration file)
- Q: How do operators update the configuration in production (e.g., adding new decision type)? → A: Restart required (configuration changes require restarting the application/service)

## User Scenarios & Testing

### User Story 1 - Security Monitoring with Concurrent Decisions (Priority: P1)

A security team needs to grant access to sensitive resources while simultaneously triggering security alerts for monitoring purposes. When users access classified data, the system should both allow the access (if authorized) AND generate an alert for the security operations center.

**Why this priority**: This is the core value proposition - enabling authorization decisions to carry multiple concurrent outcomes. Without this, teams must choose between access control and monitoring, or build separate monitoring systems.

**Independent Test**: Can be fully tested by defining policies with custom decision types (e.g., "alert"), evaluating an authorization request, and verifying that multiple decision outcomes are returned simultaneously (e.g., both "allow" and "alert").

**Acceptance Scenarios**:

1. **Given** a policy that grants access to sensitive resources AND a policy that triggers alerts for the same resources, **When** an authorized user requests access to a sensitive resource, **Then** the authorization result includes both "allow" and "alert" decisions
2. **Given** multiple concurrent policies with different decision types (allow, alert, validate), **When** an authorization request matches multiple policies, **Then** all applicable decision types are returned in the result
3. **Given** a policy configuration with combination rules, **When** decisions need to be combined, **Then** the rules are applied correctly and the result reflects the combined decision set

---

### User Story 2 - Conditional Additional Verification (Priority: P2)

A financial services application needs to allow transactions for authorized users but require additional verification (e.g., two-factor authentication) when transaction amounts exceed a certain threshold. The system should return both "allow" and "validate" decisions so the application can prompt for additional verification before completing the transaction.

**Why this priority**: Enables risk-based authorization where decisions can carry additional requirements. This is critical for compliance and security in high-stakes domains like finance and healthcare.

**Independent Test**: Can be tested by creating policies that return "validate" decisions based on resource attributes (e.g., transaction amount), and verifying that the application receives both authorization and validation signals in a single request.

**Acceptance Scenarios**:

1. **Given** a policy that allows transfers AND a policy that requires validation for amounts over $10,000, **When** a user initiates a $15,000 transfer, **Then** the result includes both "allow" and "validate" decisions
2. **Given** a policy that requires validation, **When** the authorization is evaluated, **Then** the application can distinguish between "allow" (proceed normally) and "allow + validate" (proceed with additional verification)

---

### User Story 3 - Comprehensive Audit Trail (Priority: P2)

A compliance team needs to maintain an audit log of all access attempts to resources containing personally identifiable information (PII), regardless of whether access is granted or denied. The system should return "audit" decisions that trigger logging side effects.

**Why this priority**: Essential for regulatory compliance (GDPR, HIPAA, SOC2) where audit trails are legally required. This enables policy-driven audit decisions rather than application-level logging.

**Independent Test**: Can be tested by defining audit policies that match specific resource types, evaluating authorization requests against those resources, and verifying that "audit" decisions are returned alongside primary authorization decisions.

**Acceptance Scenarios**:

1. **Given** a policy that triggers audits for PII resources, **When** any user attempts to access a PII resource (whether permitted or forbidden), **Then** the result includes an "audit" decision
2. **Given** audit policies for specific resource types, **When** authorization requests are evaluated, **Then** audit decisions are returned independently of allow/deny outcomes

---

### User Story 4 - Configurable Decision Types (Priority: P2)

An authorization system operator needs to define custom decision types beyond the default allow/deny to match their organization's specific authorization requirements. They should be able to configure decision types, their precedence, and how they combine with each other through a configuration file.

**Why this priority**: Provides flexibility for different organizational needs without requiring code changes. Different industries and use cases require different authorization semantics.

**Independent Test**: Can be tested by defining a configuration file with custom decision types and their properties, loading it into the system, and verifying that policies can use those decision types and that combination rules are applied correctly.

**Acceptance Scenarios**:

1. **Given** a configuration file defining custom decision types (name, precedence, combinability), **When** the system initializes, **Then** policies can reference those decision types
2. **Given** decision types with precedence levels, **When** multiple policies match with different decision types, **Then** conflicts are resolved according to precedence rules
3. **Given** decision types marked as "exclusive", **When** an exclusive decision is present, **Then** it excludes other incompatible decisions as configured

---

### User Story 5 - Backward Compatibility with Legacy Policies (Priority: P1)

Existing Cedar users with binary permit/forbid policies need to adopt the multi-valued decision system without breaking their current authorization logic. Legacy policies and API calls should continue to work exactly as before, with the option to gradually adopt multi-valued decisions.

**Why this priority**: Critical for adoption - breaking existing systems is unacceptable. Users must be able to migrate incrementally without a "big bang" cutover.

**Independent Test**: Can be tested by running existing Cedar policies and API calls against the extended system and verifying identical behavior to the original Cedar implementation. Legacy API should return binary Allow/Deny as before.

**Acceptance Scenarios**:

1. **Given** existing Cedar policies using `permit` and `forbid` effects, **When** those policies are evaluated in the extended system, **Then** they produce identical authorization results to the original Cedar
2. **Given** an application using the legacy binary authorization API, **When** it evaluates requests, **Then** it receives simple Allow/Deny responses without any multi-valued decision information
3. **Given** a mix of legacy and extended policies in the same policy set, **When** authorization is evaluated, **Then** both types of policies work together correctly
4. **Given** legacy policies, **When** they are parsed and validated, **Then** the `permit` keyword maps to "allow" decision type and `forbid` maps to "deny" decision type seamlessly

---

### Edge Cases

- What happens when conflicting decision types are returned (e.g., both "allow" and "deny")? System should apply configured precedence rules and conflict resolution strategy.
- How does the system handle unknown decision type names in policies? System should reject policies with undefined decision types during validation phase.
- What happens when a decision type is removed from configuration but existing policies reference it? System should fail validation with clear error messages identifying affected policies.
- How does the system behave when no policies match? Should return default "deny" decision, consistent with Cedar's default-deny semantics.
- What happens when combination rules conflict or create circular dependencies? System should detect and reject invalid configurations during initialization.
- How does the system handle concurrent modifications to decision type configuration? Configuration is loaded at initialization time and immutable during runtime.
- What happens when configuration file is missing or inaccessible at startup? System initialization fails with clear error indicating the required configuration file path and that it must exist before the system can start.
- How do operators deploy configuration updates to add or modify decision types? Configuration changes require an application restart. The system does not support hot-reloading or in-place configuration updates.

## Requirements

### Functional Requirements

- **FR-001**: System MUST support defining custom authorization decision types beyond binary permit/forbid
- **FR-002**: System MUST allow configuration of decision types including name, precedence level, combinability rules, and exclusivity properties
- **FR-003**: System MUST support returning multiple concurrent decision types from a single authorization evaluation
- **FR-004**: System MUST provide a way to define combination rules that specify how different decision types interact when multiple policies match
- **FR-005**: System MUST support precedence-based conflict resolution when incompatible decisions are present
- **FR-006**: System MUST maintain 100% backward compatibility with existing Cedar permit/forbid syntax and binary authorization API
- **FR-007**: System MUST validate decision type names in policies against the configured decision type registry at parse time
- **FR-008**: System MUST support at least 5 custom decision types (allow, deny, alert, validate, audit) concurrently
- **FR-009**: System MUST provide extended authorization API that returns sets of decisions with metadata about which policies contributed to each decision
- **FR-010**: System MUST support legacy binary authorization API that converts multi-valued results to Allow/Deny for backward compatibility
- **FR-011**: System MUST load decision type configuration from external configuration files and MUST fail initialization with a clear error message if the configuration file is missing or inaccessible
- **FR-012**: System MUST reject policies that reference undefined decision types with clear error messages
- **FR-013**: System MUST detect and reject invalid combination rules (circular dependencies, conflicting rules) at configuration load time
- **FR-014**: System MUST support marking decision types as "exclusive" so they exclude other decisions when present
- **FR-015**: System MUST preserve all diagnostics and policy evaluation information when converting multi-valued results to binary results
- **FR-016**: System MUST support querying whether specific decision types are present in an authorization result
- **FR-017**: System MUST support retrieving all decision types from an authorization result
- **FR-018**: System MUST support identifying which policies contributed to each decision type in the result

### Key Entities

- **Decision Type**: Represents a custom authorization outcome (e.g., "allow", "deny", "alert", "validate", "audit"). Key attributes include unique name, precedence level (numeric), combinability flag (boolean indicating if it can coexist with other decisions), and exclusivity flag (boolean indicating if it excludes other decisions when present).

- **Decision Set**: The result of an authorization evaluation containing multiple concurrent decision types. Attributes include the set of decision types present, mapping of decision types to contributing policies, and reference to the decision type registry for lookups.

- **Decision Type Registry**: Central registry managing all configured decision types. Maintains mappings between decision type names and identifiers, precedence ordering, and combination rules.

- **Combination Rule**: Defines how decision types interact when multiple are present. Attributes include set of decision types the rule applies to, resolution strategy (merge, exclusive, etc.), and resulting decision set.

- **Authorization Request**: Input to the authorization evaluation containing principal, action, resource, and context. Same as standard Cedar requests.

- **Multi-Valued Response**: Extended authorization response containing the decision set (multiple concurrent decisions), diagnostics information, and policy evaluation details.

## Success Criteria

### Measurable Outcomes

- **SC-001**: System successfully evaluates authorization requests with 2-5 concurrent decision types with less than 15% performance overhead compared to binary decisions
- **SC-002**: 100% of existing Cedar policies continue to function without modification or behavior changes
- **SC-003**: Legacy binary authorization API returns identical results to original Cedar implementation for all existing policies
- **SC-004**: System correctly validates and rejects policies with undefined decision types with clear error messages indicating the specific undefined type and affected policy
- **SC-005**: Authorization operators can define and deploy new decision types through configuration changes without code modifications
- **SC-006**: System handles at least 10,000 authorization requests per second with multi-valued policies (within 5% of binary decision throughput)
- **SC-007**: Applications can query for specific decision types (e.g., "is alert present?") and retrieve all decisions in a single authorization call
- **SC-008**: Precedence rules and conflict resolution produce deterministic, predictable results across all scenarios
- **SC-009**: Configuration errors (invalid combination rules, undefined precedence) are detected at system initialization with actionable error messages
- **SC-010**: Applications can incrementally adopt multi-valued decisions - some parts use legacy API while others use extended API without conflicts

## Assumptions

- **A-001**: The Cedar policy engine is implemented in Rust and follows Cedar 3.x/4.x architecture (based on standard Cedar distribution)
- **A-002**: Decision type configuration is loaded once at system initialization and remains immutable during runtime (no hot-reloading required). System initialization fails if configuration file is missing or inaccessible. Configuration updates require application restart.
- **A-003**: Configuration format will be YAML for readability and ease of editing by operations teams
- **A-004**: Maximum of 10 custom decision types per configuration is sufficient for production use cases
- **A-005**: Decision type precedence is expressed as numeric values where higher numbers indicate higher priority
- **A-006**: Default conflict resolution strategy when not specified is precedence-based (highest precedence wins)
- **A-007**: Built-in decision types "allow" and "deny" are always present in the registry with default precedence (allow: 100, deny: 200)
- **A-008**: Extended policy syntax uses `effect(name)` grammar to distinguish from legacy `permit`/`forbid` keywords
- **A-009**: Performance target assumes 2-5 concurrent decision types; performance may degrade with more than 10 concurrent decisions
- **A-010**: Thread-safety is required for multi-threaded authorization evaluation scenarios

## Dependencies

- **D-001**: Cedar Policy Engine source code access for core type modifications
- **D-002**: YAML parsing library for configuration loading
- **D-003**: Existing Cedar parser infrastructure for grammar extension
- **D-004**: Cedar schema validation system for integration with custom decision types

## Constraints

- **C-001**: Solution must maintain API compatibility with existing Cedar deployments
- **C-002**: Performance overhead for binary decisions must be minimal (< 5%) to avoid impacting existing users
- **C-003**: Solution must work within Cedar's existing type system and evaluation model
- **C-004**: Configuration format must be human-readable and editable without specialized tools
- **C-005**: Error messages must be clear enough for non-developers to diagnose configuration issues
- **C-006**: Configuration changes require application restart; no hot-reloading support in initial implementation
