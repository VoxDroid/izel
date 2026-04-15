# Capability Matrix

This matrix links project-overview functionality to concrete applications.

| Project Overview Capability | Demonstrated In |
| --- | --- |
| Variables, mutability sigil, arithmetic, conditionals | 001-012, 041-050 |
| Loops (`while` / `loop`-style iteration patterns) | 001-020, 051-060 |
| Functions and explicit `give` returns | 001-100 |
| Contracts (`@requires`, `@ensures`) | 026-033, 091-100 |
| Effects (`!io`) and effect propagation | 034-041, 081-090 |
| Witness proofs (`@proof`, `Witness<T>`, `raw`) | 042-049, 061-070 |
| Memory zones (`zone`, allocator scope) | 050-057, 071-080 |
| Shapes and method-style modeling | 058-067 |
| Scroll enums and `branch` matching | 068-075 |
| Weaves / interface-style behavior | 076-083 |
| Generics and parametric helpers | 084-087 |
| Iterator protocol and pipelines (`|>`, `bind`) | 088-093 |
| Macros (`macro`, invocation) | 094-095 |
| Async syntax (`flow`, `tide`) | 096-097 |
| Duality types (`dual shape`) | 098 |
| Wards/modules (`ward`) and boundary organization | 099 |
| Stdin and file IO intrinsics (`read_stdin`, `read_file`, `write_file`) | 013, 083 |
| File utility intrinsics (`append_file`, `remove_file`, `file_exists`, `file_exists_bool`, `list_dir`, `io_last_status`) | 013 |
| Stdin numeric parsing intrinsics (`read_stdin_int`, `read_stdin_float`) | 083 |
| Runtime std surfaces (`std/io`, `std/mem`, `std/tui`) | 001-100 (varied) |

Notes:
- The suite is compile-first verified against the current toolchain state.
- Some advanced roadmap features are represented as syntax-and-structure demonstrations while
  runtime semantics continue to evolve.
