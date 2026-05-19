# Optimization Truth Layer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Manual's optimization analysis, comparison, and reporting derive from persisted run history so customers see truthful workflow-specific insight instead of static canned output.

**Architecture:** Extend `manual-optimization` with pure cohort-analysis helpers that operate on stored run values, then thread persisted `optimization_runs` from app-server into those helpers without breaking the existing JSON-RPC surface. Keep the current response keys stable while converting their values from static defaults to data-driven calculations, and update tests plus wiki docs to prove the product now reacts to real execution evidence.

**Tech Stack:** Rust, serde_json, Cargo tests, llm-wiki documentation

---

### Task 1: Lock data-driven optimization behavior with failing tests

**Files:**
- Modify: `manual-rs/crates/manual-optimization/src/lib.rs`
- Create: `manual-rs/crates/app-server/tests/optimization_data_driven.rs`
- Modify: `app/cli/tests/real_cli.rs`

- [ ] **Step 1: Write a failing manual-optimization unit test for analyze cohort changes**

```rust
#[test]
fn analyze_changes_when_run_history_changes() {
    let stable = vec![json!({
        "id": "run-a",
        "workflow_id": "wf-main",
        "status": "completed",
        "token_usage": {
            "total": 1200,
            "by_step": [{ "step_id": "plan", "tokens": 400, "budget": 1000, "over_budget": false, "over_by": 0, "over_ratio": 0.0 }],
            "by_model": [{ "model": "gpt-5.4-mini", "tokens": 1200, "cost": 0.02 }],
            "hotspots": ["plan"]
        },
        "verification": { "pass_rate": 0.95, "requirements_satisfied": 0.95, "items": [], "missing": [], "risks": [] },
        "time": { "total_ms": 500, "by_step": [{ "step_id": "plan", "duration_ms": 500, "retries": 0 }], "review_ms": 0 },
        "model_calls": [],
        "tool_calls": [],
        "context_sources": [],
        "created_at": "2026-05-19T00:00:00Z"
    })];
    let regressed = vec![
        stable[0].clone(),
        json!({
            "id": "run-b",
            "workflow_id": "wf-main",
            "status": "completed",
            "token_usage": {
                "total": 5400,
                "by_step": [
                    { "step_id": "plan", "tokens": 1200, "budget": 1500, "over_budget": false, "over_by": 0, "over_ratio": 0.0 },
                    { "step_id": "implement", "tokens": 4200, "budget": 3000, "over_budget": true, "over_by": 1200, "over_ratio": 0.4 }
                ],
                "by_model": [{ "model": "gpt-5.5", "tokens": 4200, "cost": 0.42 }],
                "hotspots": ["implement"]
            },
            "verification": {
                "pass_rate": 0.72,
                "requirements_satisfied": 0.8,
                "items": [
                    { "name": "tests", "status": "passed", "evidence": ["cargo test"] },
                    { "name": "review", "status": "unknown", "evidence": [] }
                ],
                "missing": ["review"],
                "risks": ["review evidence missing"]
            },
            "time": {
                "total_ms": 2400,
                "by_step": [
                    { "step_id": "plan", "duration_ms": 600, "retries": 0 },
                    { "step_id": "implement", "duration_ms": 1800, "retries": 1 }
                ],
                "review_ms": 300
            },
            "model_calls": [{ "step_id": "implement", "model": "gpt-5.5", "tokens": 4200, "cost": 0.42, "reason": "high-risk implementation" }],
            "tool_calls": [{ "tool": "rg", "count": 3 }],
            "context_sources": [{ "source": "docs/wiki/목차.md", "summary": "wiki navigation" }],
            "created_at": "2026-05-19T01:00:00Z"
        })
    ];

    let stable_analysis = super::analyze(&json!({ "workflow_id": "wf-main" }), &stable);
    let regressed_analysis = super::analyze(&json!({ "workflow_id": "wf-main" }), &regressed);

    assert_ne!(stable_analysis["bottlenecks"], regressed_analysis["bottlenecks"]);
    assert_eq!(regressed_analysis["regression"]["possible"], true);
}
```

- [ ] **Step 2: Run the focused unit test to verify it fails**

Run: `cargo test -p manual-optimization analyze_changes_when_run_history_changes -- --exact`
Expected: FAIL because `analyze` does not yet accept run history or derive results from it.

- [ ] **Step 3: Write a failing app-server integration test for persisted optimization history**

```rust
#[test]
fn optimization_endpoints_reflect_persisted_run_history() {
    let server = test_server("optimization-data-driven");
    rpc(&server, json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "optimization.record_run",
        "params": {
            "run_id": "before",
            "workflow_id": "wf-main",
            "status": "completed",
            "token_usage": {
                "total": 1800,
                "by_step": [{ "step_id": "plan", "tokens": 1800, "budget": 2500, "over_budget": false, "over_by": 0, "over_ratio": 0.0 }],
                "by_model": [{ "model": "gpt-5.4-mini", "tokens": 1800, "cost": 0.04 }],
                "hotspots": ["plan"]
            },
            "verification": { "pass_rate": 0.94, "requirements_satisfied": 0.94, "items": [], "missing": [], "risks": [] },
            "time": { "total_ms": 700, "by_step": [{ "step_id": "plan", "duration_ms": 700, "retries": 0 }], "review_ms": 0 }
        }
    }));
    let _ = rpc(&server, json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "optimization.record_run",
        "params": {
            "run_id": "after",
            "workflow_id": "wf-main",
            "status": "completed",
            "token_usage": {
                "total": 6200,
                "by_step": [
                    { "step_id": "plan", "tokens": 1000, "budget": 1500, "over_budget": false, "over_by": 0, "over_ratio": 0.0 },
                    { "step_id": "implement", "tokens": 5200, "budget": 3200, "over_budget": true, "over_by": 2000, "over_ratio": 0.625 }
                ],
                "by_model": [{ "model": "gpt-5.5", "tokens": 5200, "cost": 0.52 }],
                "hotspots": ["implement"]
            },
            "verification": { "pass_rate": 0.7, "requirements_satisfied": 0.78, "items": [], "missing": ["review"], "risks": ["review evidence missing"] },
            "time": { "total_ms": 3100, "by_step": [{ "step_id": "implement", "duration_ms": 3100, "retries": 2 }], "review_ms": 400 }
        }
    }));

    let report = rpc(&server, json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "optimization.report",
        "params": { "workflow_id": "wf-main" }
    }));

    assert_eq!(report["result"]["main_issue"], "implementation step used most tokens");
}
```

- [ ] **Step 4: Run the app-server integration test to verify it fails**

Run: `cargo test -p app-server optimization_endpoints_reflect_persisted_run_history -- --exact`
Expected: FAIL because app-server still ignores stored optimization history when calling `analyze`, `compare`, and `report`.

- [ ] **Step 5: Update the CLI real test to expect data-driven report output**

```rust
let optimization_json = harness.write_json(
    "optimization.json",
    &json!({
        "run_id": "opt-1",
        "workflow_id": "wf-main",
        "status": "completed",
        "token_usage": {
            "total": 5400,
            "by_step": [
                { "step_id": "plan", "tokens": 1200, "budget": 1500, "over_budget": false, "over_by": 0, "over_ratio": 0.0 },
                { "step_id": "implement", "tokens": 4200, "budget": 3000, "over_budget": true, "over_by": 1200, "over_ratio": 0.4 }
            ],
            "by_model": [{ "model": "gpt-5.5", "tokens": 4200, "cost": 0.42 }],
            "hotspots": ["implement"]
        }
    }),
);
```

- [ ] **Step 6: Run the CLI real test to verify it fails before implementation**

Run: `cargo test --manifest-path app/cli/Cargo.toml manual_sandbox_skill_optimization_and_agent_commands_work_against_real_app_server -- --exact`
Expected: FAIL once the assertion expects report output tied to the provided run payload rather than a hard-coded static template.

### Task 2: Implement data-driven cohort analysis in manual-optimization

**Files:**
- Modify: `manual-rs/crates/manual-optimization/src/lib.rs`

- [ ] **Step 1: Add helpers to normalize and filter run cohorts**

```rust
fn select_runs<'a>(params: &Value, runs: &'a [Value]) -> Vec<&'a Value> {
    let manual_id = params.get("manual_id").and_then(Value::as_str).filter(|value| !value.is_empty());
    let workflow_id = params.get("workflow_id").and_then(Value::as_str).filter(|value| !value.is_empty());

    let mut selected = runs
        .iter()
        .filter(|run| {
            manual_id.is_none_or(|id| run["manual_id"].as_str() == Some(id))
                && workflow_id.is_none_or(|id| run["workflow_id"].as_str() == Some(id))
        })
        .collect::<Vec<_>>();

    selected.sort_by_key(|run| run["created_at"].as_str().unwrap_or_default().to_owned());
    selected
}
```

- [ ] **Step 2: Implement analyze from selected run history**

```rust
pub fn analyze(params: &Value, runs: &[Value]) -> Value {
    let selected = select_runs(params, runs);
    if selected.is_empty() {
        return empty_analysis();
    }

    let bottlenecks = derive_bottlenecks(&selected);
    let regression = derive_regression(&selected);

    json!({
        "candidates": derive_candidates(&selected, &bottlenecks),
        "model_recommendations": derive_model_recommendations(&selected),
        "adaptive_compute": derive_adaptive_compute(&selected),
        "regression": regression,
        "bottlenecks": bottlenecks,
        "preprocessing": derive_preprocessing(&selected),
        "suggestions": derive_suggestions(&selected),
        "auto_apply_allowed": ["file list caching", "log cleanup", "summary input generation"],
        "requires_approval": derive_requires_approval(&selected),
        "weakens_verification": params["weakens_verification"].as_bool().unwrap_or(false),
    })
}
```

- [ ] **Step 3: Implement compare from explicit ids or baseline/latest selection**

```rust
pub fn compare_runs(params: &Value, runs: &[Value]) -> Value {
    let selected = select_runs(params, runs);
    let (before, after) = choose_comparison_pair(params, &selected);
    let Some((before, after)) = before.zip(after) else {
        return empty_comparison();
    };

    json!({
        "token_delta": after["token_usage"]["total"].as_i64().unwrap_or(0) - before["token_usage"]["total"].as_i64().unwrap_or(0),
        "verification_delta": after["verification"]["pass_rate"].as_f64().unwrap_or(0.0) - before["verification"]["pass_rate"].as_f64().unwrap_or(0.0),
        "time_delta_ms": after["time"]["total_ms"].as_i64().unwrap_or(0) - before["time"]["total_ms"].as_i64().unwrap_or(0),
        "failed_run": summarize_failed_run(&selected),
        "successful_run": summarize_successful_run(&selected),
        "retry_extra": summarize_retry_extra(before, after),
        "quality": summarize_quality(before, after),
    })
}
```

- [ ] **Step 4: Implement report from the same cohort calculations**

```rust
pub fn report(params: &Value, runs: &[Value]) -> Value {
    let analysis = analyze(params, runs);
    json!({
        "sections": ["Token Usage", "Verification", "Time"],
        "main_issue": derive_main_issue(&analysis),
        "recommendations": analysis["suggestions"].clone(),
    })
}
```

- [ ] **Step 5: Run the manual-optimization unit tests to verify green**

Run: `cargo test -p manual-optimization`
Expected: PASS

### Task 3: Thread persisted optimization history through app-server

**Files:**
- Modify: `manual-rs/crates/app-server/src/lib.rs`
- Test: `manual-rs/crates/app-server/tests/optimization_data_driven.rs`

- [ ] **Step 1: Add a helper that snapshots stored optimization runs**

```rust
fn optimization_run_values(&self) -> Vec<Value> {
    self.optimization_runs
        .read()
        .expect("optimization lock should not poison")
        .values()
        .cloned()
        .collect()
}
```

- [ ] **Step 2: Pass run history into analyze, compare, and report**

```rust
fn analyze_optimization(&self, id: Value, params: Value) -> Value {
    let runs = self.optimization_run_values();
    rpc_result(id, manual_optimization::analyze(&params, &runs))
}

fn compare_optimization_runs(&self, id: Value, params: Value) -> Value {
    let runs = self.optimization_run_values();
    rpc_result(id, manual_optimization::compare_runs(&params, &runs))
}

fn optimization_report(&self, id: Value, params: Value) -> Value {
    let runs = self.optimization_run_values();
    rpc_result(id, manual_optimization::report(&params, &runs))
}
```

- [ ] **Step 3: Run the focused app-server test to verify green**

Run: `cargo test -p app-server optimization_endpoints_reflect_persisted_run_history -- --exact`
Expected: PASS

- [ ] **Step 4: Run the full app-server suite**

Run: `cargo test -p app-server`
Expected: PASS

### Task 4: Align contract tests, CLI tests, and docs with real run data

**Files:**
- Modify: `manual-rs/crates/app-server/tests/usecase_contracts.rs`
- Modify: `app/cli/tests/real_cli.rs`
- Modify: `docs/wiki/systems/매뉴얼-최적화-기능.md`
- Modify: `docs/wiki/작업-로그.md`

- [ ] **Step 1: Make optimization contract scenarios feed distinct run evidence**

```rust
fn record_optimization_run(world: &mut ManualWorld, run_id: &str, status: &str) {
    let params = match run_id {
        "before" => json!({
            "run_id": "before",
            "workflow_id": "ai-workflow",
            "status": "completed",
            "token_usage": {
                "total": 1800,
                "by_step": [{ "step_id": "plan", "tokens": 1800, "budget": 2500, "over_budget": false, "over_by": 0, "over_ratio": 0.0 }],
                "by_model": [{ "model": "gpt-5.4-mini", "tokens": 1800, "cost": 0.04 }],
                "hotspots": ["plan"]
            }
        }),
        "after" => json!({
            "run_id": "after",
            "workflow_id": "ai-workflow",
            "status": "completed",
            "token_usage": {
                "total": 6200,
                "by_step": [
                    { "step_id": "plan", "tokens": 1000, "budget": 1500, "over_budget": false, "over_by": 0, "over_ratio": 0.0 },
                    { "step_id": "implement", "tokens": 5200, "budget": 3200, "over_budget": true, "over_by": 2000, "over_ratio": 0.625 }
                ],
                "by_model": [{ "model": "gpt-5.5", "tokens": 5200, "cost": 0.52 }],
                "hotspots": ["implement"]
            }
        }),
        _ => json!({ "run_id": run_id, "workflow_id": "ai-workflow", "status": status }),
    };
    let recorded = rpc(world, json!({ "jsonrpc": "2.0", "id": 65, "method": "optimization.record_run", "params": params }));
    world.optimization_run_id = recorded["result"]["run"]["id"].as_str().map(str::to_owned);
    world.last_response = Some(recorded);
}
```

- [ ] **Step 2: Update the CLI real test assertion to verify data-driven output**

```rust
let optimization_report = harness.run_jsons([
    "optimization".into(),
    "report".into(),
    "--params".into(),
    optimization_report_params.display().to_string(),
]);
assert_eq!(optimization_report[0]["main_issue"], "implementation step used most tokens");
```

- [ ] **Step 3: Document the truth-layer behavior in the optimization wiki page**

```md
## Optimization Truth Layer

- `optimization.record_run`에 저장된 실행 기록이 `analyze`, `compare`, `report`의 단일 근거다.
- 리포트와 분석은 같은 cohort 계산 결과를 사용해야 한다.
- 근거가 부족한 경우에는 확정적 추천 대신 약한 근거 상태를 보여준다.
```

- [ ] **Step 4: Append a work-log entry**

```md
## [2026-05-19] update | Optimization truth layer made data-driven

- Summary: `optimization.analyze`, `optimization.compare`, `optimization.report`가 저장된 실행 기록 cohort를 직접 계산하도록 바꾸고, 관련 계약/CLI 테스트를 실데이터 기반으로 보강했다.
- Pages created: `docs/superpowers/specs/2026-05-19-optimization-truth-layer-design.md`, `docs/superpowers/plans/2026-05-19-optimization-truth-layer.md`
- Pages updated: `docs/wiki/systems/매뉴얼-최적화-기능.md`, `docs/wiki/작업-로그.md`
```

- [ ] **Step 5: Run verification commands**

Run: `cargo test -p manual-optimization`
Expected: PASS

Run: `cargo test -p app-server`
Expected: PASS

Run: `cargo test --manifest-path app/cli/Cargo.toml`
Expected: PASS

Run: `cargo run --manifest-path docs/test/Cargo.toml`
Expected: `ok: no orphan documents found`
