# Policy Syntax Contract

**Feature**: Cedar Multi-Valued Authorization Decisions  
**Date**: 2026-03-18

## Grammar Extension

### Extended EBNF
```ebnf
Effect ::= 'permit' | 'forbid' | 'effect' '(' IDENT ')'
IDENT  ::= [a-z][a-z0-9_]*
```

### LALRPOP Implementation
```lalrpop
Effect: Effect = {
    "permit" => Effect::Permit,
    "forbid" => Effect::Forbid,
    "effect" "(" <name:Ident> ")" => Effect::CustomName(name),
};

Ident: String = r"[a-z][a-z0-9_]*" => <>.to_string();
```

## Syntax Examples

### Legacy (Unchanged)
```cedar
permit(principal, action, resource);
forbid(principal, action, resource) when { resource.private };
```

### Extended
```cedar
effect(alert)(principal, action, resource) when { resource.sensitive };
effect(validate)(principal, action == Action::"transfer", resource)
    when { resource.amount > 10000 };
effect(audit)(principal, action, resource) when { resource.contains_pii };
```

## Validation

**Parse Time**: Identifier format validation  
**AST Time**: Registry lookup validation (fail if unknown decision type)

### Error Example
```
Error: Unknown decision type 'alrt'
  --> policy.cedar:5:8
   |
 5 | effect(alrt)(principal, action, resource);
   |        ^^^^ not defined in configuration
   |
   = available types: allow, deny, alert, validate, audit
   = help: did you mean 'alert'?
```

## Backward Compatibility

✅ All existing `permit`/`forbid` policies parse identically  
✅ No keyword conflicts  
✅ Parser fallback with clear errors on unknown syntax
