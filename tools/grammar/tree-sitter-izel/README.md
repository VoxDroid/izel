# tree-sitter-izel

Tree-sitter grammar scaffold for Izel syntax highlighting and editor integration.

## Scope

This grammar targets the core declaration and statement surface used by the compiler roadmap:

- `forge`, `shape`, `scroll`, `ward`, `draw`
- blocks, conditionals (`given`/`else`), and `while`
- path expressions (`std::io::println`) and calls
- core literals and comments

## Usage

```bash
cd tools/grammar/tree-sitter-izel
npm install
npm run generate
npm run test
```

## Notes

This is the initial broad-editor-support grammar milestone. The rule set is intentionally conservative and will be expanded as language coverage grows.
