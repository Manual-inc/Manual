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
            },
            node,
        ],
        dependencies: vec![DependencyDefinition {
            node: node_id,
            depends_on: STORYBOOK_INPUT_NODE_ID.to_string(),
        }],
    }
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
