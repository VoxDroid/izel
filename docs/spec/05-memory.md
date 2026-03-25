# 5. Memory And Ownership (Normative)

## Ownership
- Values have a single owner unless borrowed.
- Moves invalidate prior owners.

## Borrowing
- Mutable and immutable borrow rules are enforced statically.
- Conflicting borrows MUST be rejected.

## Lifetimes
- Region/lifetime constraints are inferred where possible.
- Escapes beyond valid region MUST produce diagnostics.

## Raw Escape Hatches
- `raw` is the explicit unsafe boundary.
- Safety guarantees outside `raw` remain enforced.
