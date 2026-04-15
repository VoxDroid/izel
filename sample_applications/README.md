# Izel Sample Applications

This directory contains 100 uniquely implemented sample applications.

Each file is intentionally different in structure, domain logic, and language-feature usage.
Coverage spans core functionality from the project overview: bindings, control flow, contracts,
effects, witnesses, zones, shapes, scrolls, weaves, generics, iterators/pipelines, macros,
flow/tide async syntax, dual types, wards/modules, interactive stdin input, stdin numeric parsing,
file operations (write/append/read/remove/exists/listing),
and std runtime surfaces.

## Compile One Application

```bash
bash tools/ci/with_llvm_env.sh cargo run -p izel_driver -- sample_applications/013_customer_churn_monitor.iz
```

## Compile All 100 Applications

```bash
for f in sample_applications/[0-9][0-9][0-9]_*.iz; do
  echo "Checking $f"
  bash tools/ci/with_llvm_env.sh cargo run -p izel_driver -- "$f" || break
done
```

## Capability Matrix

See `sample_applications/CAPABILITY_MATRIX.md` for a feature-to-app mapping tied to
`docs/project_overview.md` sections.

## Application Index

| ID | Application | Feature Profile |
| --- | --- | --- |
| 001 | budget forecast calculator | forecast loop model |
| 002 | mortgage payment planner | optimization search |
| 003 | compound interest projection | rolling average simulator |
| 004 | salary tax estimator | guardrail scoring |
| 005 | subscription break even | terminal dashboard |
| 006 | freelance rate planner | contracts clamp |
| 007 | inventory reorder calculator | contracts progression |
| 008 | energy bill forecast | effects pipeline |
| 009 | fleet fuel cost planner | effects and stderr |
| 010 | trip budget optimizer | witness proof flow |
| 011 | sprint capacity planner | witness shape proof |
| 012 | release burndown tracker | zones allocator |
| 013 | customer churn monitor | memory intrinsic usage + file utility/status intrinsics |
| 014 | marketing roi estimator | shape and impl methods |
| 015 | startup runway analyzer | scroll and branch |
| 016 | cashflow guardrail | weave implementation |
| 017 | profit margin guard | generic helper |
| 018 | warehouse slot optimizer | custom iterator |
| 019 | staffing shift planner | pipeline and bind |
| 020 | invoice collection monitor | collections usage |
| 021 | hotel occupancy forecaster | macro expansion |
| 022 | restaurant demand planner | async flow tide |
| 023 | clinic queue balancer | duality shape |
| 024 | school tuition planner | ward module boundary |
| 025 | campus shuttle scheduler | capstone mixed features |
| 026 | event ticket revenue model | forecast loop model |
| 027 | farmland yield forecast | optimization search |
| 028 | irrigation cycle planner | rolling average simulator |
| 029 | greenhouse temperature model | guardrail scoring |
| 030 | solar output estimator | terminal dashboard |
| 031 | wind farm capacity planner | contracts clamp |
| 032 | water usage guard | contracts progression |
| 033 | terminal dashboard gui | effects pipeline |
| 034 | logistics hub dashboard | effects and stderr |
| 035 | factory floor dashboard | witness proof flow |
| 036 | service desk dashboard | witness shape proof |
| 037 | airport ops dashboard | zones allocator |
| 038 | hospital ops dashboard | memory intrinsic usage |
| 039 | data center dashboard | shape and impl methods |
| 040 | city traffic dashboard | scroll and branch |
| 041 | carbon emission tracker | weave implementation |
| 042 | power grid stability model | generic helper |
| 043 | battery cycle analyzer | custom iterator |
| 044 | packet loss simulator | pipeline and bind |
| 045 | latency budget calculator | collections usage |
| 046 | cache hit ratio planner | macro expansion |
| 047 | api throughput forecast | async flow tide |
| 048 | query cost estimator | duality shape |
| 049 | search relevance tuner | ward module boundary |
| 050 | ml training budgeter | capstone mixed features |
| 051 | game scoreboard engine | forecast loop model |
| 052 | tournament bracket planner | optimization search |
| 053 | loot drop balancer | rolling average simulator |
| 054 | quest reward calculator | guardrail scoring |
| 055 | racing lap predictor | terminal dashboard |
| 056 | city builder economy | contracts clamp |
| 057 | survival supply planner | contracts progression |
| 058 | strategy resource model | effects pipeline |
| 059 | card deck probability | effects and stderr |
| 060 | party damage simulator | witness proof flow |
| 061 | fraud signal monitor | witness shape proof |
| 062 | credit risk scanner | zones allocator |
| 063 | insurance premium model | memory intrinsic usage |
| 064 | claim triage router | shape and impl methods |
| 065 | compliance risk report | scroll and branch |
| 066 | audit sampling planner | weave implementation |
| 067 | portfolio rebalance assistant | generic helper |
| 068 | retirement income projection | custom iterator |
| 069 | crypto volatility guard | pipeline and bind |
| 070 | forex position sizer | collections usage |
| 071 | iot sensor health monitor | macro expansion |
| 072 | edge device battery guard | async flow tide |
| 073 | telemetry anomaly score | duality shape |
| 074 | factory maintenance cycle | ward module boundary |
| 075 | predictive repair planner | capstone mixed features |
| 076 | quality defect forecast | forecast loop model |
| 077 | sla breach early warning | optimization search |
| 078 | call center staffing model | rolling average simulator |
| 079 | shipping delay predictor | guardrail scoring |
| 080 | last mile route budget | terminal dashboard |
| 081 | learning progress tracker | contracts clamp |
| 082 | exam readiness planner | contracts progression |
| 083 | language practice scheduler | effects pipeline + stdin parsing |
| 084 | reading habit dashboard | effects and stderr |
| 085 | fitness training load | witness proof flow |
| 086 | calorie budget calculator | witness shape proof |
| 087 | sleep quality tracker | zones allocator |
| 088 | hydration goal monitor | memory intrinsic usage |
| 089 | meditation streak keeper | shape and impl methods |
| 090 | personal finance coach | scroll and branch |
| 091 | home renovation planner | weave implementation |
| 092 | pet care schedule model | generic helper |
| 093 | kitchen inventory guard | custom iterator |
| 094 | weekly meal budgeter | pipeline and bind |
| 095 | small business kpi board | collections usage |
| 096 | nonprofit donation forecast | macro expansion |
| 097 | volunteer shift allocator | async flow tide |
| 098 | community event dashboard | duality shape |
| 099 | disaster supply readiness | ward module boundary |
| 100 | release health command center | capstone mixed features |
