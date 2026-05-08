use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver};

use manual_worflow::{WorkflowDefinition, WorkflowRun};
use serde::Deserialize;
use serde_json::{Value, json};

pub fn crate_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

#[derive(Default)]
pub struct AppServer {
    state: Mutex<ServerState>,
}

impl AppServer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handle_json(&self, input: &str) -> String {
        let response = self.handle_json_value(input);
        response.to_string()
    }

    pub fn subscribe_run(&self, run_id: &str) -> Option<Receiver<Value>> {
        let state = self
            .state
            .lock()
            .expect("server state lock should not poison");
        let run = state.runs.get(run_id)?;
        let (sender, receiver) = mpsc::channel();

        for event in run.events() {
            sender
                .send(event.clone())
                .expect("receiver should stay alive while replaying events");
        }

        Some(receiver)
    }

    fn handle_json_value(&self, input: &str) -> Value {
        let request = match serde_json::from_str::<RpcRequest>(input) {
            Ok(request) => request,
            Err(error) => return rpc_error(Value::Null, -32700, error.to_string()),
        };

        match request.method.as_str() {
            "workflow.create" => self.create_workflow(request.id, request.params),
            "workflow.start" => self.start_workflow(request.id, request.params),
            "workflow.events" => self.workflow_events(request.id, request.params),
            _ => rpc_error(request.id, -32601, "method not found"),
        }
    }

    fn create_workflow(&self, id: Value, params: Value) -> Value {
        let params = match serde_json::from_value::<CreateWorkflowParams>(params) {
            Ok(params) => params,
            Err(error) => return rpc_error(id, -32602, error.to_string()),
        };

        let node_count = params.workflow.nodes.len();
        let workflow_id = params.workflow.id.clone();

        let mut state = self
            .state
            .lock()
            .expect("server state lock should not poison");
        state.workflows.insert(workflow_id.clone(), params.workflow);

        rpc_result(
            id,
            json!({
                "workflow_id": workflow_id,
                "node_count": node_count,
            }),
        )
    }

    fn start_workflow(&self, id: Value, params: Value) -> Value {
        let params = match serde_json::from_value::<StartWorkflowParams>(params) {
            Ok(params) => params,
            Err(error) => return rpc_error(id, -32602, error.to_string()),
        };

        let mut state = self
            .state
            .lock()
            .expect("server state lock should not poison");
        let workflow = match state.workflows.get(&params.workflow_id).cloned() {
            Some(workflow) => workflow,
            None => return rpc_error(id, -32000, "workflow not found"),
        };

        state.next_run_number += 1;
        let run_id = format!("run-{}", state.next_run_number);
        let run = workflow
            .execute(&run_id)
            .expect("workflow definitions should be executable");
        state.runs.insert(run_id.clone(), run);

        rpc_result(id, json!({ "run_id": run_id }))
    }

    fn workflow_events(&self, id: Value, params: Value) -> Value {
        let params = match serde_json::from_value::<WorkflowEventsParams>(params) {
            Ok(params) => params,
            Err(error) => return rpc_error(id, -32602, error.to_string()),
        };

        let state = self
            .state
            .lock()
            .expect("server state lock should not poison");
        let run = match state.runs.get(&params.run_id) {
            Some(run) => run,
            None => return rpc_error(id, -32001, "run not found"),
        };

        let events = run
            .events()
            .iter()
            .skip(params.cursor)
            .cloned()
            .collect::<Vec<_>>();

        rpc_result(
            id,
            json!({
                "events": events,
                "next_cursor": run.events().len(),
                "completed": run.completed(),
            }),
        )
    }
}

#[derive(Default)]
struct ServerState {
    workflows: BTreeMap<String, WorkflowDefinition>,
    runs: BTreeMap<String, WorkflowRun>,
    next_run_number: u64,
}

#[derive(Deserialize)]
struct RpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Value,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Deserialize)]
struct CreateWorkflowParams {
    workflow: WorkflowDefinition,
}

#[derive(Deserialize)]
struct StartWorkflowParams {
    workflow_id: String,
}

#[derive(Deserialize)]
struct WorkflowEventsParams {
    run_id: String,
    #[serde(default)]
    cursor: usize,
}

fn rpc_result(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
}

fn rpc_error(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message.into(),
        },
    })
}
