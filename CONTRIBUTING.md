# Contributing to Izel

Thanks for helping improve Izel. Contributions are welcome across compiler internals, language tooling, docs, tests, examples, and developer experience.

## Ways to Contribute

- Report bugs and regressions.
- Propose language and tooling improvements.
- Improve compiler diagnostics and error messages.
- Add or refine tests under `tests/` and crate-level test suites.
- Improve docs under `docs/` and examples under `examples/`.

## Before You Start

1. Search existing issues and pull requests to avoid duplicate work.
2. If your change is large, open an issue first to align on direction.
3. Keep changes scoped and reviewable.

## Development Setup

Follow the repository setup in `README.md`. In short:

1. Install Rust using the pinned toolchain in `rust-toolchain.toml`.
2. Install LLVM 17, `clang`, `lld`, and `cmake`.
3. Validate dependencies:

```bash
bash tools/ci/check_system_deps.sh
```

4. Build the workspace:

```bash
bash tools/ci/with_llvm_env.sh cargo build --workspace
```

## Development Workflow

1. Fork the repository and create a feature branch.
2. Make focused commits with clear messages.
3. Add or update tests for behavior changes.
4. Run formatting, linting, and tests locally.
5. Open a pull request using the PR template.

## Local Validation Checklist

Run these commands before opening a pull request:

```bash
pre-commit run --all-files
bash tools/ci/with_llvm_env.sh cargo check --workspace --all-targets
bash tools/ci/with_llvm_env.sh cargo test --workspace
cargo fmt --all -- --check
bash tools/ci/with_llvm_env.sh cargo clippy --workspace --all-targets -- -D warnings
```

For faster iteration, run targeted checks while developing, then run the full checklist before submitting.

## Pull Request Expectations

- Explain what changed and why.
- Reference related issues (for example, `Closes #123`).
- Include test coverage for new logic or bug fixes.
- Update docs/examples when user-facing behavior changes.
- Keep unrelated refactors out of the same pull request.

## Commit Message Guidance

Use short, imperative summaries.

Examples:

- `parser: reject duplicate witness declarations`
- `lexer: preserve span for escaped newline`
- `docs: clarify ownership chapter terminology`

## Reporting Bugs

Use the issue templates in `.github/ISSUE_TEMPLATE/` for reproducible reports.

## Security Issues

Do not open public issues for vulnerabilities. See `SECURITY.md` for responsible disclosure steps.

## License

By contributing, you agree that your contributions are licensed under the repository's MIT license.
