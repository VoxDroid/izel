# 2. Syntax And Grammar (Normative)

## Parsing Model
- Parsing is deterministic and produces a concrete syntax tree (CST).
- CST lowering produces the abstract syntax tree (AST).

## Top-Level Items
The following top-level declarations are part of v1.0:
- `forge`, `shape`, `scroll`, `weave`, `impl`, `ward`, `draw`, `echo`, `bridge`, `raw` blocks.

## Expressions
v1.0 includes:
- literals and identifiers,
- unary and binary operators,
- function calls,
- `given`/`else`, `branch`, loops,
- blocks and trailing expressions,
- pipelines with `|>`.

## Attributes
- Bracket attributes `#[...]` are valid on supported declarations.
- Unsupported placement MUST produce a diagnostic.
