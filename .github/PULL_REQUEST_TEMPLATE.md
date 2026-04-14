## Summary

Describe the change and the motivation behind it.

## Related Issues

Closes #

## Type of Change

- [ ] Bug fix
- [ ] New feature
- [ ] Refactor
- [ ] Documentation update
- [ ] Tests only
- [ ] CI or tooling change

## What Changed

- 

## Validation

List the commands you ran and include notable output when relevant.

```bash
pre-commit run --all-files
bash tools/ci/with_llvm_env.sh cargo check --workspace --all-targets
bash tools/ci/with_llvm_env.sh cargo test --workspace
cargo fmt --all -- --check
bash tools/ci/with_llvm_env.sh cargo clippy --workspace --all-targets -- -D warnings
```

## Checklist

- [ ] I have read `CONTRIBUTING.md`.
- [ ] I added or updated tests for behavior changes.
- [ ] I updated docs/examples/changelog where needed.
- [ ] I kept this PR focused and free of unrelated changes.
- [ ] I verified the change on my target platform.
