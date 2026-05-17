use serde_json::{Value, json};

pub fn create_manual(manual_id: String, params: &Value, now: &str) -> Result<Value, &'static str> {
    let name = params["name"].as_str().unwrap_or_default().trim();
    if name.is_empty() {
        return Err("manual name is required");
    }

    // Why this exists: docs/wiki/systems/매뉴얼-시스템.md makes manuals the
    // local-first unit that binds workflow, execution policy, and evidence.
    Ok(json!({
        "id": manual_id,
        "name": name,
        "purpose": params.get("purpose").cloned().unwrap_or_else(|| json!("Reusable AI work unit")),
        "description": params.get("description").cloned().unwrap_or_else(|| json!("")),
        "tags": params.get("tags").cloned().unwrap_or_else(|| json!(["mvp"])),
        "status": params.get("status").cloned().unwrap_or_else(|| json!("draft")),
        "default_agent": params.get("default_agent").cloned().unwrap_or_else(|| json!("codex")),
        "execution_mode": params.get("execution_mode").cloned().unwrap_or_else(|| json!("single_agent")),
        "model": params.get("model").cloned().unwrap_or_else(|| json!({ "provider": "local", "name": "default" })),
        "workflow_steps": params.get("workflow_steps").cloned().unwrap_or_else(default_steps),
        "optimization": params.get("optimization").cloned().unwrap_or_else(default_optimization),
        "created_at": now,
        "updated_at": now,
        "current_version": 1,
        "versions": [{
            "version": 1,
            "created_at": now,
            "summary": "initial draft"
        }],
        "change_history": [{
            "at": now,
            "change": "manual_created"
        }],
        "recent_runs": [],
        "deleted": false,
        "running": params.get("running").cloned().unwrap_or(Value::Bool(false)),
    }))
}

pub fn update_manual(
    mut manual: Value,
    changes: &Value,
    execution_affecting: bool,
    now: &str,
) -> Value {
    merge_object(&mut manual, changes);
    let execution_affecting = execution_affecting
        || changes.get("workflow_steps").is_some()
        || changes.get("sandbox_policy").is_some()
        || changes.get("verification_policy").is_some()
        || changes.get("model").is_some();

    manual["updated_at"] = json!(now);
    push_json_array(
        &mut manual["change_history"],
        json!({ "at": now, "change": "manual_updated", "execution_affecting": execution_affecting }),
    );
    if execution_affecting {
        let version = manual["current_version"].as_u64().unwrap_or(1) + 1;
        manual["current_version"] = json!(version);
        push_json_array(
            &mut manual["versions"],
            json!({ "version": version, "created_at": now, "summary": "execution-affecting update" }),
        );
        manual["last_diff"] = json!({
            "before": "previous version",
            "after": "updated execution policy",
        });
    }

    manual
}

pub fn clone_manual(mut source: Value, clone_id: String, now: &str) -> Value {
    source["id"] = json!(clone_id);
    source["name"] = json!(format!(
        "{} copy",
        source["name"].as_str().unwrap_or("Manual")
    ));
    source["status"] = json!("draft");
    source["created_at"] = json!(now);
    source["updated_at"] = json!(now);
    source["recent_runs"] = json!([]);
    source["change_history"] = json!([{ "at": now, "change": "manual_cloned" }]);
    source
}

pub fn set_status(mut manual: Value, status: &str, now: &str) -> Value {
    manual["status"] = json!(status);
    manual["updated_at"] = json!(now);
    manual
}

pub fn mark_deleted(mut manual: Value, now: &str) -> Result<Value, &'static str> {
    if manual["running"].as_bool().unwrap_or(false) {
        return Err("running manual cannot be deleted");
    }
    manual["deleted"] = json!(true);
    push_json_array(
        &mut manual["change_history"],
        json!({ "at": now, "change": "manual_deleted" }),
    );
    Ok(manual)
}

pub fn validate_for_activation(manual: &Value) -> Value {
    let mut missing = Vec::new();
    if manual["name"]
        .as_str()
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        missing.push(json!({ "field": "name", "message": "name is required" }));
    }
    if manual["default_agent"]
        .as_str()
        .unwrap_or_default()
        .is_empty()
    {
        missing.push(json!({ "field": "default_agent", "message": "local agent installation or path configuration is required" }));
    }
    for step in manual["workflow_steps"].as_array().into_iter().flatten() {
        if step["sandbox_policy"].is_null() {
            missing.push(json!({ "field": "sandbox_policy", "step_id": step["id"], "message": "sandbox policy is required" }));
        }
        if manual["optimization"]["token_measurement"] == true && step["token_budget"].is_null() {
            missing.push(json!({ "field": "token_budget", "step_id": step["id"], "message": "token budget is required" }));
        }
        if manual["optimization"]["token_measurement"] == true
            && step["verification_policy"].is_null()
        {
            missing.push(json!({ "field": "verification_policy", "step_id": step["id"], "message": "verification criteria are required" }));
        }
    }

    json!({
        "valid": missing.is_empty(),
        "missing": missing,
    })
}

pub fn versions_response(manual: &Value) -> Value {
    json!({
        "versions": manual["versions"],
        "current_version": manual["current_version"],
        "diff": manual.get("last_diff").cloned().unwrap_or_else(|| json!({
            "before": "version 1",
            "after": "current version",
        })),
    })
}

pub fn list_summary(manual: &Value) -> Value {
    json!({
        "id": manual["id"],
        "name": manual["name"],
        "status": manual["status"],
        "updated_at": manual["updated_at"],
        "current_version": manual["current_version"],
        "tags": manual["tags"],
    })
}

pub fn matches_filters(
    manual: &Value,
    status: Option<&str>,
    query: &str,
    tag: Option<&str>,
) -> bool {
    if manual["deleted"] == true || manual["status"] == "archived" {
        return false;
    }
    if status.is_some_and(|status| manual["status"] != status) {
        return false;
    }
    let query = query.to_lowercase();
    if !query.is_empty()
        && !manual["name"]
            .as_str()
            .unwrap_or_default()
            .to_lowercase()
            .contains(&query)
        && !manual["description"]
            .as_str()
            .unwrap_or_default()
            .to_lowercase()
            .contains(&query)
    {
        return false;
    }
    tag.is_none_or(|tag| {
        manual["tags"]
            .as_array()
            .is_some_and(|tags| tags.iter().any(|value| value.as_str() == Some(tag)))
    })
}

fn default_steps() -> Value {
    json!([
        {
            "id": "agent-step",
            "kind": "codex",
            "input_schema": [{ "name": "prompt", "required": true }],
            "output_schema": "agent result object",
            "verification_policy": { "required": true, "criteria": ["tests pass"] },
            "sandbox_policy": { "sandbox_id": "default" },
            "token_budget": 4000
        }
    ])
}

fn default_optimization() -> Value {
    json!({
        "token_measurement": true,
        "time_measurement": true,
        "verification_criteria": ["requirements", "tests"],
        "improvement_recommendations": true,
        "self_evolution_mode": "suggest"
    })
}

fn merge_object(target: &mut Value, changes: &Value) {
    let (Some(target), Some(changes)) = (target.as_object_mut(), changes.as_object()) else {
        return;
    };
    for (key, value) in changes {
        target.insert(key.clone(), value.clone());
    }
}

fn push_json_array(target: &mut Value, value: Value) {
    if !target.is_array() {
        *target = json!([]);
    }
    target
        .as_array_mut()
        .expect("target should be JSON array")
        .push(value);
}

pub fn crate_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn creates_manual_with_default_execution_policy() {
        let manual = super::create_manual(
            "manual-1".to_owned(),
            &json!({ "name": "Docs Writer" }),
            "2026-05-17T00:00:00Z",
        )
        .expect("manual should be created");

        assert_eq!(manual["id"], "manual-1");
        assert_eq!(manual["status"], "draft");
        assert!(manual["workflow_steps"][0]["sandbox_policy"].is_object());
        assert_eq!(manual["optimization"]["token_measurement"], true);
    }

    #[test]
    fn rejects_empty_manual_name() {
        let error = super::create_manual(
            "manual-1".to_owned(),
            &json!({ "name": " " }),
            "2026-05-17T00:00:00Z",
        )
        .expect_err("empty name should fail");

        assert_eq!(error, "manual name is required");
    }

    #[test]
    fn update_versions_execution_affecting_changes() {
        let mut manual = super::create_manual(
            "manual-1".to_owned(),
            &json!({ "name": "Docs Writer" }),
            "2026-05-17T00:00:00Z",
        )
        .unwrap();

        manual = super::update_manual(
            manual,
            &json!({ "model": { "provider": "local", "name": "frontier" } }),
            false,
            "2026-05-17T00:01:00Z",
        );

        assert_eq!(manual["current_version"], 2);
        assert_eq!(manual["change_history"][1]["execution_affecting"], true);
    }

    #[test]
    fn activation_validation_reports_missing_execution_fields() {
        let manual = json!({
            "name": "",
            "default_agent": "",
            "workflow_steps": [{ "id": "step-1" }],
            "optimization": { "token_measurement": true }
        });

        let validation = super::validate_for_activation(&manual);

        assert_eq!(validation["valid"], false);
        assert!(
            validation["missing"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["field"] == "sandbox_policy")
        );
    }
}
