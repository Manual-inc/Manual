use manual_worflow::{DependencyDefinition, NodeDefinition, NodeKind, WorkflowDefinition};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const STORYBOOK_INPUT_NODE_ID: &str = "__storybook_input__";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NodeTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub node: NodeDefinition,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NodeRun {
    pub id: String,
    pub node: NodeDefinition,
    pub inputs: Value,
    events: Vec<Value>,
    completed: bool,
    pub started_at: String,
}

impl NodeRun {
    pub fn pending(id: String, node: NodeDefinition, inputs: Value) -> Self {
        Self {
            id,
            node,
            inputs,
            events: Vec::new(),
            completed: false,
            started_at: iso_timestamp(),
        }
    }

    pub fn events(&self) -> &[Value] {
        &self.events
    }

    pub fn completed(&self) -> bool {
        self.completed
    }

    pub fn record_event(&mut self, event: Value) {
        if event["type"] == "workflow_completed" || event["type"] == "workflow_failed" {
            self.completed = true;
        }
        self.events.push(event);
    }
}

pub fn node_schema(kind: &NodeKind) -> Value {
    match kind {
        NodeKind::Claude => json!({
            "kind": "claude",
            "inputs": [
                { "name": "prompt", "type": "string", "required": true, "description": "Prompt for the Claude agent" },
                { "name": "model", "type": "string", "required": false, "description": "Model to use (e.g. claude-opus-4-5)" },
                { "name": "cwd", "type": "string", "required": false, "description": "Working directory for the agent process" },
                { "name": "extra_args", "type": "array", "required": false, "description": "Extra CLI arguments" },
            ],
            "output_description": "Object with status_code, stdout, and stderr fields from the Claude CLI",
        }),
        NodeKind::Codex => json!({
            "kind": "codex",
            "inputs": [
                { "name": "prompt", "type": "string", "required": true, "description": "Prompt for the Codex agent" },
                { "name": "model", "type": "string", "required": false, "description": "Model to use" },
                { "name": "cwd", "type": "string", "required": false, "description": "Working directory for the agent process" },
                { "name": "extra_args", "type": "array", "required": false, "description": "Extra CLI arguments" },
            ],
            "output_description": "Object with status_code, stdout, and stderr fields from the Codex CLI",
        }),
        NodeKind::Pi => json!({
            "kind": "pi",
            "inputs": [
                { "name": "prompt", "type": "string", "required": true, "description": "Prompt for the Pi agent" },
                { "name": "model", "type": "string", "required": false, "description": "Model to use" },
                { "name": "cwd", "type": "string", "required": false, "description": "Working directory for the agent process" },
                { "name": "extra_args", "type": "array", "required": false, "description": "Extra CLI arguments" },
            ],
            "output_description": "Object with status_code, stdout, and stderr fields from the Pi CLI",
        }),
        NodeKind::Constant => json!({
            "kind": "constant",
            "inputs": [
                { "name": "value", "type": "any", "required": true, "description": "The constant value to output (any JSON value)" },
            ],
            "output_description": "The constant value as-is",
        }),
        NodeKind::Template => json!({
            "kind": "template",
            "inputs": [
                { "name": "template", "type": "string", "required": true, "description": "Template string using {{node_id}} or {{node_id.field}} placeholders" },
            ],
            "output_description": "Rendered string with all placeholders replaced by upstream node outputs",
        }),
        NodeKind::Delay => json!({
            "kind": "delay",
            "inputs": [
                { "name": "duration_ms", "type": "number", "required": true, "description": "Delay duration in milliseconds" },
            ],
            "output_description": "null after the delay completes",
        }),
        NodeKind::Fail => json!({
            "kind": "fail",
            "inputs": [
                { "name": "error", "type": "string", "required": false, "description": "Error message", "default": "node execution failed" },
            ],
            "output_description": "Always fails with the provided error message",
        }),
        NodeKind::Script => json!({
            "kind": "script",
            "inputs": [
                { "name": "script", "type": "string", "required": true, "description": "Script path or shell snippet to run" },
                { "name": "sandbox_policy", "type": "object", "required": true, "description": "Sandbox policy applied to the script process" },
            ],
            "output_description": "Object with status_code, stdout, and stderr fields from the sandboxed script process",
        }),
    }
}

pub fn node_run_summary(run_id: &str, run: &NodeRun) -> Value {
    let mut status = "pending";
    let mut result = Value::Null;
    let mut error = Value::Null;

    for event in run.events() {
        match event["type"].as_str() {
            Some("workflow_started") => {
                status = "running";
            }
            Some("workflow_completed") => {
                status = "completed";
            }
            Some("workflow_failed") => {
                status = "failed";
                error = event["error"].clone();
            }
            Some("node_completed") => {
                if event["node_id"].as_str() == Some(run.node.id.as_str()) {
                    result = event["result"].clone();
                    status = "completed";
                }
            }
            Some("node_failed") => {
                if event["node_id"].as_str() == Some(run.node.id.as_str()) {
                    error = event["error"].clone();
                    status = "failed";
                }
            }
            _ => {}
        }
    }

    json!({
        "run_id": run_id,
        "node_id": run.node.id,
        "status": status,
        "result": result,
        "error": error,
        "started_at": run.started_at,
    })
}

pub fn storybook_workflow(node: NodeDefinition, inputs: Value, run_id: &str) -> WorkflowDefinition {
    let node_id = node.id.clone();
    WorkflowDefinition {
        id: format!("__storybook__{run_id}"),
        nodes: vec![
            NodeDefinition {
                id: STORYBOOK_INPUT_NODE_ID.to_string(),
                kind: NodeKind::Constant,
                value: inputs,
                template: String::new(),
                duration_ms: 0,
                error: String::new(),
                prompt: String::new(),
                model: None,
                cwd: None,
                extra_args: Vec::new(),
                script: String::new(),
                sandbox_policy: Value::Null,
            },
            node,
        ],
        dependencies: vec![DependencyDefinition {
            node: node_id,
            depends_on: STORYBOOK_INPUT_NODE_ID.to_string(),
        }],
    }
}

pub fn node_run_result(run: &NodeRun) -> Value {
    run.events()
        .iter()
        .rev()
        .find(|event| {
            event["type"] == "node_completed" && event["node_id"].as_str() == Some(&run.node.id)
        })
        .map(|event| event["result"].clone())
        .unwrap_or(Value::Null)
}

pub fn create_test_case(
    case_id: String,
    run: &NodeRun,
    expected_output: Option<Value>,
    criteria: Option<Value>,
    now: &str,
) -> Value {
    // Why this exists: docs/wiki/features/node-storybook.md defines reusable node
    // examples as regression assets rather than mock-only UI fixtures.
    json!({
        "id": case_id,
        "node_id": run.node.id,
        "node": run.node,
        "inputs": run.inputs,
        "expected_output": expected_output.unwrap_or_else(|| node_run_result(run)),
        "criteria": criteria.unwrap_or_else(|| json!({
            "comparison": "json_equal",
            "schema_match_required": true,
        })),
        "created_at": now,
        "updated_at": now,
    })
}

pub fn verify_test_cases(cases: Vec<Value>, node_id: Option<&str>) -> Value {
    let mut results = Vec::new();
    for case in cases
        .into_iter()
        .filter(|case| node_id.is_none_or(|needle| case["node_id"] == needle))
    {
        let Some(node) = serde_json::from_value::<NodeDefinition>(case["node"].clone()).ok() else {
            continue;
        };
        let temp_workflow = storybook_workflow(node, case["inputs"].clone(), "verify");
        let mut events = Vec::new();
        let execution = temp_workflow.execute_with_events("verify", |event| events.push(event));
        let actual = events
            .iter()
            .rev()
            .find(|event| event["type"] == "node_completed" && event["node_id"] == case["node_id"])
            .map(|event| event["result"].clone())
            .unwrap_or(Value::Null);
        let passed = execution.is_ok() && actual == case["expected_output"];
        results.push(json!({
            "test_case_id": case["id"],
            "node_id": case["node_id"],
            "passed": passed,
            "expected_output": case["expected_output"],
            "actual_output": actual,
            "diff": if passed { Value::Null } else { json!({ "expected": case["expected_output"], "actual": actual }) },
        }));
    }

    let failed = results
        .iter()
        .filter(|result| result["passed"] == false)
        .cloned()
        .collect::<Vec<_>>();

    json!({ "results": results, "failed": failed })
}

pub fn compose_registry_candidate(template: &NodeTemplate) -> Value {
    json!({
        "candidate": template,
        "stage": {
            "node": template.node,
            "input_schema": node_schema(&template.node.kind)["inputs"],
            "output_schema": node_schema(&template.node.kind)["output_description"],
        },
        "unregistered_allowed": false,
    })
}

pub fn iso_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    let days = secs / 86400;
    let (year, month, day) = days_to_ymd(days);
    format!("{year:04}-{month:02}-{day:02}T{h:02}:{m:02}:{s:02}Z")
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut year = 1970u64;
    loop {
        let year_days = if is_leap_year(year) { 366 } else { 365 };
        if days < year_days {
            break;
        }
        days -= year_days;
        year += 1;
    }
    let month_days: &[u64] = if is_leap_year(year) {
        &[31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        &[31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1u64;
    for &md in month_days {
        if days < md {
            break;
        }
        days -= md;
        month += 1;
    }
    (year, month, days + 1)
}

fn is_leap_year(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

pub fn crate_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

#[cfg(test)]
mod storybook_tests {
    use manual_worflow::{NodeDefinition, NodeKind};
    use serde_json::json;

    use crate::{NodeRun, NodeTemplate};

    fn constant_node() -> NodeDefinition {
        NodeDefinition {
            id: "digest".to_owned(),
            kind: NodeKind::Constant,
            value: json!("ok"),
            template: String::new(),
            duration_ms: 0,
            error: String::new(),
            prompt: String::new(),
            model: None,
            cwd: None,
            extra_args: Vec::new(),
            script: String::new(),
            sandbox_policy: serde_json::Value::Null,
        }
    }

    #[test]
    fn saves_node_run_as_reusable_test_case() {
        let mut run = NodeRun::pending("node-run-1".to_owned(), constant_node(), json!({}));
        run.record_event(json!({
            "type": "node_completed",
            "node_id": "digest",
            "result": "ok"
        }));

        let case = super::create_test_case(
            "node-case-1".to_owned(),
            &run,
            None,
            None,
            "2026-05-17T00:00:00Z",
        );

        assert_eq!(case["id"], "node-case-1");
        assert_eq!(case["expected_output"], "ok");
        assert_eq!(case["criteria"]["comparison"], "json_equal");
    }

    #[test]
    fn verifies_saved_storybook_cases() {
        let case = json!({
            "id": "node-case-1",
            "node_id": "digest",
            "node": constant_node(),
            "inputs": {},
            "expected_output": "ok",
            "criteria": { "comparison": "json_equal" }
        });

        let report = super::verify_test_cases(vec![case], Some("digest"));

        assert_eq!(report["failed"].as_array().unwrap().len(), 0);
        assert_eq!(report["results"][0]["passed"], true);
    }

    #[test]
    fn composes_registry_template_candidate() {
        let template = NodeTemplate {
            id: "digest-template".to_owned(),
            name: "Digest".to_owned(),
            description: "Summarize input".to_owned(),
            node: constant_node(),
            created_at: "2026-05-17T00:00:00Z".to_owned(),
            updated_at: "2026-05-17T00:00:00Z".to_owned(),
        };

        let candidate = super::compose_registry_candidate(&template);

        assert_eq!(candidate["unregistered_allowed"], false);
        assert!(candidate["stage"]["input_schema"].is_array());
    }
}
