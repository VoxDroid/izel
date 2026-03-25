# 7. Temporal Contracts (Normative)

## Preconditions And Postconditions
- `@requires` expresses preconditions.
- `@ensures` expresses postconditions.

## Compile-Time Evaluation
- Contracts over compile-time-known values are validated statically.
- Violations MUST emit diagnostics.

## Runtime Instrumentation
- For dynamic values, implementations MAY emit runtime assertions.
- Contract instrumentation behavior is controlled by compiler flags.

## Invariants
`#[invariant]` constraints on stateful structures are part of the v1.0 contract model.
