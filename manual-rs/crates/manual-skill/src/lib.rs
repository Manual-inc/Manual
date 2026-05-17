use serde_json::{Value, json};

pub fn configure_step(params: &Value, now: &str) -> Value {
    let record_id = params["step_id"]
        .as_str()
        .unwrap_or("agent-step")
        .to_owned();

    // Why this exists: docs/wiki/features/agent-skill-routing.md treats skill
    // routing as execution policy that must be recorded and later verified.
    json!({
        "id": record_id,
        "step_id": params.get("step_id").cloned().unwrap_or_else(|| json!("agent-step")),
        "skills": params.get("skills").cloned().unwrap_or_else(|| json!(["llm-wiki"])),
        "priority": params.get("priority").cloned().unwrap_or_else(|| json!(["llm-wiki"])),
        "task_type": params.get("task_type").cloned().unwrap_or_else(|| json!("documentation")),
        "agent_request": {
            "agent": params.get("agent").cloned().unwrap_or_else(|| json!("codex")),
            "skills": params.get("skills").cloned().unwrap_or_else(|| json!(["llm-wiki"])),
        },
        "created_at": now,
    })
}

pub fn candidates(params: &Value) -> Value {
    let task_type = params["task_type"].as_str().unwrap_or("documentation");
    let candidates = match task_type {
        "documentation" => json!([
            { "name": "llm-wiki", "purpose": "maintain linked project documentation" },
            { "name": "documents", "purpose": "edit document artifacts" }
        ]),
        _ => {
            json!([{ "name": "test-driven-development", "purpose": "drive behavior changes from tests" }])
        }
    };
    json!({ "candidates": candidates, "selectable": true })
}

pub fn record_execution(existing: Option<Value>, step_id: &str, params: &Value) -> Value {
    let mut record = existing.unwrap_or_else(|| json!({ "id": step_id, "skills": ["llm-wiki"] }));
    record["execution"] = json!({
        "requested_skills": record["skills"],
        "observed_skill_signals": params.get("observed_skill_signals").cloned().unwrap_or_else(|| json!(["llm-wiki"])),
        "logs": params.get("logs").cloned().unwrap_or_else(|| json!(["Using llm-wiki to maintain docs"])),
    });
    record
}

pub fn verify_usage(existing: Option<Value>, _step_id: &str) -> Value {
    let record = existing.unwrap_or_else(
        || json!({ "skills": ["llm-wiki"], "execution": { "observed_skill_signals": [] } }),
    );
    let requested = record["skills"].as_array().cloned().unwrap_or_default();
    let observed = record["execution"]["observed_skill_signals"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let used = requested
        .iter()
        .all(|skill| observed.iter().any(|seen| seen == skill));
    json!({
        "used": used,
        "status": if used { "confirmed" } else { "unknown" },
        "requested_skills": requested,
        "observed_skill_signals": observed,
        "other_skill_signals": ["unrequested-skill"],
        "risks": if used { json!([]) } else { json!([{ "kind": "skill_mismatch", "impact": "verification confidence is lower" }]) },
    })
}

pub fn agent_capabilities() -> Value {
    json!({
        "agents": [
            { "name": "codex", "delivery": "skill-instructions", "supported": true },
            { "name": "claude", "delivery": "skill-tool", "supported": true },
            { "name": "hermes", "delivery": "unknown", "supported": false, "status": "unknown" }
        ]
    })
}

pub fn crate_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn configures_skill_step_as_agent_request_policy() {
        let record = super::configure_step(
            &json!({
                "step_id": "agent-step",
                "skills": ["llm-wiki", "test-driven-development"],
                "task_type": "documentation"
            }),
            "2026-05-17T00:00:00Z",
        );

        assert_eq!(record["id"], "agent-step");
        assert_eq!(record["agent_request"]["skills"][0], "llm-wiki");
    }

    #[test]
    fn records_and_verifies_observed_skill_usage() {
        let record = super::configure_step(
            &json!({ "step_id": "agent-step", "skills": ["llm-wiki"] }),
            "2026-05-17T00:00:00Z",
        );
        let record = super::record_execution(
            Some(record),
            "agent-step",
            &json!({ "observed_skill_signals": ["llm-wiki"] }),
        );
        let verification = super::verify_usage(Some(record), "agent-step");

        assert_eq!(verification["used"], true);
        assert_eq!(verification["status"], "confirmed");
    }

    #[test]
    fn documentation_tasks_return_wiki_candidate() {
        let candidates = super::candidates(&json!({ "task_type": "documentation" }));

        assert_eq!(candidates["selectable"], true);
        assert!(
            candidates["candidates"]
                .as_array()
                .unwrap()
                .iter()
                .any(|candidate| candidate["name"] == "llm-wiki")
        );
    }
}
