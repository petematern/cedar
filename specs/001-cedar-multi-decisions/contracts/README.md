# Contracts: Cedar Multi-Valued Decisions

This directory contains the formal interface contracts for the Cedar multi-valued decisions extension.

## Files

- **policy-syntax.md** - Extended Cedar policy grammar specification
- **api-contracts.md** - Public API function signatures and behavior contracts
- **config-schema.yaml** - Configuration file schema and examples

## Contract Principles

1. **Backward Compatibility**: All legacy Cedar syntax and APIs remain unchanged
2. **Fail-Fast Validation**: Configuration errors detected at startup (not runtime)
3. **Restart-Based Updates**: Configuration changes require application restart
4. **Clear Error Messages**: All failures include actionable remediation steps

## Clarifications Incorporated

These contracts incorporate operational clarifications:
- Configuration file MUST exist at startup (fail-fast on missing)
- Configuration updates require restart (immutable registry, no hot-reload)
- Error handling emphasizes early detection and clear feedback
