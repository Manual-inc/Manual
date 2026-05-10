use std::collections::BTreeMap;
use std::io::{self, BufRead, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

use manual_worflow::{
    DependencyDefinition, NodeDefinition, WorkflowDefinition, WorkflowError, WorkflowRun,
};
use serde::Deserialize;
use serde_json::{Map, Value, json};

mod workflow_store;

use workflow_store::{WorkflowStore, default_workflow_storage_dir};

pub fn crate_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

#[derive(Clone)]
pub struct HttpServerConfig {
    pub auth_token: String,
}

pub fn serve_http_listener(
    listener: TcpListener,
    server: AppServer,
    config: HttpServerConfig,
) -> io::Result<()> {
    for stream in listener.incoming() {
        let stream = stream?;
        let server = server.clone();
        let config = config.clone();
        thread::spawn(move || {
            if let Err(error) = handle_http_connection(stream, server, config) {
                eprintln!("failed to handle app-server HTTP connection: {error}");
            }
        });
    }

    Ok(())
}

#[derive(Clone)]
pub struct AppServer {
    workflows: Arc<RwLock<BTreeMap<String, WorkflowDefinition>>>,
    runs: Arc<RwLock<BTreeMap<String, WorkflowRun>>>,
    next_run_number: Arc<Mutex<u64>>,
    workflow_store: WorkflowStore,
    event_hub: EventHub,
}

impl AppServer {
    pub fn new() -> Self {
        Self::with_storage_dir(default_workflow_storage_dir())
    }

    pub fn with_storage_dir(storage_dir: impl AsRef<Path>) -> Self {
        let workflow_store = WorkflowStore::new(storage_dir);
        let runs = workflow_store.load_runs();
        let workflows = workflow_store.load_workflows();
        let next_run_number = next_run_number(&runs);

        Self {
            workflows: Arc::new(RwLock::new(workflows)),
            runs: Arc::new(RwLock::new(runs)),
            next_run_number: Arc::new(Mutex::new(next_run_number)),
            workflow_store,
            event_hub: EventHub::default(),
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
            "workflow.patch" => self.patch_workflow(request.id, request.params),
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

        if let Err(error) = self.workflow_store.save_workflow(&params.workflow) {
            return rpc_error(id, -32002, error.to_string());
        }

        self.workflows
            .write()
            .expect("workflow state lock should not poison")
            .insert(workflow_id.clone(), params.workflow);
        self.event_hub
            .publish(workflow_changed_event("workflow_created", &workflow_id));

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

        let workflows = self
            .workflows
            .read()
            .expect("workflow state lock should not poison");
        let workflow = match workflows.get(&params.workflow_id) {
            Some(workflow) => workflow.clone(),
            None => return rpc_error(id, -32000, "workflow not found"),
        };

        rpc_result(id, json!({ "workflow": workflow }))
    }

    fn list_workflows(&self, id: Value) -> Value {
        let workflows = self
            .workflows
            .read()
            .expect("workflow state lock should not poison")
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

        if !self
            .workflows
            .read()
            .expect("workflow state lock should not poison")
            .contains_key(&params.workflow_id)
        {
            return rpc_error(id, -32000, "workflow not found");
        }

        if let Err(error) = self.workflow_store.save_workflow(&params.workflow) {
            return rpc_error(id, -32002, error.to_string());
        }

        self.workflows
            .write()
            .expect("workflow state lock should not poison")
            .insert(params.workflow_id.clone(), params.workflow);
        self.event_hub
            .publish(workflow_changed_event("workflow_updated", &workflow_id));

        rpc_result(
            id,
            json!({
                "workflow_id": workflow_id,
                "node_count": node_count,
            }),
        )
    }

    fn patch_workflow(&self, id: Value, params: Value) -> Value {
        let params = match serde_json::from_value::<PatchWorkflowParams>(params) {
            Ok(params) => params,
            Err(error) => return rpc_error(id, -32602, error.to_string()),
        };

        let mut workflow = match self
            .workflows
            .read()
            .expect("workflow state lock should not poison")
            .get(&params.workflow_id)
            .cloned()
        {
            Some(workflow) => workflow,
            None => return rpc_error(id, -32000, "workflow not found"),
        };

        for operation in params.operations {
            if let Err(error) = apply_workflow_patch_operation(&mut workflow, operation) {
                return rpc_error(id, -32602, error);
            }
        }

        if let Err(error) = workflow.execution_plan() {
            return rpc_error(id, -32602, workflow_error_message(&error));
        }

        if let Err(error) = self.workflow_store.save_workflow(&workflow) {
            return rpc_error(id, -32002, error.to_string());
        }

        let node_count = workflow.nodes.len();
        let dependency_count = workflow.dependencies.len();
        self.workflows
            .write()
            .expect("workflow state lock should not poison")
            .insert(params.workflow_id.clone(), workflow);
        self.event_hub.publish(workflow_changed_event(
            "workflow_patched",
            &params.workflow_id,
        ));

        rpc_result(
            id,
            json!({
                "workflow_id": params.workflow_id,
                "node_count": node_count,
                "dependency_count": dependency_count,
            }),
        )
    }

    fn delete_workflow(&self, id: Value, params: Value) -> Value {
        let params = match serde_json::from_value::<WorkflowIdParams>(params) {
            Ok(params) => params,
            Err(error) => return rpc_error(id, -32602, error.to_string()),
        };

        if !self
            .workflows
            .read()
            .expect("workflow state lock should not poison")
            .contains_key(&params.workflow_id)
        {
            return rpc_error(id, -32000, "workflow not found");
        }

        if let Err(error) = self.workflow_store.delete_workflow(&params.workflow_id) {
            return rpc_error(id, -32002, error.to_string());
        }

        self.workflows
            .write()
            .expect("workflow state lock should not poison")
            .remove(&params.workflow_id);
        self.event_hub.publish(workflow_changed_event(
            "workflow_deleted",
            &params.workflow_id,
        ));

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

        let workflow = match self
            .workflows
            .read()
            .expect("workflow state lock should not poison")
            .get(&params.workflow_id)
            .cloned()
        {
            Some(workflow) => workflow,
            None => return rpc_error(id, -32000, "workflow not found"),
        };

        let run_id = {
            let mut next_run_number = self
                .next_run_number
                .lock()
                .expect("run number lock should not poison");
            *next_run_number += 1;
            format!("run-{}", *next_run_number)
        };

        let pending_run = WorkflowRun::pending();
        if let Err(error) = self.workflow_store.save_run(&run_id, &pending_run) {
            return rpc_error(id, -32002, error.to_string());
        }

        self.runs
            .write()
            .expect("run state lock should not poison")
            .insert(run_id.clone(), pending_run);
        self.event_hub
            .publish(run_changed_event(&run_id, "created"));

        let runs = Arc::clone(&self.runs);
        let workflow_store = self.workflow_store.clone();
        let event_hub = self.event_hub.clone();
        let thread_run_id = run_id.clone();

        thread::spawn(move || {
            let result = workflow.execute_with_events(&thread_run_id, |event| {
                let mut runs = runs.write().expect("run state lock should not poison");
                if let Some(run) = runs.get_mut(&thread_run_id) {
                    run.record_event(event);
                    if let Err(error) = workflow_store.save_run(&thread_run_id, run) {
                        eprintln!("failed to persist workflow run {thread_run_id}: {error}");
                    }
                    event_hub.publish(run_changed_event(&thread_run_id, "event"));
                }
            });

            if let Err(error) = result {
                let mut runs = runs.write().expect("run state lock should not poison");
                if let Some(run) = runs.get_mut(&thread_run_id) {
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
                        event_hub.publish(run_changed_event(&thread_run_id, "event"));
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

        if let Some(stored_run) = self.workflow_store.load_run(&params.run_id) {
            let mut runs = self.runs.write().expect("run state lock should not poison");
            let should_update = runs
                .get(&params.run_id)
                .is_none_or(|run| stored_run.events().len() > run.events().len());
            if should_update {
                runs.insert(params.run_id.clone(), stored_run);
            }
        }

        let runs = self.runs.read().expect("run state lock should not poison");
        let run = match runs.get(&params.run_id) {
            Some(run) => run.clone(),
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
                "run": run_summary(&params.run_id, &run),
            }),
        )
    }
}

impl Default for AppServer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Default)]
struct EventHub {
    subscribers: Arc<Mutex<Vec<Sender<Value>>>>,
}

impl EventHub {
    fn subscribe(&self) -> Receiver<Value> {
        let (sender, receiver) = mpsc::channel();
        self.subscribers
            .lock()
            .expect("event subscriber lock should not poison")
            .push(sender);
        receiver
    }

    fn publish(&self, event: Value) {
        let mut subscribers = self
            .subscribers
            .lock()
            .expect("event subscriber lock should not poison");
        subscribers.retain(|subscriber| subscriber.send(event.clone()).is_ok());
    }
}

fn workflow_changed_event(kind: &str, workflow_id: &str) -> Value {
    json!({
        "type": "workflow_changed",
        "change": kind,
        "workflow_id": workflow_id,
    })
}

fn run_changed_event(run_id: &str, change: &str) -> Value {
    json!({
        "type": "run_changed",
        "change": change,
        "run_id": run_id,
    })
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
struct PatchWorkflowParams {
    workflow_id: String,
    #[serde(default)]
    operations: Vec<WorkflowPatchOperation>,
}

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum WorkflowPatchOperation {
    AddNode {
        node: NodeDefinition,
    },
    UpdateNode {
        node: NodeDefinition,
    },
    DeleteNode {
        node_id: String,
    },
    AddDependency {
        dependency: DependencyDefinition,
    },
    UpdateDependency {
        node: String,
        depends_on: String,
        dependency: DependencyDefinition,
    },
    DeleteDependency {
        node: String,
        depends_on: String,
    },
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

fn apply_workflow_patch_operation(
    workflow: &mut WorkflowDefinition,
    operation: WorkflowPatchOperation,
) -> Result<(), String> {
    match operation {
        WorkflowPatchOperation::AddNode { node } => {
            if workflow.nodes.iter().any(|existing| existing.id == node.id) {
                return Err(format!("duplicate node: {}", node.id));
            }

            workflow.nodes.push(node);
        }
        WorkflowPatchOperation::UpdateNode { node } => {
            let Some(existing) = workflow
                .nodes
                .iter_mut()
                .find(|existing| existing.id == node.id)
            else {
                return Err(format!("unknown node: {}", node.id));
            };

            *existing = node;
        }
        WorkflowPatchOperation::DeleteNode { node_id } => {
            let original_node_count = workflow.nodes.len();
            workflow.nodes.retain(|node| node.id != node_id);
            if workflow.nodes.len() == original_node_count {
                return Err(format!("unknown node: {node_id}"));
            }

            workflow.dependencies.retain(|dependency| {
                dependency.node != node_id && dependency.depends_on != node_id
            });
        }
        WorkflowPatchOperation::AddDependency { dependency } => {
            if workflow.dependencies.iter().any(|existing| {
                existing.node == dependency.node && existing.depends_on == dependency.depends_on
            }) {
                return Err(format!(
                    "duplicate dependency: {} depends on {}",
                    dependency.node, dependency.depends_on
                ));
            }

            workflow.dependencies.push(dependency);
        }
        WorkflowPatchOperation::UpdateDependency {
            node,
            depends_on,
            dependency,
        } => {
            let Some(existing_index) = workflow
                .dependencies
                .iter()
                .position(|existing| existing.node == node && existing.depends_on == depends_on)
            else {
                return Err(format!(
                    "unknown dependency: {node} depends on {depends_on}"
                ));
            };

            if workflow
                .dependencies
                .iter()
                .enumerate()
                .any(|(index, existing)| {
                    index != existing_index
                        && existing.node == dependency.node
                        && existing.depends_on == dependency.depends_on
                })
            {
                return Err(format!(
                    "duplicate dependency: {} depends on {}",
                    dependency.node, dependency.depends_on
                ));
            }

            workflow.dependencies[existing_index] = dependency;
        }
        WorkflowPatchOperation::DeleteDependency { node, depends_on } => {
            let original_dependency_count = workflow.dependencies.len();
            workflow.dependencies.retain(|dependency| {
                dependency.node != node || dependency.depends_on != depends_on
            });
            if workflow.dependencies.len() == original_dependency_count {
                return Err(format!(
                    "unknown dependency: {node} depends on {depends_on}"
                ));
            }
        }
    }

    Ok(())
}

fn workflow_error_message(error: &WorkflowError) -> String {
    match error {
        WorkflowError::EmptyNodeId => "empty node id".to_owned(),
        WorkflowError::NodeIdTooLong { max } => format!("node id too long; max {max}"),
        WorkflowError::DuplicateNode { node } => format!("duplicate node: {node}"),
        WorkflowError::UnknownNode { node } => format!("unknown node: {node}"),
        WorkflowError::MissingTask { node } => format!("missing task: {node}"),
        WorkflowError::NodeExecutionFailed { message, .. } => message.clone(),
        WorkflowError::AgentCommandIo { message } => message.clone(),
        WorkflowError::AgentCommandFailed {
            status_code,
            stderr,
        } => {
            format!("agent command failed with status {status_code:?}: {stderr}")
        }
        WorkflowError::CycleDetected => "cycle detected".to_owned(),
    }
}

struct HttpRequest {
    method: String,
    path: String,
    headers: BTreeMap<String, String>,
    body: String,
}

fn handle_http_connection(
    mut stream: TcpStream,
    server: AppServer,
    config: HttpServerConfig,
) -> io::Result<()> {
    let request = read_http_request(&stream)?;

    match (request.method.as_str(), request.path_without_query()) {
        ("POST", "/rpc") => {
            if !request.is_authorized(&config.auth_token) {
                return write_http_response(
                    &mut stream,
                    "401 Unauthorized",
                    "text/plain",
                    "unauthorized",
                );
            }

            let response = server.handle_json(&request.body);
            write_http_response(&mut stream, "200 OK", "application/json", &response)
        }
        ("GET", "/events") => {
            if !request.is_authorized(&config.auth_token) {
                return write_http_response(
                    &mut stream,
                    "401 Unauthorized",
                    "text/plain",
                    "unauthorized",
                );
            }

            write_sse_stream(stream, server.event_hub.subscribe())
        }
        ("GET", "/health") => write_http_response(&mut stream, "200 OK", "text/plain", "ok"),
        _ => write_http_response(&mut stream, "404 Not Found", "text/plain", "not found"),
    }
}

fn read_http_request(stream: &TcpStream) -> io::Result<HttpRequest> {
    let mut reader = io::BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts.next().unwrap_or_default().to_owned();
    let path = request_parts.next().unwrap_or_default().to_owned();

    let mut headers = BTreeMap::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }

        if let Some((name, value)) = trimmed.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_owned());
        }
    }

    let content_length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = vec![0; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body)?;
    }

    Ok(HttpRequest {
        method,
        path,
        headers,
        body: String::from_utf8_lossy(&body).into_owned(),
    })
}

impl HttpRequest {
    fn path_without_query(&self) -> &str {
        self.path
            .split_once('?')
            .map_or(&self.path, |(path, _)| path)
    }

    fn is_authorized(&self, expected_token: &str) -> bool {
        if expected_token.is_empty() {
            return false;
        }

        if self
            .headers
            .get("authorization")
            .and_then(|value| value.strip_prefix("Bearer "))
            == Some(expected_token)
        {
            return true;
        }

        if self.headers.get("x-manual-token").map(String::as_str) == Some(expected_token) {
            return true;
        }

        self.query_token() == Some(expected_token)
    }

    fn query_token(&self) -> Option<&str> {
        let (_, query) = self.path.split_once('?')?;
        query.split('&').find_map(|pair| {
            let (name, value) = pair.split_once('=')?;
            (name == "token").then_some(value)
        })
    }
}

fn write_http_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &str,
) -> io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )?;
    stream.flush()
}

fn write_sse_stream(mut stream: TcpStream, receiver: Receiver<Value>) -> io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n"
    )?;
    stream.flush()?;

    for event in receiver {
        let event_name = event["type"].as_str().unwrap_or("message");
        writeln!(stream, "event: {event_name}")?;
        writeln!(stream, "data: {event}")?;
        writeln!(stream)?;
        stream.flush()?;
    }

    Ok(())
}
