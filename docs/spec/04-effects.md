# 4. Effect System (Normative)

## Effect Declarations
- `forge` signatures MAY declare effects (for example `!io`).
- Pure functions are effect-free unless explicitly declared otherwise.

## Propagation
- Calls propagate effect obligations to callers.
- Missing effect propagation is a type error.

## Boundaries
- Effect boundaries MAY mask contained effects when declared.
- Boundary usage is validated at type-check time.

## Conformance
Implementations MUST reject effect-unsafe calls and MUST preserve sound effect inference.
