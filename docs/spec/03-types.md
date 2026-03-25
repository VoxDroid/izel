# 3. Types And Kinds (Normative)

## Primitive Types
v1.0 primitive kinds include integer, floating-point, bool, str, void, and pointer forms.

## Compound Types
Supported compound forms include:
- optionals,
- pointers,
- function types,
- user-defined `shape` and `scroll` types,
- witness types.

## Inference And Unification
- Type inference is Hindley-Milner style with unification.
- Ambiguous or inconsistent constraints MUST produce diagnostics.

## Coherence
- Weave coherence and orphan restrictions are enforced.
- Conflicting implementations MUST be rejected.
