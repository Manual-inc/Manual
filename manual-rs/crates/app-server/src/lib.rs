use std::collections::BTreeMap;
use std::io::{self, BufRead, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

use manual_node::{
    NodeRun, NodeTemplate, STORYBOOK_INPUT_NODE_ID, iso_timestamp, node_run_summary,
    storybook_workflow,
};
use manual_worflow::{
    DependencyDefinition, ExecutionMode, ExecutionOptions, NodeDefinition, NodeKind, RunController,
    WorkflowDefinition, WorkflowError, WorkflowRun,
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
    node_templates: Arc<RwLock<BTreeMap<String, NodeTemplate>>>,
    node_runs: Arc<RwLock<BTreeMap<String, NodeRun>>>,
    next_node_run_number: Arc<Mutex<u64>>,
    run_controllers: Arc<RwLock<BTreeMap<String, Arc<RunController>>>>,
    manuals: Arc<RwLock<BTreeMap<String, Value>>>,
    next_manual_number: Arc<Mutex<u64>>,
    sandboxes: Arc<RwLock<BTreeMap<String, Value>>>,
    next_sandbox_number: Arc<Mutex<u64>>,
    node_test_cases: Arc<RwLock<BTreeMap<String, Value>>>,
    next_node_test_case_number: Arc<Mutex<u64>>,
    optimization_runs: Arc<RwLock<BTreeMap<String, Value>>>,
    skill_records: Arc<RwLock<BTreeMap<String, Value>>>,
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
        let node_templates = workflow_store.load_node_templates();
        let node_runs = workflow_store.load_node_runs();
        let next_node_run_number_val = next_node_run_number(&node_runs);
        let manuals = workflow_store.load_values("manuals");
        let sandboxes = workflow_store.load_values("sandboxes");
        let node_test_cases = workflow_store.load_values("node_test_cases");
        let optimization_runs = workflow_store.load_values("optimization_runs");
        let skill_records = workflow_store.load_values("skill_records");

        Self {
            workflows: Arc::new(RwLock::new(workflows)),
            runs: Arc::new(RwLock::new(runs)),
            next_run_number: Arc::new(Mutex::new(next_run_number)),
            workflow_store,
            event_hub: EventHub::default(),
            node_templates: Arc::new(RwLock::new(node_templates)),
            node_runs: Arc::new(RwLock::new(node_runs)),
            next_node_run_number: Arc::new(Mutex::new(next_node_run_number_val)),
            run_controllers: Arc::new(RwLock::new(BTreeMap::new())),
            next_manual_number: Arc::new(Mutex::new(next_prefixed_number(&manuals, "manual-"))),
            manuals: Arc::new(RwLock::new(manuals)),
            next_sandbox_number: Arc::new(Mutex::new(next_prefixed_number(&sandboxes, "sandbox-"))),
            sandboxes: Arc::new(RwLock::new(sandboxes)),
            next_node_test_case_number: Arc::new(Mutex::new(next_prefixed_number(
                &node_test_cases,
                "node-case-",
            ))),
            node_test_cases: Arc::new(RwLock::new(node_test_cases)),
            optimization_runs: Arc::new(RwLock::new(optimization_runs)),
            skill_records: Arc::new(RwLock::new(skill_records)),
        }
    }

    pub fn handle_json(&self, input: &str) -> String {
        self.handle_json_value(input).to_string()
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
            "workflow.stop" => self.stop_workflow(request.id, request.params),
            "workflow.resume" => self.resume_workflow(request.id, request.params),
            "workflow.events" => self.workflow_events(request.id, request.params),
            "node.create" => self.create_node_template(request.id, request.params),
            "node.get" => self.get_node_template(request.id, request.params),
            "node.list" => self.list_node_templates(request.id),
            "node.update" => self.update_node_template(request.id, request.params),
            "node.delete" => self.delete_node_template(request.id, request.params),
            "node.schema" => self.node_schema(request.id, request.params),
            "node.run" => self.run_node(request.id, request.params),
            "node.run.get" => self.get_node_run(request.id, request.params),
            "node.run.events" => self.node_run_events(request.id, request.params),
            "node.testcase.save" => self.save_node_test_case(request.id, request.params),
            "node.testcase.verify" => self.verify_node_test_cases(request.id, request.params),
            "workflow.compose_from_registry" => {
                self.compose_workflow_from_registry(request.id, request.params)
            }
            "agent.list" => self.list_agents(request.id, request.params),
            "manual.create" => self.create_manual(request.id, request.params),
            "manual.get" => self.get_manual(request.id, request.params),
            "manual.list" => self.list_manuals(request.id, request.params),
            "manual.update" => self.update_manual(request.id, request.params),
            "manual.clone" => self.clone_manual(request.id, request.params),
            "manual.archive" => self.archive_manual(request.id, request.params),
            "manual.delete" => self.delete_manual(request.id, request.params),
            "manual.activate" => self.activate_manual(request.id, request.params),
            "manual.versions" => self.manual_versions(request.id, request.params),
            "optimization.record_run" => self.record_optimization_run(request.id, request.params),
            "optimization.analyze" => self.analyze_optimization(request.id, request.params),
            "optimization.compare" => self.compare_optimization_runs(request.id, request.params),
            "optimization.report" => self.optimization_report(request.id, request.params),
            "sandbox.create" => self.create_sandbox(request.id, request.params),
            "sandbox.update" => self.update_sandbox(request.id, request.params),
            "sandbox.evaluate" => self.evaluate_sandbox(request.id, request.params),
            "sandbox.get" => self.get_sandbox(request.id, request.params),
            "skill.configure" => self.configure_skill_step(request.id, request.params),
            "skill.candidates" => self.skill_candidates(request.id, request.params),
            "skill.record_execution" => self.record_skill_execution(request.id, request.params),
            "skill.verify" => self.verify_skill_usage(request.id, request.params),
            "skill.agent_capabilities" => self.skill_agent_capabilities(request.id),
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
        if let Err(error) = self.materialize_workflow_sandboxes(&mut workflow) {
            return rpc_error(id, -32602, error);
        }

        // resume_run_id가 있으면 기존 run을 previous_run으로 사용
        let previous_run = if let Some(ref resume_run_id) = params.resume_run_id {
            let stored = self.workflow_store.load_run(resume_run_id);
            if stored.is_none() {
                // 메모리에서 찾기
                self.runs
                    .read()
                    .expect("run state lock should not poison")
                    .get(resume_run_id)
                    .cloned()
            } else {
                stored
            }
        } else {
            None
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

        let exec_mode = match params.mode {
            StartWorkflowMode::Auto => ExecutionMode::Auto,
            StartWorkflowMode::Step => ExecutionMode::Step,
        };

        let controller = Arc::new(RunController::new(&exec_mode));
        self.run_controllers
            .write()
            .expect("run controller lock should not poison")
            .insert(run_id.clone(), Arc::clone(&controller));

        let input_overrides: BTreeMap<String, Value> = params.input_overrides.into_iter().collect();

        let opts = ExecutionOptions {
            start_node_id: params.start_node_id,
            resume_from_failure: params.resume_from_failure,
            input_overrides,
            mode: exec_mode,
            previous_run,
        };

        let runs = Arc::clone(&self.runs);
        let workflow_store = self.workflow_store.clone();
        let event_hub = self.event_hub.clone();
        let run_controllers = Arc::clone(&self.run_controllers);
        let thread_run_id = run_id.clone();

        thread::spawn(move || {
            let result =
                workflow.execute_with_options(&thread_run_id, opts, Some(controller), |event| {
                    let mut runs = runs.write().expect("run state lock should not poison");
                    if let Some(run) = runs.get_mut(&thread_run_id) {
                        run.record_event(event);
                        if let Err(error) = workflow_store.save_run(&thread_run_id, run) {
                            eprintln!("failed to persist workflow run {thread_run_id}: {error}");
                        }
                        event_hub.publish(run_changed_event(&thread_run_id, "event"));
                    }
                });

            // 실행 완료 후 controller 제거
            run_controllers
                .write()
                .expect("run controller lock should not poison")
                .remove(&thread_run_id);

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

    fn stop_workflow(&self, id: Value, params: Value) -> Value {
        let params = match serde_json::from_value::<RunIdParams>(params) {
            Ok(params) => params,
            Err(error) => return rpc_error(id, -32602, error.to_string()),
        };

        let controllers = self
            .run_controllers
            .read()
            .expect("run controller lock should not poison");

        if let Some(ctrl) = controllers.get(&params.run_id) {
            ctrl.cancel();
            rpc_result(id, json!({ "run_id": params.run_id, "cancelled": true }))
        } else {
            // 이미 완료된 run이면 해당 run 존재 여부만 확인
            drop(controllers);
            let runs = self.runs.read().expect("run state lock should not poison");
            if runs.contains_key(&params.run_id)
                || self.workflow_store.load_run(&params.run_id).is_some()
            {
                rpc_result(
                    id,
                    json!({ "run_id": params.run_id, "cancelled": false, "message": "run already completed" }),
                )
            } else {
                rpc_error(id, -32001, "run not found")
            }
        }
    }

    fn resume_workflow(&self, id: Value, params: Value) -> Value {
        let params = match serde_json::from_value::<ResumeWorkflowParams>(params) {
            Ok(params) => params,
            Err(error) => return rpc_error(id, -32602, error.to_string()),
        };

        // 활성 run의 controller가 있으면 (step 모드 대기 중) → request_step
        let controllers = self
            .run_controllers
            .read()
            .expect("run controller lock should not poison");

        if let Some(ctrl) = controllers.get(&params.run_id) {
            ctrl.request_step();
            return rpc_result(id, json!({ "run_id": params.run_id, "resumed": true }));
        }
        drop(controllers);

        // 활성 controller 없음 → 완료/실패된 run을 previous_run으로 새 실행 시작
        let run = {
            let stored = self.workflow_store.load_run(&params.run_id);
            if let Some(run) = stored {
                run
            } else {
                let runs = self.runs.read().expect("run state lock should not poison");
                match runs.get(&params.run_id).cloned() {
                    Some(run) => run,
                    None => return rpc_error(id, -32001, "run not found"),
                }
            }
        };

        // workflow_id는 run 이벤트에서 추출
        let workflow_id = run.events().iter().find_map(|e| {
            if e["type"] == "workflow_started" {
                e["workflow_id"].as_str().map(str::to_owned)
            } else {
                None
            }
        });

        let workflow_id = match workflow_id {
            Some(wid) => wid,
            None => return rpc_error(id, -32001, "could not determine workflow_id from run"),
        };

        // 새 start 파라미터 조합
        let new_params = json!({
            "workflow_id": workflow_id,
            "resume_run_id": params.run_id,
            "resume_from_failure": params.resume_from_failure.unwrap_or(false),
            "start_node_id": params.start_node_id,
            "input_overrides": params.input_overrides.unwrap_or_default(),
            "mode": params.mode.unwrap_or_else(|| "auto".to_owned()),
        });

        self.start_workflow(id, new_params)
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

    fn create_node_template(&self, id: Value, params: Value) -> Value {
        let params = match serde_json::from_value::<CreateNodeTemplateParams>(params) {
            Ok(p) => p,
            Err(e) => return rpc_error(id, -32602, e.to_string()),
        };

        if params.node.id == STORYBOOK_INPUT_NODE_ID {
            return rpc_error(
                id,
                -32602,
                format!("node id cannot be {STORYBOOK_INPUT_NODE_ID}"),
            );
        }

        let template_id = params.node.id.clone();
        let now = iso_timestamp();
        let template = NodeTemplate {
            id: template_id.clone(),
            name: params.name,
            description: params.description,
            node: params.node,
            created_at: now.clone(),
            updated_at: now,
        };

        if let Err(e) = self.workflow_store.save_node_template(&template) {
            return rpc_error(id, -32002, e.to_string());
        }

        self.node_templates
            .write()
            .expect("node template lock should not poison")
            .insert(template_id.clone(), template.clone());
        self.event_hub
            .publish(node_changed_event("node_created", &template_id));

        rpc_result(id, json!({ "template": template }))
    }

    fn get_node_template(&self, id: Value, params: Value) -> Value {
        let params = match serde_json::from_value::<NodeIdParams>(params) {
            Ok(p) => p,
            Err(e) => return rpc_error(id, -32602, e.to_string()),
        };

        let templates = self
            .node_templates
            .read()
            .expect("node template lock should not poison");
        let template = match templates.get(&params.node_id) {
            Some(t) => t.clone(),
            None => return rpc_error(id, -32000, "node template not found"),
        };

        rpc_result(id, json!({ "template": template }))
    }

    fn list_node_templates(&self, id: Value) -> Value {
        let templates = self
            .node_templates
            .read()
            .expect("node template lock should not poison")
            .values()
            .cloned()
            .collect::<Vec<_>>();

        rpc_result(id, json!({ "templates": templates }))
    }

    fn update_node_template(&self, id: Value, params: Value) -> Value {
        let params = match serde_json::from_value::<UpdateNodeTemplateParams>(params) {
            Ok(p) => p,
            Err(e) => return rpc_error(id, -32602, e.to_string()),
        };

        let mut templates = self
            .node_templates
            .write()
            .expect("node template lock should not poison");

        let existing = match templates.get_mut(&params.node_id) {
            Some(t) => t,
            None => return rpc_error(id, -32000, "node template not found"),
        };

        if let Some(name) = params.name {
            existing.name = name;
        }
        if let Some(description) = params.description {
            existing.description = description;
        }
        if let Some(node) = params.node {
            if node.id != params.node_id {
                return rpc_error(id, -32602, "node.id must match node_id");
            }
            existing.node = node;
        }
        existing.updated_at = iso_timestamp();

        let updated = existing.clone();
        drop(templates);

        if let Err(e) = self.workflow_store.save_node_template(&updated) {
            return rpc_error(id, -32002, e.to_string());
        }

        self.event_hub
            .publish(node_changed_event("node_updated", &params.node_id));
        rpc_result(id, json!({ "template": updated }))
    }

    fn delete_node_template(&self, id: Value, params: Value) -> Value {
        let params = match serde_json::from_value::<NodeIdParams>(params) {
            Ok(p) => p,
            Err(e) => return rpc_error(id, -32602, e.to_string()),
        };

        if !self
            .node_templates
            .read()
            .expect("node template lock should not poison")
            .contains_key(&params.node_id)
        {
            return rpc_error(id, -32000, "node template not found");
        }

        if let Err(e) = self.workflow_store.delete_node_template(&params.node_id) {
            return rpc_error(id, -32002, e.to_string());
        }

        self.node_templates
            .write()
            .expect("node template lock should not poison")
            .remove(&params.node_id);
        self.event_hub
            .publish(node_changed_event("node_deleted", &params.node_id));

        rpc_result(id, json!({ "node_id": params.node_id, "deleted": true }))
    }

    fn node_schema(&self, id: Value, params: Value) -> Value {
        let params = match serde_json::from_value::<NodeSchemaParams>(params) {
            Ok(p) => p,
            Err(e) => return rpc_error(id, -32602, e.to_string()),
        };

        rpc_result(
            id,
            json!({ "schema": manual_node::node_schema(&params.kind) }),
        )
    }

    fn run_node(&self, id: Value, params: Value) -> Value {
        let params = match serde_json::from_value::<RunNodeParams>(params) {
            Ok(p) => p,
            Err(e) => return rpc_error(id, -32602, e.to_string()),
        };

        if params.node.id == STORYBOOK_INPUT_NODE_ID {
            return rpc_error(
                id,
                -32602,
                format!("node id cannot be {STORYBOOK_INPUT_NODE_ID}"),
            );
        }

        let run_id = {
            let mut n = self
                .next_node_run_number
                .lock()
                .expect("node run number lock should not poison");
            *n += 1;
            format!("node-run-{}", *n)
        };

        let pending_run =
            NodeRun::pending(run_id.clone(), params.node.clone(), params.inputs.clone());

        if let Err(e) = self.workflow_store.save_node_run(&run_id, &pending_run) {
            return rpc_error(id, -32002, e.to_string());
        }

        self.node_runs
            .write()
            .expect("node run lock should not poison")
            .insert(run_id.clone(), pending_run);
        self.event_hub
            .publish(node_run_changed_event(&run_id, "created"));

        let node_runs = Arc::clone(&self.node_runs);
        let workflow_store = self.workflow_store.clone();
        let event_hub = self.event_hub.clone();
        let thread_run_id = run_id.clone();
        let inputs = params.inputs;
        let mut node = params.node;
        if let Err(error) = self.materialize_node_sandbox(&mut node) {
            let mut runs = self
                .node_runs
                .write()
                .expect("node run lock should not poison");
            if let Some(run) = runs.get_mut(&run_id) {
                run.record_event(json!({
                    "run_id": run_id,
                    "type": "node_failed",
                    "node_id": node.id,
                    "error": error.clone(),
                }));
            }
            return rpc_error(id, -32602, error);
        }

        thread::spawn(move || {
            let temp_workflow = storybook_workflow(node, inputs, &thread_run_id);

            let result = temp_workflow.execute_with_events(&thread_run_id, |event| {
                let mut runs = node_runs.write().expect("node run lock should not poison");
                if let Some(run) = runs.get_mut(&thread_run_id) {
                    run.record_event(event);
                    if let Err(e) = workflow_store.save_node_run(&thread_run_id, run) {
                        eprintln!("failed to persist node run {thread_run_id}: {e}");
                    }
                    event_hub.publish(node_run_changed_event(&thread_run_id, "event"));
                }
            });

            if let Err(error) = result {
                let mut runs = node_runs.write().expect("node run lock should not poison");
                if let Some(run) = runs.get_mut(&thread_run_id) {
                    if !run.completed() {
                        run.record_event(json!({
                            "run_id": thread_run_id,
                            "sequence": run.events().len(),
                            "type": "workflow_failed",
                            "error": format!("{error:?}"),
                        }));
                        if let Err(e) = workflow_store.save_node_run(&thread_run_id, run) {
                            eprintln!("failed to persist node run {thread_run_id}: {e}");
                        }
                        event_hub.publish(node_run_changed_event(&thread_run_id, "event"));
                    }
                }
            }
        });

        rpc_result(id, json!({ "run_id": run_id }))
    }

    fn get_node_run(&self, id: Value, params: Value) -> Value {
        let params = match serde_json::from_value::<NodeRunIdParams>(params) {
            Ok(p) => p,
            Err(e) => return rpc_error(id, -32602, e.to_string()),
        };

        if let Some(stored) = self.workflow_store.load_node_run(&params.run_id) {
            let mut runs = self
                .node_runs
                .write()
                .expect("node run lock should not poison");
            let should_update = runs
                .get(&params.run_id)
                .is_none_or(|r| stored.events().len() > r.events().len());
            if should_update {
                runs.insert(params.run_id.clone(), stored);
            }
        }

        let runs = self
            .node_runs
            .read()
            .expect("node run lock should not poison");
        let run = match runs.get(&params.run_id) {
            Some(r) => r.clone(),
            None => return rpc_error(id, -32001, "node run not found"),
        };

        rpc_result(id, json!({ "run": node_run_summary(&params.run_id, &run) }))
    }

    fn node_run_events(&self, id: Value, params: Value) -> Value {
        let params = match serde_json::from_value::<NodeRunEventsParams>(params) {
            Ok(p) => p,
            Err(e) => return rpc_error(id, -32602, e.to_string()),
        };

        if let Some(stored) = self.workflow_store.load_node_run(&params.run_id) {
            let mut runs = self
                .node_runs
                .write()
                .expect("node run lock should not poison");
            let should_update = runs
                .get(&params.run_id)
                .is_none_or(|r| stored.events().len() > r.events().len());
            if should_update {
                runs.insert(params.run_id.clone(), stored);
            }
        }

        let runs = self
            .node_runs
            .read()
            .expect("node run lock should not poison");
        let run = match runs.get(&params.run_id) {
            Some(r) => r.clone(),
            None => return rpc_error(id, -32001, "node run not found"),
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
                "run": node_run_summary(&params.run_id, &run),
            }),
        )
    }

    fn save_node_test_case(&self, id: Value, params: Value) -> Value {
        let run_id = params["run_id"].as_str().unwrap_or_default();
        let run = match self.load_node_run(run_id) {
            Some(run) => run,
            None => return rpc_error(id, -32001, "node run not found"),
        };
        let case_id = self.next_id("node-case-", &self.next_node_test_case_number);
        let case = manual_node::create_test_case(
            case_id,
            &run,
            params.get("expected_output").cloned(),
            params.get("criteria").cloned(),
            &iso_timestamp(),
        );

        if let Err(error) = self.workflow_store.save_value(
            "node_test_cases",
            case["id"].as_str().unwrap_or_default(),
            &case,
        ) {
            return rpc_error(id, -32002, error.to_string());
        }
        self.node_test_cases
            .write()
            .expect("node test case lock should not poison")
            .insert(
                case["id"].as_str().unwrap_or_default().to_owned(),
                case.clone(),
            );

        rpc_result(id, json!({ "test_case": case }))
    }

    fn verify_node_test_cases(&self, id: Value, params: Value) -> Value {
        let node_id = params["node_id"].as_str();
        let cases = self
            .node_test_cases
            .read()
            .expect("node test case lock should not poison")
            .values()
            .cloned()
            .collect::<Vec<_>>();

        rpc_result(id, manual_node::verify_test_cases(cases, node_id))
    }

    fn compose_workflow_from_registry(&self, id: Value, params: Value) -> Value {
        let node_id = params["node_id"].as_str().unwrap_or_default();
        let templates = self
            .node_templates
            .read()
            .expect("node template lock should not poison");
        let Some(template) = templates.get(node_id) else {
            return rpc_error(id, -32602, "registered node template required");
        };
        rpc_result(id, manual_node::compose_registry_candidate(template))
    }

    fn list_agents(&self, id: Value, params: Value) -> Value {
        rpc_result(id, manual_agent::list_agent_availability(&params))
    }

    fn create_manual(&self, id: Value, params: Value) -> Value {
        let manual_id = self.next_id("manual-", &self.next_manual_number);
        let manual = match manual_core::create_manual(manual_id, &params, &iso_timestamp()) {
            Ok(manual) => manual,
            Err(error) => return rpc_error(id, -32602, error),
        };
        self.store_manual(id, manual)
    }

    fn get_manual(&self, id: Value, params: Value) -> Value {
        let Some(manual) = self.find_manual(params["manual_id"].as_str()) else {
            return rpc_error(id, -32000, "manual not found");
        };
        rpc_result(id, json!({ "manual": manual }))
    }

    fn list_manuals(&self, id: Value, params: Value) -> Value {
        let status_filter = params["status"].as_str();
        let query = params["query"].as_str().unwrap_or_default();
        let tag = params["tag"].as_str();
        let manuals = self
            .manuals
            .read()
            .expect("manual lock should not poison")
            .values()
            .filter(|manual| manual_core::matches_filters(manual, status_filter, query, tag))
            .map(manual_core::list_summary)
            .collect::<Vec<_>>();
        rpc_result(
            id,
            json!({
                "manuals": manuals,
                "filters": ["tag", "status"],
                "search_fields": ["name", "description"],
            }),
        )
    }

    fn update_manual(&self, id: Value, params: Value) -> Value {
        let Some(manual) = self.find_manual(params["manual_id"].as_str()) else {
            return rpc_error(id, -32000, "manual not found");
        };
        let manual = manual_core::update_manual(
            manual,
            params.get("changes").unwrap_or(&Value::Null),
            params["execution_affecting"].as_bool().unwrap_or(false),
            &iso_timestamp(),
        );
        self.store_manual(id, manual)
    }

    fn clone_manual(&self, id: Value, params: Value) -> Value {
        let Some(source) = self.find_manual(params["manual_id"].as_str()) else {
            return rpc_error(id, -32000, "manual not found");
        };
        let cloned = manual_core::clone_manual(
            source,
            self.next_id("manual-", &self.next_manual_number),
            &iso_timestamp(),
        );
        self.store_manual(id, cloned)
    }

    fn archive_manual(&self, id: Value, params: Value) -> Value {
        self.set_manual_status(id, params, "archived")
    }

    fn delete_manual(&self, id: Value, params: Value) -> Value {
        let Some(manual) = self.find_manual(params["manual_id"].as_str()) else {
            return rpc_error(id, -32000, "manual not found");
        };
        let manual = match manual_core::mark_deleted(manual, &iso_timestamp()) {
            Ok(manual) => manual,
            Err(error) => return rpc_error(id, -32003, error),
        };
        self.store_manual(id, manual)
    }

    fn activate_manual(&self, id: Value, params: Value) -> Value {
        let Some(mut manual) = self.find_manual(params["manual_id"].as_str()) else {
            return rpc_error(id, -32000, "manual not found");
        };
        let validation = manual_core::validate_for_activation(&manual);
        if validation["valid"] == true {
            manual = manual_core::set_status(manual, "active", &iso_timestamp());
            self.store_manual(id, manual)
        } else {
            rpc_result(
                id,
                json!({ "manual": manual, "validation": validation, "activated": false }),
            )
        }
    }

    fn manual_versions(&self, id: Value, params: Value) -> Value {
        let Some(manual) = self.find_manual(params["manual_id"].as_str()) else {
            return rpc_error(id, -32000, "manual not found");
        };
        rpc_result(id, manual_core::versions_response(&manual))
    }

    fn record_optimization_run(&self, id: Value, params: Value) -> Value {
        let run = manual_optimization::record_run(&params, &iso_timestamp());
        if let Err(error) = self.workflow_store.save_value(
            "optimization_runs",
            run["id"].as_str().unwrap_or_default(),
            &run,
        ) {
            return rpc_error(id, -32002, error.to_string());
        }
        self.optimization_runs
            .write()
            .expect("optimization lock should not poison")
            .insert(
                run["id"].as_str().unwrap_or_default().to_owned(),
                run.clone(),
            );
        rpc_result(id, json!({ "run": run }))
    }

    fn analyze_optimization(&self, id: Value, _params: Value) -> Value {
        rpc_result(id, manual_optimization::analyze())
    }

    fn compare_optimization_runs(&self, id: Value, _params: Value) -> Value {
        rpc_result(id, manual_optimization::compare_runs())
    }

    fn optimization_report(&self, id: Value, _params: Value) -> Value {
        rpc_result(id, manual_optimization::report())
    }

    fn create_sandbox(&self, id: Value, params: Value) -> Value {
        let sandbox = manual_sandbox::create_sandbox(
            self.next_id("sandbox-", &self.next_sandbox_number),
            &params,
            &iso_timestamp(),
        );
        self.store_sandbox(id, sandbox)
    }

    fn update_sandbox(&self, id: Value, params: Value) -> Value {
        let Some(mut sandbox) = self.find_sandbox(params["sandbox_id"].as_str()) else {
            return rpc_error(id, -32000, "sandbox not found");
        };
        sandbox = manual_sandbox::update_sandbox(
            sandbox,
            params.get("changes").unwrap_or(&Value::Null),
            &iso_timestamp(),
        );
        self.store_sandbox(id, sandbox)
    }

    fn evaluate_sandbox(&self, id: Value, params: Value) -> Value {
        if params.get("sandbox_id").is_some()
            && params["sandbox_id"].as_str().unwrap_or_default().is_empty()
        {
            return rpc_error(id, -32602, "sandbox_id is required");
        }
        let Some(sandbox) = self.find_sandbox(params["sandbox_id"].as_str()) else {
            return rpc_error(id, -32000, "sandbox not found");
        };
        let operation = params["operation"].as_str().unwrap_or_default();
        let target = params["target"].as_str().unwrap_or_default();
        let decision = manual_sandbox::evaluate(&sandbox, operation, target);
        rpc_result(id, json!({ "decision": decision, "sandbox": sandbox }))
    }

    fn get_sandbox(&self, id: Value, params: Value) -> Value {
        let Some(sandbox) = self.find_sandbox(params["sandbox_id"].as_str()) else {
            return rpc_error(id, -32000, "sandbox not found");
        };
        rpc_result(id, json!({ "sandbox": sandbox }))
    }

    fn configure_skill_step(&self, id: Value, params: Value) -> Value {
        let record = manual_skill::configure_step(&params, &iso_timestamp());
        let record_id = record["id"].as_str().unwrap_or("agent-step").to_owned();
        if let Err(error) = self
            .workflow_store
            .save_value("skill_records", &record_id, &record)
        {
            return rpc_error(id, -32002, error.to_string());
        }
        self.skill_records
            .write()
            .expect("skill lock should not poison")
            .insert(record_id, record.clone());
        rpc_result(id, json!({ "step": record }))
    }

    fn skill_candidates(&self, id: Value, params: Value) -> Value {
        rpc_result(id, manual_skill::candidates(&params))
    }

    fn record_skill_execution(&self, id: Value, params: Value) -> Value {
        let step_id = params["step_id"].as_str().unwrap_or("agent-step");
        let existing = self
            .skill_records
            .read()
            .expect("skill lock should not poison")
            .get(step_id)
            .cloned();
        let record = manual_skill::record_execution(existing, step_id, &params);
        if let Err(error) = self
            .workflow_store
            .save_value("skill_records", step_id, &record)
        {
            return rpc_error(id, -32002, error.to_string());
        }
        self.skill_records
            .write()
            .expect("skill lock should not poison")
            .insert(step_id.to_owned(), record.clone());
        rpc_result(
            id,
            json!({ "execution": record["execution"], "step": record }),
        )
    }

    fn verify_skill_usage(&self, id: Value, params: Value) -> Value {
        let step_id = params["step_id"].as_str().unwrap_or("agent-step");
        let record = self
            .skill_records
            .read()
            .expect("skill lock should not poison")
            .get(step_id)
            .cloned();
        rpc_result(id, manual_skill::verify_usage(record, step_id))
    }

    fn skill_agent_capabilities(&self, id: Value) -> Value {
        rpc_result(id, manual_skill::agent_capabilities())
    }

    fn load_node_run(&self, run_id: &str) -> Option<NodeRun> {
        self.workflow_store.load_node_run(run_id).or_else(|| {
            self.node_runs
                .read()
                .expect("node run lock should not poison")
                .get(run_id)
                .cloned()
        })
    }

    fn next_id(&self, prefix: &str, counter: &Mutex<u64>) -> String {
        let mut next = counter.lock().expect("id counter lock should not poison");
        *next += 1;
        format!("{prefix}{next}")
    }

    fn store_manual(&self, id: Value, manual: Value) -> Value {
        let manual_id = manual["id"].as_str().unwrap_or_default().to_owned();
        if let Err(error) = self
            .workflow_store
            .save_value("manuals", &manual_id, &manual)
        {
            return rpc_error(id, -32002, error.to_string());
        }
        self.manuals
            .write()
            .expect("manual lock should not poison")
            .insert(manual_id, manual.clone());
        rpc_result(id, json!({ "manual": manual }))
    }

    fn find_manual(&self, manual_id: Option<&str>) -> Option<Value> {
        let manuals = self.manuals.read().expect("manual lock should not poison");
        if let Some(manual_id) = manual_id.filter(|value| !value.is_empty()) {
            manuals.get(manual_id).cloned()
        } else {
            manuals.values().next().cloned()
        }
    }

    fn set_manual_status(&self, id: Value, params: Value, status: &str) -> Value {
        let Some(mut manual) = self.find_manual(params["manual_id"].as_str()) else {
            return rpc_error(id, -32000, "manual not found");
        };
        manual = manual_core::set_status(manual, status, &iso_timestamp());
        self.store_manual(id, manual)
    }

    fn store_sandbox(&self, id: Value, sandbox: Value) -> Value {
        let sandbox_id = sandbox["id"].as_str().unwrap_or_default().to_owned();
        if let Err(error) = self
            .workflow_store
            .save_value("sandboxes", &sandbox_id, &sandbox)
        {
            return rpc_error(id, -32002, error.to_string());
        }
        self.sandboxes
            .write()
            .expect("sandbox lock should not poison")
            .insert(sandbox_id, sandbox.clone());
        rpc_result(id, json!({ "sandbox": sandbox }))
    }

    fn find_sandbox(&self, sandbox_id: Option<&str>) -> Option<Value> {
        let sandboxes = self
            .sandboxes
            .read()
            .expect("sandbox lock should not poison");
        if let Some(sandbox_id) = sandbox_id.filter(|value| !value.is_empty()) {
            sandboxes.get(sandbox_id).cloned()
        } else {
            sandboxes.values().next().cloned()
        }
    }

    fn materialize_workflow_sandboxes(
        &self,
        workflow: &mut WorkflowDefinition,
    ) -> Result<(), String> {
        for node in &mut workflow.nodes {
            self.materialize_node_sandbox(node)?;
        }
        Ok(())
    }

    fn materialize_node_sandbox(&self, node: &mut NodeDefinition) -> Result<(), String> {
        if node.sandbox_policy.is_null() {
            return Ok(());
        }
        let Some(sandbox_id) = node.sandbox_policy["sandbox_id"].as_str() else {
            return Ok(());
        };
        let Some(mut sandbox) = self.find_sandbox(Some(sandbox_id)) else {
            return Err(format!("sandbox not found: {sandbox_id}"));
        };
        sandbox["sandbox_id"] = json!(sandbox_id);
        node.sandbox_policy = sandbox;
        Ok(())
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

fn node_changed_event(kind: &str, node_id: &str) -> Value {
    json!({
        "type": "node_changed",
        "change": kind,
        "node_id": node_id,
    })
}

fn node_run_changed_event(run_id: &str, change: &str) -> Value {
    json!({
        "type": "node_run_changed",
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

fn next_node_run_number(runs: &BTreeMap<String, NodeRun>) -> u64 {
    runs.keys()
        .filter_map(|run_id| run_id.strip_prefix("node-run-"))
        .filter_map(|n| n.parse::<u64>().ok())
        .max()
        .unwrap_or(0)
}

fn next_prefixed_number(values: &BTreeMap<String, Value>, prefix: &str) -> u64 {
    values
        .keys()
        .filter_map(|id| id.strip_prefix(prefix))
        .filter_map(|number| number.parse::<u64>().ok())
        .max()
        .unwrap_or(0)
}

fn run_summary(run_id: &str, run: &WorkflowRun) -> Value {
    let mut nodes = Map::new();
    let mut workflow_id = Value::Null;
    let mut status = "pending";
    let mut first_failed_node: Option<String> = None;
    let mut paused = false;

    for event in run.events() {
        match event["type"].as_str() {
            Some("workflow_started") => {
                status = "running";
                paused = false;
                workflow_id = event["workflow_id"].clone();
            }
            Some("workflow_completed") => {
                status = "completed";
                paused = false;
                workflow_id = event["workflow_id"].clone();
            }
            Some("workflow_failed") => {
                status = "failed";
                paused = false;
                workflow_id = event["workflow_id"].clone();
            }
            Some("workflow_cancelled") => {
                status = "cancelled";
                paused = false;
                workflow_id = event["workflow_id"].clone();
            }
            Some("workflow_paused") => {
                paused = true;
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
                    if first_failed_node.is_none() {
                        first_failed_node = Some(node_id.to_owned());
                    }
                }
            }
            Some("node_skipped") => {
                if let Some(node_id) = event["node_id"].as_str() {
                    nodes.insert(node_id.to_owned(), json!({ "status": "skipped" }));
                }
            }
            _ => {}
        }
    }

    let resumable = matches!(status, "failed" | "cancelled") || paused;

    json!({
        "run_id": run_id,
        "workflow_id": workflow_id,
        "status": status,
        "nodes": nodes,
        "first_failed_node": first_failed_node,
        "resumable": resumable,
        "paused": paused,
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
    #[serde(default)]
    start_node_id: Option<String>,
    #[serde(default)]
    resume_from_failure: bool,
    #[serde(default)]
    input_overrides: serde_json::Map<String, Value>,
    #[serde(default)]
    mode: StartWorkflowMode,
    #[serde(default)]
    resume_run_id: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "snake_case")]
enum StartWorkflowMode {
    #[default]
    Auto,
    Step,
}

#[derive(Deserialize)]
struct RunIdParams {
    run_id: String,
}

#[derive(Deserialize)]
struct ResumeWorkflowParams {
    run_id: String,
    #[serde(default)]
    start_node_id: Option<String>,
    #[serde(default)]
    resume_from_failure: Option<bool>,
    #[serde(default)]
    input_overrides: Option<serde_json::Map<String, Value>>,
    #[serde(default)]
    mode: Option<String>,
}

#[derive(Deserialize)]
struct WorkflowEventsParams {
    run_id: String,
    #[serde(default)]
    cursor: usize,
}

#[derive(Deserialize)]
struct CreateNodeTemplateParams {
    node: NodeDefinition,
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
}

#[derive(Deserialize)]
struct NodeIdParams {
    node_id: String,
}

#[derive(Deserialize)]
struct UpdateNodeTemplateParams {
    node_id: String,
    name: Option<String>,
    description: Option<String>,
    node: Option<NodeDefinition>,
}

#[derive(Deserialize)]
struct NodeSchemaParams {
    kind: NodeKind,
}

#[derive(Deserialize)]
struct RunNodeParams {
    node: NodeDefinition,
    #[serde(default)]
    inputs: Value,
}

#[derive(Deserialize)]
struct NodeRunIdParams {
    run_id: String,
}

#[derive(Deserialize)]
struct NodeRunEventsParams {
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
