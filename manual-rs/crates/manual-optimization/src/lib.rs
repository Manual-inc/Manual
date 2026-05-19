use std::collections::{BTreeMap, BTreeSet};

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
        "manual_id": params.get("manual_id").cloned().unwrap_or(Value::Null),
        "workflow_id": params.get("workflow_id").cloned().unwrap_or_else(|| json!("ai-workflow")),
        "status": params.get("status").cloned().unwrap_or_else(|| json!("completed")),
        "token_usage": params.get("token_usage").cloned().unwrap_or_else(default_token_usage),
        "verification": params.get("verification").cloned().unwrap_or_else(default_verification),
        "time": params.get("time").cloned().unwrap_or_else(default_time_metrics),
        "model_calls": params.get("model_calls").cloned().unwrap_or_else(default_model_calls),
        "tool_calls": params.get("tool_calls").cloned().unwrap_or_else(|| json!([{ "tool": "rg", "count": 3 }])),
        "context_sources": params.get("context_sources").cloned().unwrap_or_else(|| json!([{ "source": "docs/wiki/목차.md", "summary": "wiki navigation" }])),
        "measurement_mode": params.get("measurement_mode").cloned().unwrap_or_else(|| json!("reported")),
        "measurement_note": params.get("measurement_note").cloned().unwrap_or_else(|| json!("Recorded optimization metrics.")),
        "created_at": now,
    })
}

pub fn analyze(params: &Value, runs: &[Value]) -> Value {
    // Why this exists: docs/wiki/systems/매뉴얼-최적화-기능.md requires
    // analysis, comparison, and reporting to derive customer insight from
    // stored run evidence instead of static canned responses.
    let selected = select_runs(params, runs);
    if selected.is_empty() {
        return empty_analysis(params["weakens_verification"].as_bool().unwrap_or(false));
    }

    let bottlenecks = derive_bottlenecks(&selected);
    let model_recommendations = derive_model_recommendations(&selected, &bottlenecks);
    let suggestions = derive_suggestions(&selected, &bottlenecks, &model_recommendations);
    let (measurement_mode, measurement_note) = measurement_provenance(&selected);

    json!({
        "candidates": derive_candidates(&selected, &bottlenecks),
        "model_recommendations": model_recommendations,
        "adaptive_compute": derive_adaptive_compute(&selected, &bottlenecks),
        "regression": derive_regression(&selected),
        "bottlenecks": bottlenecks,
        "preprocessing": derive_preprocessing(&selected, &bottlenecks),
        "suggestions": suggestions,
        "auto_apply_allowed": ["file list caching", "log cleanup", "summary input generation"],
        "requires_approval": derive_requires_approval(params, &selected),
        "weakens_verification": params["weakens_verification"].as_bool().unwrap_or(false),
        "measurement_mode": measurement_mode,
        "measurement_note": measurement_note,
    })
}

pub fn compare_runs(params: &Value, runs: &[Value]) -> Value {
    let selected = select_runs(params, runs);
    let Some((before, after)) = choose_comparison_pair(params, &selected) else {
        return empty_comparison();
    };
    let (measurement_mode, measurement_note) = measurement_provenance(&selected);

    json!({
        "token_delta": total_tokens(after) - total_tokens(before),
        "verification_delta": verification_pass_rate(after) - verification_pass_rate(before),
        "time_delta_ms": total_duration_ms(after) - total_duration_ms(before),
        "failed_run": summarize_run_by_status(&selected, "failed"),
        "successful_run": summarize_run_by_status(&selected, "completed"),
        "retry_extra": summarize_retry_extra(before, after),
        "quality": summarize_quality(&selected),
        "measurement_mode": measurement_mode,
        "measurement_note": measurement_note,
    })
}

pub fn report(params: &Value, runs: &[Value]) -> Value {
    let analysis = analyze(params, runs);
    let selected = select_runs(params, runs);
    let (measurement_mode, measurement_note) = measurement_provenance(&selected);
    json!({
        "sections": ["Token Usage", "Verification", "Time"],
        "main_issue": derive_main_issue(&analysis),
        "recommendations": analysis["suggestions"].clone(),
        "measurement_mode": measurement_mode,
        "measurement_note": measurement_note,
    })
}

#[derive(Default, Clone)]
struct StepStats {
    tokens: i64,
    duration_ms: i64,
    retries: i64,
    occurrences: i64,
    over_budget_count: i64,
    max_over_ratio: f64,
}

fn empty_analysis(weakens_verification: bool) -> Value {
    json!({
        "candidates": [],
        "model_recommendations": [],
        "adaptive_compute": {
            "determinism": "unknown",
            "reasoning_depth": "unknown",
            "failure_cost": "unknown",
            "verifiability": "unknown",
            "input_size": "unknown",
            "reusability": "unknown"
        },
        "regression": {
            "possible": false,
            "step_id": Value::Null,
            "reason": "insufficient optimization history"
        },
        "bottlenecks": {
            "token_waste": [],
            "verification_gaps": [],
            "slow_steps": [],
            "unstable_tasks": []
        },
        "preprocessing": {
            "candidates": [],
            "scriptable": [],
            "compressed_input": "insufficient history",
            "estimated_token_savings": 0
        },
        "suggestions": [],
        "auto_apply_allowed": [],
        "requires_approval": [],
        "weakens_verification": weakens_verification,
        "measurement_mode": "unknown",
        "measurement_note": "Measurement provenance unavailable.",
    })
}

fn empty_comparison() -> Value {
    json!({
        "token_delta": 0,
        "verification_delta": 0.0,
        "time_delta_ms": 0,
        "failed_run": { "tokens": 0, "cost": 0.0 },
        "successful_run": { "tokens": 0, "cost": 0.0 },
        "retry_extra": { "tokens": 0, "cost": 0.0, "duration_ms": 0 },
        "quality": {
            "low_cost_model": { "verification_pass_rate": 0.0, "schema_compliant": true, "cost": 0.0, "duration_ms": 0 },
            "frontier_model": { "verification_pass_rate": 0.0, "schema_compliant": true, "cost": 0.0, "duration_ms": 0 }
        },
        "measurement_mode": "unknown",
        "measurement_note": "Measurement provenance unavailable.",
    })
}

fn select_runs(params: &Value, runs: &[Value]) -> Vec<Value> {
    let manual_id = params
        .get("manual_id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty());
    let workflow_id = params
        .get("workflow_id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty());

    let mut selected = runs.iter().map(normalize_run).collect::<Vec<_>>();
    selected.sort_by(|left, right| {
        let left_key = (
            left["created_at"].as_str().unwrap_or_default(),
            left["id"].as_str().unwrap_or_default(),
        );
        let right_key = (
            right["created_at"].as_str().unwrap_or_default(),
            right["id"].as_str().unwrap_or_default(),
        );
        left_key.cmp(&right_key)
    });

    if let Some(id) = manual_id {
        selected.retain(|run| run["manual_id"].as_str() == Some(id));
    }

    if let Some(id) = workflow_id {
        selected.retain(|run| run["workflow_id"].as_str() == Some(id));
    } else if manual_id.is_none()
        && let Some(latest_workflow_id) = selected.iter().rev().find_map(|run| {
            run["workflow_id"]
                .as_str()
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
        })
    {
        selected.retain(|run| run["workflow_id"].as_str() == Some(latest_workflow_id.as_str()));
    }

    selected
}

fn normalize_run(run: &Value) -> Value {
    let mut normalized = run.clone();
    if !normalized.is_object() {
        normalized = json!({});
    }

    if normalized["manual_id"].is_null() {
        normalized["manual_id"] = Value::Null;
    }
    if !normalized["workflow_id"].is_string() {
        normalized["workflow_id"] = json!("ai-workflow");
    }
    if !normalized["status"].is_string() {
        normalized["status"] = json!("completed");
    }
    if !normalized["token_usage"].is_object() {
        normalized["token_usage"] = default_token_usage();
    }
    if !normalized["verification"].is_object() {
        normalized["verification"] = default_verification();
    }
    if !normalized["time"].is_object() {
        normalized["time"] = default_time_metrics();
    }
    if !normalized["model_calls"].is_array() {
        normalized["model_calls"] = default_model_calls();
    }
    if !normalized["tool_calls"].is_array() {
        normalized["tool_calls"] = json!([{ "tool": "rg", "count": 3 }]);
    }
    if !normalized["context_sources"].is_array() {
        normalized["context_sources"] = json!([{ "source": "docs/wiki/목차.md", "summary": "wiki navigation" }]);
    }
    if !normalized["created_at"].is_string() {
        normalized["created_at"] = json!("");
    }

    normalized
}

fn derive_bottlenecks(selected: &[Value]) -> Value {
    let step_stats = collect_step_stats(selected);
    let token_waste = ranked_step_ids(&step_stats, |stats| {
        (stats.over_budget_count as f64 * 10_000.0)
            + stats.tokens as f64
            + (stats.max_over_ratio * 1_000.0)
    });
    let slow_steps = ranked_step_ids(&step_stats, |stats| {
        average(stats.duration_ms, stats.occurrences) as f64 + (stats.retries as f64 * 500.0)
    });

    let mut verification_gaps = Vec::new();
    let mut unstable_tasks = Vec::new();

    for run in selected {
        for missing in run["verification"]["missing"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            push_unique(&mut verification_gaps, missing.to_owned());
        }
        for item in run["verification"]["items"]
            .as_array()
            .into_iter()
            .flatten()
        {
            if item["status"] == "unknown" {
                push_unique(
                    &mut verification_gaps,
                    item["name"].as_str().unwrap_or("unknown").to_owned(),
                );
            }
        }
    }

    if selected.len() > 1 {
        let regression = derive_regression(selected);
        if regression["possible"] == true
            && let Some(step_id) = regression["step_id"].as_str()
        {
            push_unique(&mut unstable_tasks, step_id.to_owned());
        }
    }

    if unstable_tasks.is_empty()
        && let Some(step_id) = slow_steps.as_array().and_then(|steps| steps.first()).and_then(Value::as_str)
    {
        push_unique(&mut unstable_tasks, step_id.to_owned());
    }

    json!({
        "token_waste": token_waste,
        "verification_gaps": verification_gaps,
        "slow_steps": slow_steps,
        "unstable_tasks": unstable_tasks,
    })
}

fn derive_candidates(selected: &[Value], bottlenecks: &Value) -> Value {
    let mut candidates = Vec::new();

    if repeated_discovery_detected(selected) {
        candidates.push(json!({
            "kind": "repeated_discovery",
            "step_id": "plan",
            "run_ids": run_ids(selected),
        }));
    }

    if let Some(step_id) = bottlenecks["token_waste"].as_array().and_then(|steps| steps.first()) {
        candidates.push(json!({
            "kind": "token_waste",
            "step_id": step_id,
            "run_ids": run_ids(selected),
        }));
    }

    if let Some(step_id) = bottlenecks["verification_gaps"]
        .as_array()
        .and_then(|steps| steps.first())
    {
        candidates.push(json!({
            "kind": "missing_verification",
            "step_id": step_id,
            "run_ids": run_ids(selected),
        }));
    }

    if let Some(step_id) = bottlenecks["unstable_tasks"]
        .as_array()
        .and_then(|steps| steps.first())
    {
        candidates.push(json!({
            "kind": "unstable_output",
            "step_id": step_id,
            "run_ids": run_ids(selected),
        }));
    }

    Value::Array(candidates)
}

fn derive_model_recommendations(selected: &[Value], bottlenecks: &Value) -> Value {
    let mut recommendations = Vec::new();

    let step_stats = collect_step_stats(selected);
    if step_stats.contains_key("plan") {
        recommendations.push(json!({
            "step_id": "plan",
            "recommendation": "use smaller model",
            "expected_impact": {
                "tokens": -estimated_plan_token_reduction(selected),
                "verification": -0.01,
                "time_ms": -500
            }
        }));
    }

    if let Some(step_id) = bottlenecks["token_waste"]
        .as_array()
        .and_then(|steps| steps.first())
        .and_then(Value::as_str)
        .filter(|step_id| is_high_failure_cost_step(selected, step_id))
    {
        recommendations.push(json!({
            "step_id": step_id,
            "recommendation": "keep high-quality model",
            "reason": "high failure cost"
        }));
    }

    Value::Array(recommendations)
}

fn derive_adaptive_compute(selected: &[Value], bottlenecks: &Value) -> Value {
    let average_tokens = average(
        selected.iter().map(total_tokens).sum::<i64>(),
        selected.len() as i64,
    );
    let has_retry = selected.iter().any(|run| {
        run["time"]["by_step"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|step| step["retries"].as_i64().unwrap_or(0) > 0)
    });
    let has_high_risk_reason = selected.iter().any(|run| {
        run["model_calls"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|call| call["reason"].as_str())
            .any(|reason| reason.contains("high-risk") || reason.contains("high failure"))
    });

    json!({
        "determinism": if repeated_discovery_detected(selected) { "mixed" } else { "mostly-deterministic" },
        "reasoning_depth": if has_high_risk_reason || average_tokens >= 4000 { "high" } else { "medium" },
        "failure_cost": if has_high_risk_reason { "high" } else { "medium" },
        "verifiability": if !bottlenecks["verification_gaps"].as_array().unwrap_or(&Vec::new()).is_empty() { "testable" } else { "automated" },
        "input_size": if average_tokens >= 4000 { "large" } else if average_tokens >= 2000 { "medium" } else { "small" },
        "reusability": if repeated_discovery_detected(selected) || has_retry { "medium" } else { "low" },
    })
}

fn derive_regression(selected: &[Value]) -> Value {
    if selected.len() < 2 {
        return json!({
            "possible": false,
            "step_id": Value::Null,
            "reason": "insufficient optimization history"
        });
    }

    let before = selected
        .iter()
        .min_by(|left, right| regression_order_key(left).cmp(&regression_order_key(right)))
        .expect("selected should not be empty");
    let after = selected
        .iter()
        .max_by(|left, right| regression_order_key(left).cmp(&regression_order_key(right)))
        .expect("selected should not be empty");
    let token_delta = total_tokens(after) - total_tokens(before);
    let time_delta = total_duration_ms(after) - total_duration_ms(before);
    let verification_delta = verification_pass_rate(after) - verification_pass_rate(before);
    let status_regressed = after["status"].as_str() != before["status"].as_str();
    let possible = token_delta > 0 || time_delta > 0 || verification_delta < 0.0 || status_regressed;

    let reason = if token_delta > 0 && time_delta > 0 && verification_delta < 0.0 {
        "tokens and time increased while success rate fell"
    } else if token_delta > 0 {
        "token usage increased"
    } else if time_delta > 0 {
        "execution time increased"
    } else if verification_delta < 0.0 {
        "verification pass rate fell"
    } else {
        "status regressed"
    };

    json!({
        "possible": possible,
        "step_id": dominant_step_id(after),
        "reason": if possible { reason } else { "no measurable regression detected" },
    })
}

fn regression_order_key(run: &Value) -> (&str, i64, &str) {
    (
        run["created_at"].as_str().unwrap_or_default(),
        total_tokens(run),
        run["id"].as_str().unwrap_or_default(),
    )
}

fn derive_preprocessing(selected: &[Value], bottlenecks: &Value) -> Value {
    let estimated_token_savings = bottlenecks["token_waste"]
        .as_array()
        .and_then(|steps| steps.first())
        .and_then(Value::as_str)
        .map(|step_id| step_token_overage(selected, step_id))
        .unwrap_or_default()
        .max(0);

    json!({
        "candidates": if repeated_discovery_detected(selected) {
            vec!["file discovery", "log filtering"]
        } else {
            vec!["log filtering"]
        },
        "scriptable": ["rg file list", "test log summarization"],
        "compressed_input": "changed files and relevant summaries",
        "estimated_token_savings": estimated_token_savings,
    })
}

fn derive_suggestions(
    selected: &[Value],
    bottlenecks: &Value,
    model_recommendations: &Value,
) -> Value {
    let mut suggestions = Vec::new();
    if let Some(step_id) = bottlenecks["token_waste"]
        .as_array()
        .and_then(|steps| steps.first())
        .and_then(Value::as_str)
    {
        if repeated_discovery_detected(selected) {
            suggestions.push(Value::String("preprocess file discovery".to_owned()));
        } else {
            suggestions.push(Value::String(format!(
                "preprocess {} inputs",
                human_step_label(step_id)
            )));
        }
    }
    if !bottlenecks["verification_gaps"]
        .as_array()
        .unwrap_or(&Vec::new())
        .is_empty()
    {
        suggestions.push(Value::String("add verification checklist".to_owned()));
    }
    if model_recommendations
        .as_array()
        .into_iter()
        .flatten()
        .any(|recommendation| recommendation["recommendation"] == "use smaller model")
    {
        suggestions.push(Value::String("try smaller model for planning".to_owned()));
    }
    Value::Array(suggestions)
}

fn derive_requires_approval(params: &Value, selected: &[Value]) -> Value {
    let mut approvals = BTreeSet::new();
    if params["weakens_verification"].as_bool().unwrap_or(false) {
        approvals.insert("verification criteria change".to_owned());
    }
    if selected.iter().any(|run| verification_pass_rate(run) < 0.8) {
        approvals.insert("procedure change".to_owned());
    }
    Value::Array(approvals.into_iter().map(Value::String).collect())
}

fn choose_comparison_pair<'a>(params: &Value, selected: &'a [Value]) -> Option<(&'a Value, &'a Value)> {
    let before_run_id = params
        .get("before_run_id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty());
    let after_run_id = params
        .get("after_run_id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty());

    if let (Some(before_run_id), Some(after_run_id)) = (before_run_id, after_run_id) {
        let before = selected
            .iter()
            .find(|run| run["id"].as_str() == Some(before_run_id))?;
        let after = selected
            .iter()
            .find(|run| run["id"].as_str() == Some(after_run_id))?;
        return Some((before, after));
    }

    if selected.len() < 2 {
        return None;
    }

    let before = selected
        .iter()
        .min_by(|left, right| regression_order_key(left).cmp(&regression_order_key(right)))?;
    let after = selected
        .iter()
        .max_by(|left, right| regression_order_key(left).cmp(&regression_order_key(right)))?;
    Some((before, after))
}

fn summarize_run_by_status(selected: &[Value], target_status: &str) -> Value {
    let matched = if target_status == "failed" {
        selected
            .iter()
            .find(|run| run["status"] == "failed" || run["status"] == "error")
    } else {
        selected
            .iter()
            .rev()
            .find(|run| run["status"] == target_status)
    };

    matched.map_or_else(
        || json!({ "tokens": 0, "cost": 0.0 }),
        |run| json!({
            "tokens": total_tokens(run),
            "cost": total_cost(run),
        }),
    )
}

fn summarize_retry_extra(before: &Value, after: &Value) -> Value {
    json!({
        "tokens": (total_tokens(after) - total_tokens(before)).max(0),
        "cost": (total_cost(after) - total_cost(before)).max(0.0),
        "duration_ms": (total_duration_ms(after) - total_duration_ms(before)).max(0),
    })
}

fn summarize_quality(selected: &[Value]) -> Value {
    let mut low_cost = ModelQuality::default();
    let mut frontier = ModelQuality::default();

    for run in selected {
        let pass_rate = verification_pass_rate(run);
        let duration_ms = total_duration_ms(run);
        for model in run["token_usage"]["by_model"]
            .as_array()
            .into_iter()
            .flatten()
        {
            let name = model["model"].as_str().unwrap_or_default();
            let cost = model["cost"].as_f64().unwrap_or(0.0);
            let bucket = if name.contains("mini") || cost <= 0.1 {
                &mut low_cost
            } else {
                &mut frontier
            };
            bucket.pass_rates.push(pass_rate);
            bucket.cost += cost;
            bucket.duration_ms += duration_ms;
        }
    }

    json!({
        "low_cost_model": low_cost.to_value(),
        "frontier_model": frontier.to_value(),
    })
}

#[derive(Default)]
struct ModelQuality {
    pass_rates: Vec<f64>,
    cost: f64,
    duration_ms: i64,
}

impl ModelQuality {
    fn to_value(&self) -> Value {
        json!({
            "verification_pass_rate": if self.pass_rates.is_empty() {
                0.0
            } else {
                self.pass_rates.iter().sum::<f64>() / self.pass_rates.len() as f64
            },
            "schema_compliant": true,
            "cost": self.cost,
            "duration_ms": self.duration_ms,
        })
    }
}

fn derive_main_issue(analysis: &Value) -> Value {
    let issue = analysis["bottlenecks"]["token_waste"]
        .as_array()
        .and_then(|steps| steps.first())
        .and_then(Value::as_str)
        .map(|step_id| format!("{} step used most tokens", human_step_label(step_id)))
        .unwrap_or_else(|| "insufficient run history to identify a main issue".to_owned());
    Value::String(issue)
}

fn measurement_provenance(selected: &[Value]) -> (Value, Value) {
    let modes = selected
        .iter()
        .filter_map(|run| run["measurement_mode"].as_str())
        .collect::<BTreeSet<_>>();

    if modes.is_empty() {
        return (json!("unknown"), json!("Measurement provenance unavailable."));
    }

    if modes.len() == 1 {
        let mode = modes.iter().next().copied().unwrap_or("unknown");
        let note = selected
            .iter()
            .find_map(|run| run["measurement_note"].as_str())
            .unwrap_or_else(|| default_measurement_note(mode));
        return (json!(mode), json!(note));
    }

    (
        json!("mixed"),
        json!("Mixed measurement provenance: includes recorded and derived optimization evidence."),
    )
}

fn default_measurement_note(mode: &str) -> &'static str {
    match mode {
        "derived" => "Estimated from workflow events and workflow definition.",
        "reported" => "Recorded optimization metrics.",
        _ => "Measurement provenance unavailable.",
    }
}

fn collect_step_stats(selected: &[Value]) -> BTreeMap<String, StepStats> {
    let mut step_stats = BTreeMap::new();
    for run in selected {
        for step in run["token_usage"]["by_step"]
            .as_array()
            .into_iter()
            .flatten()
        {
            let step_id = step["step_id"].as_str().unwrap_or("unknown").to_owned();
            let entry = step_stats.entry(step_id).or_insert_with(StepStats::default);
            entry.tokens += step["tokens"].as_i64().unwrap_or(0);
            entry.occurrences += 1;
            if step["over_budget"].as_bool().unwrap_or(false) {
                entry.over_budget_count += 1;
            }
            entry.max_over_ratio = entry
                .max_over_ratio
                .max(step["over_ratio"].as_f64().unwrap_or(0.0));
        }

        for step in run["time"]["by_step"]
            .as_array()
            .into_iter()
            .flatten()
        {
            let step_id = step["step_id"].as_str().unwrap_or("unknown").to_owned();
            let entry = step_stats.entry(step_id).or_insert_with(StepStats::default);
            entry.duration_ms += step["duration_ms"].as_i64().unwrap_or(0);
            entry.retries += step["retries"].as_i64().unwrap_or(0);
            entry.occurrences += 1;
        }
    }
    step_stats
}

fn ranked_step_ids(
    step_stats: &BTreeMap<String, StepStats>,
    score: impl Fn(&StepStats) -> f64,
) -> Value {
    let mut ranked = step_stats
        .iter()
        .map(|(step_id, stats)| (step_id.clone(), score(stats)))
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.0.cmp(&right.0))
    });

    Value::Array(
        ranked
            .into_iter()
            .filter(|(_, score)| *score > 0.0)
            .map(|(step_id, _)| Value::String(step_id))
            .take(2)
            .collect(),
    )
}

fn repeated_discovery_detected(selected: &[Value]) -> bool {
    let rg_count = selected
        .iter()
        .flat_map(|run| run["tool_calls"].as_array().into_iter().flatten())
        .filter(|call| call["tool"] == "rg")
        .map(|call| call["count"].as_i64().unwrap_or(1))
        .sum::<i64>();

    let repeated_sources = selected
        .iter()
        .flat_map(|run| run["context_sources"].as_array().into_iter().flatten())
        .filter_map(|source| source["source"].as_str())
        .collect::<Vec<_>>();

    rg_count >= 2 || repeated_sources.len() >= 2
}

fn run_ids(selected: &[Value]) -> Vec<Value> {
    selected
        .iter()
        .filter_map(|run| run["id"].as_str())
        .map(|run_id| Value::String(run_id.to_owned()))
        .collect()
}

fn dominant_step_id(run: &Value) -> Value {
    run["token_usage"]["by_step"]
        .as_array()
        .and_then(|steps| {
            steps.iter().max_by_key(|step| step["tokens"].as_i64().unwrap_or(0))
        })
        .and_then(|step| step["step_id"].as_str())
        .map(|step_id| Value::String(step_id.to_owned()))
        .unwrap_or(Value::Null)
}

fn total_tokens(run: &Value) -> i64 {
    run["token_usage"]["total"].as_i64().unwrap_or(0)
}

fn total_duration_ms(run: &Value) -> i64 {
    run["time"]["total_ms"].as_i64().unwrap_or(0)
}

fn verification_pass_rate(run: &Value) -> f64 {
    run["verification"]["pass_rate"].as_f64().unwrap_or(0.0)
}

fn total_cost(run: &Value) -> f64 {
    run["token_usage"]["by_model"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|model| model["cost"].as_f64().unwrap_or(0.0))
        .sum()
}

fn average(total: i64, count: i64) -> i64 {
    if count <= 0 {
        0
    } else {
        total / count
    }
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn estimated_plan_token_reduction(selected: &[Value]) -> i64 {
    let plan_tokens = selected
        .iter()
        .flat_map(|run| run["token_usage"]["by_step"].as_array().into_iter().flatten())
        .find(|step| step["step_id"] == "plan")
        .and_then(|step| step["tokens"].as_i64())
        .unwrap_or(800);
    (plan_tokens / 2).max(200)
}

fn is_high_failure_cost_step(selected: &[Value], step_id: &str) -> bool {
    selected.iter().any(|run| {
        run["model_calls"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|call| {
                call["step_id"].as_str() == Some(step_id)
                    && call["reason"]
                        .as_str()
                        .map(|reason| reason.contains("high-risk") || reason.contains("high failure"))
                        .unwrap_or(false)
            })
    }) || step_id == "implement"
}

fn step_token_overage(selected: &[Value], step_id: &str) -> i64 {
    selected
        .iter()
        .flat_map(|run| run["token_usage"]["by_step"].as_array().into_iter().flatten())
        .filter(|step| step["step_id"].as_str() == Some(step_id))
        .map(|step| step["over_by"].as_i64().unwrap_or(0))
        .sum::<i64>()
        .max(200)
}

fn human_step_label(step_id: &str) -> String {
    match step_id {
        "implement" => "implementation".to_owned(),
        "plan" => "planning".to_owned(),
        "review" => "review".to_owned(),
        _ => step_id
            .split(['-', '_'])
            .filter(|segment| !segment.is_empty())
            .map(|segment| {
                let mut chars = segment.chars();
                match chars.next() {
                    Some(first) => {
                        let mut word = first.to_uppercase().to_string();
                        word.push_str(chars.as_str());
                        word
                    }
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" "),
    }
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
        let run = super::record_run(
            &json!({ "run_id": "analysis-default", "workflow_id": "wf-main" }),
            "2026-05-17T00:00:00Z",
        );
        let analysis = super::analyze(&json!({ "workflow_id": "wf-main" }), &[run]);

        assert!(analysis["candidates"].as_array().unwrap().len() >= 3);
        assert_eq!(analysis["adaptive_compute"]["verifiability"], "testable");
    }

    #[test]
    fn analyze_changes_when_run_history_changes() {
        let stable = vec![json!({
            "id": "run-a",
            "workflow_id": "wf-main",
            "status": "completed",
            "token_usage": {
                "total": 1200,
                "by_step": [
                    { "step_id": "plan", "tokens": 400, "budget": 1000, "over_budget": false, "over_by": 0, "over_ratio": 0.0 }
                ],
                "by_model": [
                    { "model": "gpt-5.4-mini", "tokens": 1200, "cost": 0.02 }
                ],
                "hotspots": ["plan"]
            },
            "verification": {
                "pass_rate": 0.95,
                "requirements_satisfied": 0.95,
                "items": [],
                "missing": [],
                "risks": []
            },
            "time": {
                "total_ms": 500,
                "by_step": [{ "step_id": "plan", "duration_ms": 500, "retries": 0 }],
                "review_ms": 0
            },
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
                    "by_model": [
                        { "model": "gpt-5.5", "tokens": 4200, "cost": 0.42 }
                    ],
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
                "model_calls": [
                    { "step_id": "implement", "model": "gpt-5.5", "tokens": 4200, "cost": 0.42, "reason": "high-risk implementation" }
                ],
                "tool_calls": [{ "tool": "rg", "count": 3 }],
                "context_sources": [{ "source": "docs/wiki/목차.md", "summary": "wiki navigation" }],
                "created_at": "2026-05-19T01:00:00Z"
            })
        ];

        let stable_analysis = super::analyze(&json!({ "workflow_id": "wf-main" }), &stable);
        let regressed_analysis =
            super::analyze(&json!({ "workflow_id": "wf-main" }), &regressed);

        assert_ne!(stable_analysis["bottlenecks"], regressed_analysis["bottlenecks"]);
        assert_eq!(regressed_analysis["regression"]["possible"], true);
    }

    #[test]
    fn comparison_keeps_quality_metrics() {
        let before = json!({
            "id": "before",
            "workflow_id": "wf-main",
            "status": "failed",
            "token_usage": {
                "total": 4200,
                "by_step": [{ "step_id": "implement", "tokens": 4200, "budget": 3000, "over_budget": true, "over_by": 1200, "over_ratio": 0.4 }],
                "by_model": [{ "model": "gpt-5.5", "tokens": 4200, "cost": 0.42 }],
                "hotspots": ["implement"]
            },
            "verification": { "pass_rate": 0.8, "requirements_satisfied": 0.8, "items": [], "missing": [], "risks": [] },
            "time": { "total_ms": 2200, "by_step": [{ "step_id": "implement", "duration_ms": 2200, "retries": 1 }], "review_ms": 0 },
            "created_at": "2026-05-19T00:00:00Z"
        });
        let after = json!({
            "id": "after",
            "workflow_id": "wf-main",
            "status": "completed",
            "token_usage": {
                "total": 3000,
                "by_step": [{ "step_id": "implement", "tokens": 3000, "budget": 3000, "over_budget": false, "over_by": 0, "over_ratio": 0.0 }],
                "by_model": [
                    { "model": "gpt-5.4-mini", "tokens": 1000, "cost": 0.05 },
                    { "model": "gpt-5.5", "tokens": 2000, "cost": 0.26 }
                ],
                "hotspots": ["implement"]
            },
            "verification": { "pass_rate": 0.92, "requirements_satisfied": 0.92, "items": [], "missing": [], "risks": [] },
            "time": { "total_ms": 700, "by_step": [{ "step_id": "implement", "duration_ms": 700, "retries": 0 }], "review_ms": 0 },
            "created_at": "2026-05-19T01:00:00Z"
        });
        let comparison = super::compare_runs(
            &json!({
                "workflow_id": "wf-main",
                "before_run_id": "before",
                "after_run_id": "after"
            }),
            &[before, after],
        );

        assert_eq!(comparison["token_delta"], -1200);
        assert!(comparison["quality"]["frontier_model"]["cost"].is_number());
    }

    #[test]
    fn main_issue_uses_humanized_step_label_for_unknown_step_ids() {
        let issue = super::derive_main_issue(&json!({
            "bottlenecks": {
                "token_waste": ["digest"]
            }
        }));

        assert_eq!(issue, "Digest step used most tokens");
    }

    #[test]
    fn suggestions_follow_actual_bottleneck_instead_of_generic_file_discovery() {
        let analysis = super::analyze(
            &json!({ "workflow_id": "demo-workflow" }),
            &[json!({
                "id": "run-demo",
                "workflow_id": "demo-workflow",
                "status": "completed",
                "token_usage": {
                    "total": 2400,
                    "by_step": [
                        { "step_id": "brief", "tokens": 600, "budget": 400, "over_budget": true, "over_by": 200, "over_ratio": 0.5 },
                        { "step_id": "digest", "tokens": 1800, "budget": 700, "over_budget": true, "over_by": 1100, "over_ratio": 1.57 }
                    ],
                    "by_model": [],
                    "hotspots": ["digest"]
                },
                "verification": {
                    "pass_rate": 0.95,
                    "requirements_satisfied": 0.95,
                    "items": [],
                    "missing": [],
                    "risks": []
                },
                "time": {
                    "total_ms": 700,
                    "by_step": [{ "step_id": "digest", "duration_ms": 700, "retries": 0 }],
                    "review_ms": 0
                },
                "model_calls": [],
                "tool_calls": [],
                "context_sources": [],
                "measurement_mode": "derived",
                "measurement_note": "Estimated from workflow events.",
                "created_at": "2026-05-19T00:00:00Z"
            })],
        );

        let suggestions = analysis["suggestions"].as_array().unwrap();
        assert!(suggestions.iter().any(|value| value.as_str().is_some_and(|text| text.contains("Digest"))));
        assert!(!suggestions.iter().any(|value| value == "preprocess file discovery"));
    }
}
