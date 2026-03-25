# 6. Witness Types (Normative)

## Model
Witness values encode compile-time proof obligations as types.

## Built-Ins
v1.0 includes built-in witnesses such as `NonZero`, `InBounds`, and `Sorted`.

## Construction Rules
- Witness construction is restricted to proof-valid paths or explicit `raw` contexts.
- Invalid witness construction MUST be rejected.

## Runtime Elision
Where witness evidence is present, redundant runtime assertions MAY be omitted.
