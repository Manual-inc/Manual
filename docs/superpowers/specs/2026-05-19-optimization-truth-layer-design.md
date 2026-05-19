# Optimization Truth Layer Design

## Context

Manual's current `optimization.record_run` endpoint persists run records, but `optimization.analyze`, `optimization.compare`, and `optimization.report` still return mostly fixed responses. That means the product can satisfy shape-based contracts while failing to deliver the main customer value promised by [[매뉴얼-최적화-기능]]: helping users understand their own workflow cost, verification gaps, and next improvements from actual execution data.

## Problem

Customers cannot trust optimization output if the analysis does not materially change when their run history changes. Today the product records the right evidence categories, but it does not consistently turn those records into customer-specific bottlenecks, regressions, comparisons, and recommendations.

## Goals

- Make persisted optimization runs the single source of truth for `optimization.analyze`, `optimization.compare`, and `optimization.report`.
- Preserve the existing JSON-RPC surface and the high-level response keys already used by CLI, app-server contract tests, and mac cucumber drivers.
- Produce analysis that changes when stored runs change.
- Keep the first release focused on trustworthy insight generation, not automatic workflow mutation.

## Non-Goals

- No automatic optimization application in this iteration.
- No UI redesign in this iteration.
- No incompatible CLI or app-server method rename.

## Truth Layer

The truth layer is the persisted `optimization_runs` namespace managed by app-server. Every optimization-facing response must be derived from this stored evidence set.

Run selection rules:

1. Prefer filtering by `manual_id` when supplied.
2. Otherwise group by `workflow_id`.
3. If neither is supplied, analyze the latest coherent cohort of runs from the same workflow.
4. Keep the current record shape, but normalize missing fields before calculation.

Normalization rules:

- Missing `token_usage`, `verification`, `time`, `model_calls`, `tool_calls`, and `context_sources` fall back to the existing default capture shape.
- Derived results may use normalized values, but the system should internally distinguish "recorded" vs "filled default" inputs so future reporting can expose weak evidence instead of overclaiming certainty.

## Analyze

`optimization.analyze` keeps the existing response keys:

- `candidates`
- `model_recommendations`
- `adaptive_compute`
- `regression`
- `bottlenecks`
- `preprocessing`
- `suggestions`
- `auto_apply_allowed`
- `requires_approval`
- `weakens_verification`

The difference is that values come from cohort calculations.

### Bottlenecks

- `token_waste`: steps with the largest token share, repeated budget overruns, or the highest `tokens / budget` ratio.
- `verification_gaps`: steps with missing verification items, unknown evidence, or degraded pass rate.
- `slow_steps`: steps with the largest average duration or repeated retries.
- `unstable_tasks`: steps or runs with status variance, verification variance, or large swings in token/time cost.

### Candidates

Candidate entries keep the current conceptual names such as `repeated_discovery`, `token_waste`, `missing_verification`, and `unstable_output`, but each candidate should be attached to the concrete step id and the supporting run ids that triggered it.

### Model Recommendations

- Recommend `use smaller model` only when a step is expensive, verification-sensitive enough to measure, and not high failure cost.
- Recommend `keep high-quality model` when a step is expensive but failure cost or verification sensitivity makes downgrade risky.

### Adaptive Compute Summary

The `adaptive_compute` object remains present, but its values should reflect the dominant signals in the analyzed cohort:

- determinism
- reasoning depth
- failure cost
- verifiability
- input size
- reusability

For the first iteration, this can be driven by simple heuristics from token concentration, retries, verification coverage, and model-call reasons.

### Regression

Regression should be based on real cohort comparison:

- `possible: true` only when a latest run is materially worse than a chosen baseline in token cost, time, verification, or status.
- `step_id` should point to the most severe regressed step.
- `reason` should summarize the measurable regression driver.

## Compare

`optimization.compare` should accept optional `before_run_id` and `after_run_id`.

- If both are supplied, compare them directly.
- If omitted, choose a baseline/latest pair from the filtered cohort.
- If the cohort is too small, return a weak-evidence comparison state instead of invented certainty.

The existing response shape remains:

- `token_delta`
- `verification_delta`
- `time_delta_ms`
- `failed_run`
- `successful_run`
- `retry_extra`
- `quality`

All values should be calculated from the selected runs and related retry evidence.

## Report

`optimization.report` keeps the existing top-level contract:

- `sections`
- `main_issue`
- `recommendations`

The sections remain `Token Usage`, `Verification`, and `Time`, but `main_issue` and `recommendations` must come from the same cohort calculations used by `analyze`, not from a separate static template. This prevents contradictions between analysis and report output.

## Compatibility

- Preserve existing JSON-RPC method names.
- Preserve current top-level response keys used by contract tests and CLI tests.
- Add new filter params only as optional expansions.

## Error Handling

- If no run cohort can be formed, return an empty-but-honest response with clear weak-evidence indicators rather than fabricated bottlenecks.
- If only one run exists, allow `report` and partial `analyze`, but downgrade `compare` certainty.

## Testing Strategy

### manual-optimization unit tests

- Verify `analyze` changes when run cohorts change.
- Verify `compare` respects explicit before/after ids.
- Verify `report` derives `main_issue` from actual token-heavy steps.

### app-server integration tests

- Record multiple runs through JSON-RPC.
- Assert that `optimization.analyze`, `optimization.compare`, and `optimization.report` vary with stored run data.

### CLI real tests

- Feed distinct optimization run inputs.
- Assert report strings and comparison deltas follow the supplied run history rather than a static fixture.

## Documentation Updates

Update:

- `docs/wiki/systems/매뉴얼-최적화-기능.md`
- `docs/wiki/작업-로그.md`

The documentation should state that Manual's first customer-visible optimization value depends on turning stored run evidence into truthful workflow-specific insight.
