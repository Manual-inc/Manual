use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;

use manual_worflow::{WorkflowDefinition, WorkflowRun};
use serde::Deserialize;
use serde_json::{Map, Value, json};

mod workflow_store;

use workflow_store::{WorkflowStore, default_workflow_storage_dir};

pub fn crate_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

pub struct AppServer {
    state: Arc<Mutex<ServerState>>,
    workflow_store: WorkflowStore,
}

impl AppServer {
    pub fn new() -> Self {
        Self::with_storage_dir(default_workflow_storage_dir())
    }

    pub fn with_storage_dir(storage_dir: impl AsRef<Path>) -> Self {
        let workflow_store = WorkflowStore::new(storage_dir);
        let runs = workflow_store.load_runs();
        let state = ServerState {
            workflows: workflow_store.load_workflows(),
            next_run_number: next_run_number(&runs),
            runs,
            ..ServerState::default()
        };

        Self {
            state: Arc::new(Mutex::new(state)),
            workflow_store,
        }
    }

    pub fn handle_json(&self, input: &str) -> String {
        let response = self.handle_json_value(input);
        response.to_string()
    }

    fn handle_json_value(&self, input: &str) -> Value {
        let request = match serde_json::from_str::<RpcRequest>(input) {
            Ok(request) => request,
            Err(error) => return rpc_error(Value::Null, -32700, error.to_string()),
        };

        match request.method.as_str() {
            "workflow.create" => self.create_workflow(request.id, request.params),
            "workflow.get" => self.get_workflow(request.id, request.params),
            "workflow.list" => self.list_workflows(request.id),
            "workflow.update" => self.update_workflow(request.id, request.params),
            "workflow.delete" => self.delete_workflow(request.id, request.params),
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
        if let Err(error) = self.workflow_store.save_workflow(&params.workflow) {
            return rpc_error(id, -32002, error.to_string());
        }

        state.workflows.insert(workflow_id.clone(), params.workflow);

        rpc_result(
            id,
            json!({
                "workflow_id": workflow_id,
                "node_count": node_count,
            }),
        )
    }

    fn get_workflow(&self, id: Value, params: Value) -> Value {
        let params = match serde_json::from_value::<WorkflowIdParams>(params) {
            Ok(params) => params,
            Err(error) => return rpc_error(id, -32602, error.to_string()),
        };

        let state = self
            .state
            .lock()
            .expect("server state lock should not poison");
        let workflow = match state.workflows.get(&params.workflow_id) {
            Some(workflow) => workflow,
            None => return rpc_error(id, -32000, "workflow not found"),
        };

        rpc_result(id, json!({ "workflow": workflow }))
    }

    fn list_workflows(&self, id: Value) -> Value {
        let state = self
            .state
            .lock()
            .expect("server state lock should not poison");
        let workflows = state
            .workflows
            .values()
            .map(|workflow| {
                json!({
                    "workflow_id": workflow.id,
                    "node_count": workflow.nodes.len(),
                })
            })
            .collect::<Vec<_>>();

        rpc_result(id, json!({ "workflows": workflows }))
    }

    fn update_workflow(&self, id: Value, params: Value) -> Value {
        let params = match serde_json::from_value::<UpdateWorkflowParams>(params) {
            Ok(params) => params,
            Err(error) => return rpc_error(id, -32602, error.to_string()),
        };

        if params.workflow_id != params.workflow.id {
            return rpc_error(id, -32602, "workflow_id must match workflow.id");
        }

        let node_count = params.workflow.nodes.len();
        let workflow_id = params.workflow_id.clone();

        let mut state = self
            .state
            .lock()
            .expect("server state lock should not poison");
        if !state.workflows.contains_key(&params.workflow_id) {
            return rpc_error(id, -32000, "workflow not found");
        }

        if let Err(error) = self.workflow_store.save_workflow(&params.workflow) {
            return rpc_error(id, -32002, error.to_string());
        }

        state
            .workflows
            .insert(params.workflow_id.clone(), params.workflow);

        rpc_result(
            id,
            json!({
                "workflow_id": workflow_id,
                "node_count": node_count,
            }),
        )
    }

    fn delete_workflow(&self, id: Value, params: Value) -> Value {
        let params = match serde_json::from_value::<WorkflowIdParams>(params) {
            Ok(params) => params,
            Err(error) => return rpc_error(id, -32602, error.to_string()),
        };

        let mut state = self
            .state
            .lock()
            .expect("server state lock should not poison");
        if !state.workflows.contains_key(&params.workflow_id) {
            return rpc_error(id, -32000, "workflow not found");
        }

        if let Err(error) = self.workflow_store.delete_workflow(&params.workflow_id) {
            return rpc_error(id, -32002, error.to_string());
        }

        state.workflows.remove(&params.workflow_id);

        rpc_result(
            id,
            json!({
                "workflow_id": params.workflow_id,
                "deleted": true,
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
        state.runs.insert(run_id.clone(), WorkflowRun::pending());
        if let Some(run) = state.runs.get(&run_id) {
            if let Err(error) = self.workflow_store.save_run(&run_id, run) {
                state.runs.remove(&run_id);
                return rpc_error(id, -32002, error.to_string());
            }
        }
        let state = Arc::clone(&self.state);
        let workflow_store = self.workflow_store.clone();
        let thread_run_id = run_id.clone();

        thread::spawn(move || {
            let result = workflow.execute_with_events(&thread_run_id, |event| {
                let mut state = state.lock().expect("server state lock should not poison");
                if let Some(run) = state.runs.get_mut(&thread_run_id) {
                    run.record_event(event);
                    if let Err(error) = workflow_store.save_run(&thread_run_id, run) {
                        eprintln!("failed to persist workflow run {thread_run_id}: {error}");
                    }
                }
            });

            if let Err(error) = result {
                let mut state = state.lock().expect("server state lock should not poison");
                if let Some(run) = state.runs.get_mut(&thread_run_id) {
                    if !run.completed() {
                        run.record_event(json!({
                            "run_id": thread_run_id,
                            "sequence": run.events().len(),
                            "type": "workflow_failed",
                            "error": format!("{error:?}"),
                        }));
                        if let Err(error) = workflow_store.save_run(&thread_run_id, run) {
                            eprintln!("failed to persist workflow run {thread_run_id}: {error}");
                        }
                    }
                }
            }
        });

        rpc_result(id, json!({ "run_id": run_id }))
    }

    fn workflow_events(&self, id: Value, params: Value) -> Value {
        let params = match serde_json::from_value::<WorkflowEventsParams>(params) {
            Ok(params) => params,
            Err(error) => return rpc_error(id, -32602, error.to_string()),
        };

        let mut state = self
            .state
            .lock()
            .expect("server state lock should not poison");
        if let Some(stored_run) = self.workflow_store.load_run(&params.run_id) {
            state.runs.insert(params.run_id.clone(), stored_run);
        }

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
                "run": run_summary(&params.run_id, run),
            }),
        )
    }
}

impl Default for AppServer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default)]
struct ServerState {
    workflows: BTreeMap<String, WorkflowDefinition>,
    runs: BTreeMap<String, WorkflowRun>,
    next_run_number: u64,
}

fn next_run_number(runs: &BTreeMap<String, WorkflowRun>) -> u64 {
    runs.keys()
        .filter_map(|run_id| run_id.strip_prefix("run-"))
        .filter_map(|run_number| run_number.parse::<u64>().ok())
        .max()
        .unwrap_or(0)
}

fn run_summary(run_id: &str, run: &WorkflowRun) -> Value {
    let mut nodes = Map::new();
    let mut workflow_id = Value::Null;
    let mut status = "pending";

    for event in run.events() {
        match event["type"].as_str() {
            Some("workflow_started") => {
                status = "running";
                workflow_id = event["workflow_id"].clone();
            }
            Some("workflow_completed") => {
                status = "completed";
                workflow_id = event["workflow_id"].clone();
            }
            Some("workflow_failed") => {
                status = "failed";
                workflow_id = event["workflow_id"].clone();
            }
            Some("node_started") => {
                if let Some(node_id) = event["node_id"].as_str() {
                    nodes.insert(node_id.to_owned(), json!({ "status": "running" }));
                }
            }
            Some("node_completed") => {
                if let Some(node_id) = event["node_id"].as_str() {
                    nodes.insert(
                        node_id.to_owned(),
                        json!({
                            "status": "completed",
                            "result": event["result"].clone(),
                        }),
                    );
                }
            }
            Some("node_failed") => {
                if let Some(node_id) = event["node_id"].as_str() {
                    nodes.insert(
                        node_id.to_owned(),
                        json!({
                            "status": "failed",
                            "error": event["error"].clone(),
                        }),
                    );
                }
            }
            _ => {}
        }
    }

    json!({
        "run_id": run_id,
        "workflow_id": workflow_id,
        "status": status,
        "nodes": nodes,
    })
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
struct WorkflowIdParams {
    workflow_id: String,
}

#[derive(Deserialize)]
struct UpdateWorkflowParams {
    workflow_id: String,
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
