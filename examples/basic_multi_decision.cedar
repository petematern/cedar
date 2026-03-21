// Cedar Multi-Valued Decision Example Policies
// These demonstrate custom decision types using the same syntax as permit/forbid

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
alert(
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
validate(
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
audit(
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
alert(
    principal,
    action == Action::"view",
    resource
) when {
    principal.role == "executive" &&
    resource.classification == "confidential"
};

// Require validation for access outside business hours
validate(
    principal,
    action == Action::"view",
    resource
) when {
    context.hour < 6 || context.hour > 18
};

// Audit all confidential data access
audit(
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
