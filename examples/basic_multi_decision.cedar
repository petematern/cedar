// Cedar Multi-Valued Decision Example Policies
// These demonstrate the extended effect(name) syntax for custom decision types

// =============================================================================
// Example 1: Security Monitoring with Concurrent Decisions
// Grant access while simultaneously triggering security alerts
// =============================================================================

// Basic access control for department members
permit(
    principal,
    action == Action::"read",
    resource
) when {
    principal.department == resource.department
};

// Alert when sensitive resources are accessed (even if allowed)
effect(alert)(
    principal,
    action == Action::"read",
    resource
) when {
    resource.classification == "sensitive"
};

// =============================================================================
// Example 2: Conditional Additional Verification
// Financial services - require 2FA for high-value transactions
// =============================================================================

// Allow transfers for finance staff
permit(
    principal,
    action == Action::"transfer",
    resource
) when {
    principal.role == "finance_staff"
};

// Require additional validation for large amounts
effect(validate)(
    principal,
    action == Action::"transfer",
    resource
) when {
    resource.amount > 10000
};

// =============================================================================
// Example 3: Comprehensive Audit Trail
// Compliance - log all PII access attempts
// =============================================================================

// Audit all access to resources containing PII (regardless of allow/deny)
effect(audit)(
    principal,
    action,
    resource
) when {
    resource.contains_pii == true
};

// =============================================================================
// Example 4: Combining Multiple Decision Types
// Complex scenario with allow + alert + validate + audit
// =============================================================================

// Grant access to executives
permit(
    principal,
    action == Action::"view",
    resource
) when {
    principal.role == "executive"
};

// Alert on executive access to confidential data
effect(alert)(
    principal,
    action == Action::"view",
    resource
) when {
    principal.role == "executive" &&
    resource.classification == "confidential"
};

// Require validation for access outside business hours
effect(validate)(
    principal,
    action == Action::"view",
    resource
) when {
    context.hour < 6 || context.hour > 18
};

// Audit all confidential data access
effect(audit)(
    principal,
    action,
    resource
) when {
    resource.classification == "confidential"
};

// =============================================================================
// Example 5: Backward Compatibility
// Legacy permit/forbid syntax continues to work unchanged
// =============================================================================

// Standard forbid rule (maps to "deny" decision type)
forbid(
    principal,
    action,
    resource
) when {
    resource.archived == true
};

// Standard permit rule (maps to "allow" decision type)
permit(
    principal,
    action == Action::"delete",
    resource
) when {
    principal.role == "admin"
};
