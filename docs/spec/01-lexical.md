# 1. Lexical Structure (Normative)

## Source Encoding
- Source files MUST be UTF-8.
- File extensions are `.iz` for source and `.izm` for module bundles.

## Comments
- Line comment: `// ...`.
- Block comment: `/~ ... ~/`.
- Doc comments are `///` and `//!`.

## Keywords And Sigils
- Language keywords and reserved words are defined in `docs/project_overview.md` section 4.
- Sigils such as `~`, `!`, `@`, and `|>` are lexical tokens with fixed meaning.

## Literals
- Integer, float, string, bool, and nil literals are part of core lexical syntax.
- Numeric separators (`_`) are permitted where grammar allows.

## Identifiers
- Identifiers MUST be valid Unicode identifier sequences.
- Keywords are not valid identifiers unless escaped by future language rules.
