# 9. Duality Types (Normative)

## Dual Declarations
`dual` declarations define paired encode/decode behavior over one structural source.

## Elaboration Rules
- Missing direction may be synthesized when derivation is valid.
- Unsupported derivation MUST produce diagnostics.

## Round-Trip Law
Implementations MUST enforce or verify round-trip compatibility for dual representations.

## Testing
Generated round-trip tests are part of conformance behavior for effectful dual declarations.
