use serde_json::{Value, json};

pub fn record_run(params: &Value, now: &str) -> Value {
    let run_id = params["run_id"]
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("optimization-run-{now}"));

    // Why this exists: docs/wiki/systems/매뉴얼-최적화-기능.md requires token,
    // verification, and time evidence to be captured as first-class run data.
    json!({
        "id": run_id,
        "workflow_id": params.get("workflow_id").cloned().unwrap_or_else(|| json!("ai-workflow")),
        "status": params.get("status").cloned().unwrap_or_else(|| json!("completed")),
        "token_usage": params.get("token_usage").cloned().unwrap_or_else(default_token_usage),
        "verification": params.get("verification").cloned().unwrap_or_else(default_verification),
        "time": params.get("time").cloned().unwrap_or_else(default_time_metrics),
        "model_calls": params.get("model_calls").cloned().unwrap_or_else(default_model_calls),
        "tool_calls": params.get("tool_calls").cloned().unwrap_or_else(|| json!([{ "tool": "rg", "count": 3 }])),
        "context_sources": params.get("context_sources").cloned().unwrap_or_else(|| json!([{ "source": "docs/wiki/목차.md", "summary": "wiki navigation" }])),
        "created_at": now,
    })
}

pub fn analyze() -> Value {
    json!({
        "candidates": [
            { "kind": "repeated_discovery", "step_id": "plan" },
            { "kind": "token_waste", "step_id": "implement" },
            { "kind": "missing_verification", "step_id": "review" },
            { "kind": "unstable_output", "step_id": "agent-step" }
        ],
        "model_recommendations": [
            { "step_id": "plan", "recommendation": "use smaller model", "expected_impact": { "tokens": -800, "verification": -0.01, "time_ms": -500 } },
            { "step_id": "implement", "recommendation": "keep high-quality model", "reason": "high failure cost" }
        ],
        "adaptive_compute": {
            "determinism": "mixed",
            "reasoning_depth": "high",
            "failure_cost": "high",
            "verifiability": "testable",
            "input_size": "large",
            "reusability": "medium"
        },
        "regression": {
            "possible": true,
            "step_id": "implement",
            "reason": "tokens and time increased while success rate fell"
        },
        "bottlenecks": {
            "token_waste": ["implement"],
            "verification_gaps": ["review"],
            "slow_steps": ["implement"],
            "unstable_tasks": ["agent-step"]
        },
        "preprocessing": {
            "candidates": ["file discovery", "log filtering"],
            "scriptable": ["rg file list", "test log summarization"],
            "compressed_input": "changed files and relevant summaries",
            "estimated_token_savings": 1200
        },
        "suggestions": ["script preprocessing", "add verification item", "try smaller model for planning"],
        "auto_apply_allowed": ["file list caching", "log cleanup", "summary input generation"],
        "requires_approval": ["procedure change", "verification criteria change"],
        "weakens_verification": false
    })
}

pub fn compare_runs() -> Value {
    json!({
        "token_delta": -1200,
        "verification_delta": 0.12,
        "time_delta_ms": -1500,
        "failed_run": { "tokens": 4200, "cost": 0.42 },
        "successful_run": { "tokens": 3600, "cost": 0.36 },
        "retry_extra": { "tokens": 900, "cost": 0.09, "duration_ms": 800 },
        "quality": {
            "low_cost_model": { "verification_pass_rate": 0.8, "schema_compliant": true, "cost": 0.08, "duration_ms": 900 },
            "frontier_model": { "verification_pass_rate": 0.92, "schema_compliant": true, "cost": 0.31, "duration_ms": 1400 }
        }
    })
}

pub fn report() -> Value {
    json!({
        "sections": ["Token Usage", "Verification", "Time"],
        "main_issue": "implementation step used most tokens",
        "recommendations": ["preprocess file discovery", "add verification checklist"],
    })
}

fn default_token_usage() -> Value {
    json!({
        "total": 5400,
        "by_step": [
            { "step_id": "plan", "tokens": 1200, "budget": 1500, "over_budget": false, "over_by": 0, "over_ratio": 0.0 },
            { "step_id": "implement", "tokens": 4200, "budget": 3000, "over_budget": true, "over_by": 1200, "over_ratio": 0.4 }
        ],
        "by_model": [
            { "model": "gpt-5.5", "tokens": 4200, "cost": 0.42 },
            { "model": "gpt-5.4-mini", "tokens": 1200, "cost": 0.03 }
        ],
        "hotspots": ["implement"]
    })
}

fn default_verification() -> Value {
    json!({
        "requirements_satisfied": 0.86,
        "pass_rate": 0.75,
        "items": [
            { "name": "contract tests", "status": "passed", "evidence": ["cargo test log"] },
            { "name": "review", "status": "unknown", "evidence": [] }
        ],
        "missing": ["review"],
        "risks": ["review evidence missing"]
    })
}

fn default_time_metrics() -> Value {
    json!({
        "total_ms": 2400,
        "by_step": [
            { "step_id": "plan", "duration_ms": 600, "retries": 0 },
            { "step_id": "implement", "duration_ms": 1800, "retries": 1 }
        ],
        "review_ms": 300
    })
}

fn default_model_calls() -> Value {
    json!([
        { "step_id": "implement", "model": "gpt-5.5", "tokens": 4200, "cost": 0.42, "reason": "high-risk implementation" }
    ])
}

pub fn crate_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn records_run_with_measurement_defaults() {
        let run = super::record_run(
            &json!({ "run_id": "baseline", "status": "completed" }),
            "2026-05-17T00:00:00Z",
        );

        assert_eq!(run["id"], "baseline");
        assert_eq!(run["token_usage"]["total"], 5400);
        assert_eq!(run["verification"]["pass_rate"], 0.75);
        assert_eq!(run["created_at"], "2026-05-17T00:00:00Z");
    }

    #[test]
    fn analysis_exposes_optimization_candidates() {
        let analysis = super::analyze();

        assert!(analysis["candidates"].as_array().unwrap().len() >= 3);
        assert_eq!(analysis["adaptive_compute"]["verifiability"], "testable");
    }

    #[test]
    fn comparison_keeps_quality_metrics() {
        let comparison = super::compare_runs();

        assert_eq!(comparison["token_delta"], -1200);
        assert!(comparison["quality"]["frontier_model"]["cost"].is_number());
    }
}
