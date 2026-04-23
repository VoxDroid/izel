# 9. Sample Applications

This chapter introduces the `sample_applications/` suite.

## What You Get

The suite contains 100 numbered applications (`001` to `100`) that model practical domains:

- calculators and forecasting workflows,
- operations and planning use cases,
- monitoring and risk-style reports,
- terminal GUI-style dashboards via `draw std/tui`.

## Core Workflow

Use compile mode as the primary validation path:

```bash
bash tools/ci/with_llvm_env.sh cargo run -p izel_driver -- sample_applications/001_budget_forecast_calculator.iz
```

This runs the end-to-end compiler path and emits LLVM IR.

## Validate The Full Suite

```bash
for f in sample_applications/[0-9][0-9][0-9]_*.iz; do
  bash tools/ci/with_llvm_env.sh cargo run -p izel_driver -- "$f" || break
done
```

## Tutorial And Categorized Index

- `sample_applications/README.md`
- `sample_applications/TUTORIAL.md`

## Runtime Expansion Note

Runtime control-flow execution is under active expansion in the compiler/runtime pipeline. `while`
loops are runtime-validated, while broader `loop`/`each` lowering support continues to grow. The
sample suite is designed to remain compile-first reliable while runtime support expands.
