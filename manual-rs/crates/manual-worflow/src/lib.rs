use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::Path;
use std::sync::Condvar;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use manual_agent::{Agent, AgentCommand, CommandRequest, claude::Claude, codex::Codex, pi::Pi};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const MAX_NODE_ID_LEN: usize = 128;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(String);

impl NodeId {
    pub fn new(value: impl Into<String>) -> Result<Self, WorkflowError> {
        let value = value.into();

        if value.trim().is_empty() {
            return Err(WorkflowError::EmptyNodeId);
        }

        if value.len() > MAX_NODE_ID_LEN {
            return Err(WorkflowError::NodeIdTooLong {
                max: MAX_NODE_ID_LEN,
            });
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl TryFrom<&str> for NodeId {
    type Error = WorkflowError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<String> for NodeId {
    type Error = WorkflowError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkflowError {
    EmptyNodeId,
    NodeIdTooLong {
        max: usize,
    },
    DuplicateNode {
        node: NodeId,
    },
    UnknownNode {
        node: NodeId,
    },
    MissingTask {
        node: NodeId,
    },
    NodeExecutionFailed {
        node: NodeId,
        message: String,
    },
    AgentCommandIo {
        message: String,
    },
    AgentCommandFailed {
        status_code: Option<i32>,
        stderr: String,
    },
    CycleDetected,
}

pub trait WorkflowNode {
    fn run(&self, input: &NodeInput) -> Result<WorkflowValue, WorkflowError>;
}

#[derive(Clone, Debug, PartialEq)]
pub enum WorkflowValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    List(Vec<WorkflowValue>),
    Object(BTreeMap<String, WorkflowValue>),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct WorkflowDefinition {
    pub id: String,
    pub nodes: Vec<NodeDefinition>,
    #[serde(default)]
    pub dependencies: Vec<DependencyDefinition>,
}

impl WorkflowDefinition {
    pub fn execution_plan(&self) -> Result<ExecutionPlan, WorkflowError> {
        let mut workflow = Workflow::new();

        for node in &self.nodes {
            workflow.add_node(node.id.clone())?;
        }

        for dependency in &self.dependencies {
            workflow.add_dependency(dependency.node.clone(), dependency.depends_on.clone())?;
        }

        workflow.execution_plan()
    }

    pub fn execute(&self, run_id: &str) -> Result<WorkflowRun, WorkflowError> {
        let mut run = WorkflowRun::pending();
        self.execute_with_events(run_id, |event| run.record_event(event))?;
        Ok(run)
    }

    pub fn execute_with_events(
        &self,
        run_id: &str,
        emit: impl FnMut(Value),
    ) -> Result<(), WorkflowError> {
        self.execute_with_options(run_id, ExecutionOptions::default(), None, emit)
    }

    pub fn execute_with_options(
        &self,
        run_id: &str,
        opts: ExecutionOptions,
        controller: Option<Arc<RunController>>,
        mut emit: impl FnMut(Value),
    ) -> Result<(), WorkflowError> {
        let mut sequence = 0;

        // 이전 run에서 완료된 노드 출력 복원 + input_overrides 병합
        let mut outputs: BTreeMap<String, Value> = if let Some(ref prev) = opts.previous_run {
            prev.completed_nodes()
        } else {
            BTreeMap::new()
        };
        for (node_id, value) in &opts.input_overrides {
            outputs.insert(node_id.clone(), value.clone());
        }

        emit_event(
            &mut sequence,
            &mut emit,
            json!({
                "run_id": run_id,
                "type": "workflow_started",
                "workflow_id": self.id,
            }),
        );
        for (node_id, value) in &opts.input_overrides {
            // Why this exists: docs/wiki/features/partial-run-and-restart.md
            // requires user-supplied intermediate inputs to remain auditable.
            emit_event(
                &mut sequence,
                &mut emit,
                json!({
                    "run_id": run_id,
                    "type": "input_override",
                    "node_id": node_id,
                    "value": value,
                }),
            );
        }

        let plan = match self.execution_plan() {
            Ok(plan) => plan,
            Err(error) => {
                emit_workflow_failed(run_id, &self.id, &mut sequence, &mut emit, &error);
                return Err(error);
            }
        };

        let skippable = self.skippable_nodes(&opts, &outputs, &plan);

        for stage in plan.stages() {
            // cancel 체크
            if let Some(ref ctrl) = controller {
                if ctrl.is_cancelled() {
                    emit_event(
                        &mut sequence,
                        &mut emit,
                        json!({
                            "run_id": run_id,
                            "type": "workflow_cancelled",
                            "workflow_id": self.id,
                        }),
                    );
                    return Ok(());
                }
            }

            let stage_nodes: Vec<&NodeDefinition> = stage
                .iter()
                .map(|node_id| {
                    self.nodes
                        .iter()
                        .find(|node| node.id == node_id.as_str())
                        .expect("execution plan should only include defined nodes")
                })
                .collect();

            let (skip_nodes, run_nodes): (Vec<&&NodeDefinition>, Vec<&&NodeDefinition>) =
                stage_nodes
                    .iter()
                    .partition(|node| skippable.contains(&node.id));

            for node in &skip_nodes {
                emit_event(
                    &mut sequence,
                    &mut emit,
                    json!({
                        "run_id": run_id,
                        "type": "node_skipped",
                        "node_id": node.id,
                    }),
                );
            }

            if run_nodes.is_empty() {
                continue;
            }

            // Step 모드: 스테이지 실행 전 대기
            if let Some(ref ctrl) = controller {
                let (lock, cvar) = &*ctrl.step_gate;
                let mut state = lock.lock().expect("step gate lock should not poison");
                if *state == StepState::AwaitingStep {
                    emit_event(
                        &mut sequence,
                        &mut emit,
                        json!({
                            "run_id": run_id,
                            "type": "workflow_paused",
                            "workflow_id": self.id,
                        }),
                    );
                    state = cvar
                        .wait_while(state, |s| *s == StepState::AwaitingStep)
                        .expect("step gate lock should not poison");
                }
                if *state == StepState::Cancelled || ctrl.is_cancelled() {
                    drop(state);
                    emit_event(
                        &mut sequence,
                        &mut emit,
                        json!({
                            "run_id": run_id,
                            "type": "workflow_cancelled",
                            "workflow_id": self.id,
                        }),
                    );
                    return Ok(());
                }
                if *state == StepState::StepRequested {
                    *state = StepState::AwaitingStep;
                }
            }

            for node in &run_nodes {
                emit_event(
                    &mut sequence,
                    &mut emit,
                    json!({
                        "run_id": run_id,
                        "type": "node_started",
                        "node_id": node.id,
                    }),
                );
            }

            let stage_inputs = outputs.clone();
            let (tx, rx) = mpsc::channel();

            let stage_error = thread::scope(|scope| {
                for node in &run_nodes {
                    let tx = tx.clone();
                    let stage_inputs = &stage_inputs;
                    scope.spawn(move || {
                        let result = execute_definition_node(node, stage_inputs);
                        tx.send((node.id.clone(), result))
                            .expect("stage result receiver should stay open");
                    });
                }

                drop(tx);

                let mut stage_error = None;
                for (node_id, result) in rx {
                    match result {
                        Ok(result) => {
                            outputs.insert(node_id.clone(), result.clone());
                            emit_event(
                                &mut sequence,
                                &mut emit,
                                json!({
                                    "run_id": run_id,
                                    "type": "node_completed",
                                    "node_id": node_id,
                                    "result": result,
                                }),
                            );
                        }
                        Err(error) => {
                            emit_event(
                                &mut sequence,
                                &mut emit,
                                json!({
                                    "run_id": run_id,
                                    "type": "node_failed",
                                    "node_id": node_id,
                                    "error": workflow_error_message(&error),
                                }),
                            );
                            if stage_error.is_none() {
                                stage_error = Some(error);
                            }
                        }
                    }
                }
                stage_error
            });

            if let Some(error) = stage_error {
                emit_workflow_failed(run_id, &self.id, &mut sequence, &mut emit, &error);
                return Err(error);
            }
        }

        emit_event(
            &mut sequence,
            &mut emit,
            json!({
                "run_id": run_id,
                "type": "workflow_completed",
                "workflow_id": self.id,
            }),
        );

        Ok(())
    }

    fn skippable_nodes(
        &self,
        opts: &ExecutionOptions,
        completed_outputs: &BTreeMap<String, Value>,
        plan: &ExecutionPlan,
    ) -> std::collections::BTreeSet<String> {
        let mut skippable = std::collections::BTreeSet::new();

        if let Some(ref start_node_id) = opts.start_node_id {
            let start_stage = plan
                .stages()
                .iter()
                .position(|stage| stage.iter().any(|n| n.as_str() == start_node_id));
            if let Some(start_idx) = start_stage {
                for stage in plan.stages().iter().take(start_idx) {
                    for node_id in stage {
                        skippable.insert(node_id.as_str().to_owned());
                    }
                }
            }
        } else if opts.resume_from_failure {
            for node_id in completed_outputs.keys() {
                skippable.insert(node_id.clone());
            }
        }

        skippable
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NodeDefinition {
    pub id: String,
    pub kind: NodeKind,
    #[serde(default)]
    pub value: Value,
    #[serde(default)]
    pub template: String,
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default)]
    pub error: String,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_args: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub script: String,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub sandbox_policy: Value,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Claude,
    Constant,
    Codex,
    Delay,
    Fail,
    Pi,
    Script,
    Template,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DependencyDefinition {
    pub node: String,
    pub depends_on: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct WorkflowRun {
    events: Vec<Value>,
    completed: bool,
}

impl WorkflowRun {
    pub fn pending() -> Self {
        Self {
            events: Vec::new(),
            completed: false,
        }
    }

    pub fn events(&self) -> &[Value] {
        &self.events
    }

    pub fn completed(&self) -> bool {
        self.completed
    }

    pub fn record_event(&mut self, event: Value) {
        if matches!(
            event["type"].as_str(),
            Some("workflow_completed") | Some("workflow_failed") | Some("workflow_cancelled")
        ) {
            self.completed = true;
        }

        self.events.push(event);
    }

    pub fn completed_nodes(&self) -> BTreeMap<String, Value> {
        let mut map = BTreeMap::new();
        for event in &self.events {
            if event["type"] == "node_completed" {
                if let Some(node_id) = event["node_id"].as_str() {
                    map.insert(node_id.to_owned(), event["result"].clone());
                }
            }
        }
        map
    }

    pub fn first_failed_node(&self) -> Option<String> {
        for event in &self.events {
            if event["type"] == "node_failed" {
                if let Some(node_id) = event["node_id"].as_str() {
                    return Some(node_id.to_owned());
                }
            }
        }
        None
    }

    pub fn resumable(&self) -> bool {
        for event in self.events.iter().rev() {
            match event["type"].as_str() {
                Some("workflow_failed") | Some("workflow_cancelled") | Some("workflow_paused") => {
                    return true;
                }
                Some("workflow_completed") => return false,
                _ => {}
            }
        }
        false
    }
}

fn execute_definition_node(
    node: &NodeDefinition,
    outputs: &BTreeMap<String, Value>,
) -> Result<Value, WorkflowError> {
    match node.kind {
        NodeKind::Constant => Ok(node.value.clone()),
        NodeKind::Delay => {
            thread::sleep(Duration::from_millis(node.duration_ms));
            Ok(Value::Null)
        }
        NodeKind::Fail => {
            let message = if node.error.is_empty() {
                "node execution failed".to_owned()
            } else {
                node.error.clone()
            };

            Err(WorkflowError::NodeExecutionFailed {
                node: NodeId::new(node.id.clone())?,
                message,
            })
        }
        NodeKind::Claude => execute_claude_node(node, outputs),
        NodeKind::Codex => execute_codex_node(node, outputs),
        NodeKind::Pi => execute_pi_node(node, outputs),
        NodeKind::Script => execute_script_definition_node(node),
        NodeKind::Template => Ok(render_template(&node.template, outputs).into()),
    }
}

fn execute_claude_node(
    node: &NodeDefinition,
    outputs: &BTreeMap<String, Value>,
) -> Result<Value, WorkflowError> {
    let claude = Claude::new(Agent::new(
        "claude.code_reviewer",
        "Claude Code Reviewer",
        "Use Claude CLI to review code changes.",
    ));
    execute_agent_node(&claude, node, outputs)
}

fn execute_codex_node(
    node: &NodeDefinition,
    outputs: &BTreeMap<String, Value>,
) -> Result<Value, WorkflowError> {
    let codex = Codex::new(Agent::new(
        "codex.code_reviewer",
        "Codex Code Reviewer",
        "Use Codex CLI to review code changes.",
    ));
    execute_agent_node(&codex, node, outputs)
}

fn execute_pi_node(
    node: &NodeDefinition,
    outputs: &BTreeMap<String, Value>,
) -> Result<Value, WorkflowError> {
    let pi = Pi::new(Agent::new(
        "pi.pipeline_advisor",
        "Pi Pipeline Advisor",
        "Use Pi CLI to generate workflow recommendations.",
    ));
    execute_agent_node(&pi, node, outputs)
}

fn execute_agent_node(
    agent: &impl AgentCommand,
    node: &NodeDefinition,
    outputs: &BTreeMap<String, Value>,
) -> Result<Value, WorkflowError> {
    let prompt = format!("{}\n\nInput: {}", node.prompt, json!(outputs));
    let mut request = CommandRequest::new(prompt);

    if let Some(model) = node.model.as_deref().filter(|model| !model.is_empty()) {
        request = request.with_model(model);
    }

    if let Some(cwd) = node.cwd.as_deref().filter(|cwd| !cwd.is_empty()) {
        request = request.with_cwd(cwd);
    }

    for arg in &node.extra_args {
        request = request.with_extra_arg(arg);
    }

    if !node.sandbox_policy.is_null() {
        request = request.with_sandbox_policy(node.sandbox_policy.clone());
    }

    let output = agent
        .run(&request)
        .map_err(|error| WorkflowError::AgentCommandIo {
            message: error.to_string(),
        })?;

    if output.status_code != Some(0) {
        return Err(WorkflowError::AgentCommandFailed {
            status_code: output.status_code,
            stderr: output.stderr,
        });
    }

    Ok(json!({
        "status_code": output.status_code,
        "stdout": output.stdout,
        "stderr": output.stderr,
    }))
}

fn execute_script_definition_node(node: &NodeDefinition) -> Result<Value, WorkflowError> {
    if node.sandbox_policy.is_null() {
        return Err(WorkflowError::NodeExecutionFailed {
            node: NodeId::new(node.id.clone())?,
            message: "script node requires sandbox_policy".to_owned(),
        });
    }

    let (program, args) = if Path::new(&node.script).exists() {
        (node.script.clone(), Vec::new())
    } else {
        (
            "/bin/sh".to_owned(),
            vec!["-c".to_owned(), node.script.clone()],
        )
    };
    let output =
        manual_sandbox::run_sandboxed(&node.sandbox_policy, program, &args).map_err(|error| {
            WorkflowError::AgentCommandIo {
                message: error.to_string(),
            }
        })?;

    if output.status.code() != Some(0) {
        return Err(WorkflowError::AgentCommandFailed {
            status_code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    Ok(json!({
        "status_code": output.status.code(),
        "stdout": String::from_utf8_lossy(&output.stdout).into_owned(),
        "stderr": String::from_utf8_lossy(&output.stderr).into_owned(),
    }))
}

fn render_template(template: &str, outputs: &BTreeMap<String, Value>) -> String {
    let mut rendered = template.to_owned();

    for (node_id, value) in outputs {
        rendered = rendered.replace(&format!("{{{{{node_id}}}}}"), &json_scalar_to_string(value));

        if let Value::Object(fields) = value {
            for (field, value) in fields {
                rendered = rendered.replace(
                    &format!("{{{{{node_id}.{field}}}}}"),
                    &json_scalar_to_string(value),
                );
            }
        }
    }

    rendered
}

fn json_scalar_to_string(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Number(value) => value.to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Null => "null".to_owned(),
        other => other.to_string(),
    }
}

fn emit_event(sequence: &mut usize, emit: &mut impl FnMut(Value), mut event: Value) {
    event["sequence"] = (*sequence).into();
    *sequence += 1;
    emit(event);
}

fn emit_workflow_failed(
    run_id: &str,
    workflow_id: &str,
    sequence: &mut usize,
    emit: &mut impl FnMut(Value),
    error: &WorkflowError,
) {
    emit_event(
        sequence,
        emit,
        json!({
            "run_id": run_id,
            "type": "workflow_failed",
            "workflow_id": workflow_id,
            "error": workflow_error_message(error),
        }),
    );
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScriptModule {
    name: String,
    source: String,
}

impl ScriptModule {
    pub fn new(name: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source: source.into(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn source(&self) -> &str {
        &self.source
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustScript {
    source: String,
    modules: Vec<ScriptModule>,
}

impl RustScript {
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            modules: Vec::new(),
        }
    }

    pub fn with_module(mut self, module: ScriptModule) -> Self {
        self.modules.push(module);
        self
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn modules(&self) -> &[ScriptModule] {
        &self.modules
    }
}

pub trait ScriptRunner {
    fn run_script(
        &self,
        script: &RustScript,
        input: &NodeInput,
    ) -> Result<WorkflowValue, WorkflowError>;
}

pub struct ScriptNode<R> {
    script: RustScript,
    runner: R,
}

impl<R> ScriptNode<R> {
    pub fn new(script: RustScript, runner: R) -> Self {
        Self { script, runner }
    }

    pub fn script(&self) -> &RustScript {
        &self.script
    }
}

impl<R> WorkflowNode for ScriptNode<R>
where
    R: ScriptRunner,
{
    fn run(&self, input: &NodeInput) -> Result<WorkflowValue, WorkflowError> {
        self.runner.run_script(&self.script, input)
    }
}

pub struct ManualAgentNode<C> {
    command: C,
    prompt: String,
}

impl<C> ManualAgentNode<C> {
    pub fn new(command: C, prompt: impl Into<String>) -> Self {
        Self {
            command,
            prompt: prompt.into(),
        }
    }

    pub fn prompt(&self) -> &str {
        &self.prompt
    }
}

impl<C> WorkflowNode for ManualAgentNode<C>
where
    C: AgentCommand,
{
    fn run(&self, input: &NodeInput) -> Result<WorkflowValue, WorkflowError> {
        let prompt = format!("{}\n\nInput: {:?}", self.prompt, input.values());
        let output = self
            .command
            .run(&CommandRequest::new(prompt))
            .map_err(|error| WorkflowError::AgentCommandIo {
                message: error.to_string(),
            })?;

        if output.status_code != Some(0) {
            return Err(WorkflowError::AgentCommandFailed {
                status_code: output.status_code,
                stderr: output.stderr,
            });
        }

        Ok(WorkflowValue::Object(
            [
                (
                    "status_code".into(),
                    output
                        .status_code
                        .map(|code| WorkflowValue::Number(code as f64))
                        .unwrap_or(WorkflowValue::Null),
                ),
                ("stdout".into(), WorkflowValue::String(output.stdout)),
                ("stderr".into(), WorkflowValue::String(output.stderr)),
            ]
            .into(),
        ))
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NodeInput {
    values: BTreeMap<NodeId, WorkflowValue>,
}

impl NodeInput {
    pub fn get(&self, node: impl AsRef<str>) -> Option<&WorkflowValue> {
        let node = NodeId::new(node.as_ref()).ok()?;
        self.values.get(&node)
    }

    pub fn values(&self) -> &BTreeMap<NodeId, WorkflowValue> {
        &self.values
    }
}

#[derive(Default)]
pub struct Workflow {
    dependencies: BTreeMap<NodeId, BTreeSet<NodeId>>,
    tasks: BTreeMap<NodeId, Box<dyn WorkflowNode>>,
}

impl Workflow {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: impl Into<String>) -> Result<(), WorkflowError> {
        let node = NodeId::new(node)?;

        if self.dependencies.contains_key(&node) {
            return Err(WorkflowError::DuplicateNode { node });
        }

        self.dependencies.insert(node, BTreeSet::new());
        Ok(())
    }

    pub fn add_task(
        &mut self,
        node: impl Into<String>,
        task: impl WorkflowNode + 'static,
    ) -> Result<(), WorkflowError> {
        let node = NodeId::new(node)?;

        if self.dependencies.contains_key(&node) {
            return Err(WorkflowError::DuplicateNode { node });
        }

        self.dependencies.insert(node.clone(), BTreeSet::new());
        self.tasks.insert(node, Box::new(task));
        Ok(())
    }

    pub fn add_dependency(
        &mut self,
        node: impl Into<String>,
        depends_on: impl Into<String>,
    ) -> Result<(), WorkflowError> {
        let node = NodeId::new(node)?;
        let depends_on = NodeId::new(depends_on)?;

        if !self.dependencies.contains_key(&node) {
            return Err(WorkflowError::UnknownNode { node });
        }

        if !self.dependencies.contains_key(&depends_on) {
            return Err(WorkflowError::UnknownNode { node: depends_on });
        }

        self.dependencies
            .get_mut(&node)
            .expect("node existence was checked")
            .insert(depends_on);
        Ok(())
    }

    pub fn execution_plan(&self) -> Result<ExecutionPlan, WorkflowError> {
        let mut dependencies = self.dependencies.clone();
        let mut stages = Vec::new();

        while !dependencies.is_empty() {
            let ready: Vec<NodeId> = dependencies
                .iter()
                .filter_map(|(node, node_dependencies)| {
                    if node_dependencies.is_empty() {
                        Some(node.clone())
                    } else {
                        None
                    }
                })
                .collect();

            if ready.is_empty() {
                return Err(WorkflowError::CycleDetected);
            }

            for node in &ready {
                dependencies.remove(node);
            }

            for node_dependencies in dependencies.values_mut() {
                for node in &ready {
                    node_dependencies.remove(node);
                }
            }

            stages.push(ready);
        }

        Ok(ExecutionPlan { stages })
    }

    pub fn execute(&self) -> Result<WorkflowOutput, WorkflowError> {
        let plan = self.execution_plan()?;
        let mut outputs = BTreeMap::new();

        for stage in plan.stages() {
            for node in stage {
                let task = self
                    .tasks
                    .get(node)
                    .ok_or_else(|| WorkflowError::MissingTask { node: node.clone() })?;
                let input = self.input_for(node, &outputs);
                let output = task.run(&input)?;

                outputs.insert(node.clone(), output);
            }
        }

        Ok(WorkflowOutput { values: outputs })
    }

    fn input_for(&self, node: &NodeId, outputs: &BTreeMap<NodeId, WorkflowValue>) -> NodeInput {
        let values = self
            .dependencies
            .get(node)
            .into_iter()
            .flat_map(|dependencies| dependencies.iter())
            .filter_map(|dependency| {
                outputs
                    .get(dependency)
                    .map(|value| (dependency.clone(), value.clone()))
            })
            .collect();

        NodeInput { values }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionPlan {
    stages: Vec<Vec<NodeId>>,
}

impl ExecutionPlan {
    pub fn stages(&self) -> &[Vec<NodeId>] {
        &self.stages
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorkflowOutput {
    values: BTreeMap<NodeId, WorkflowValue>,
}

impl WorkflowOutput {
    pub fn get(&self, node: impl AsRef<str>) -> Option<&WorkflowValue> {
        let node = NodeId::new(node.as_ref()).ok()?;
        self.values.get(&node)
    }

    pub fn values(&self) -> &BTreeMap<NodeId, WorkflowValue> {
        &self.values
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum ExecutionMode {
    #[default]
    Auto,
    Step,
}

pub struct ExecutionOptions {
    pub start_node_id: Option<String>,
    pub resume_from_failure: bool,
    pub input_overrides: BTreeMap<String, Value>,
    pub mode: ExecutionMode,
    pub previous_run: Option<WorkflowRun>,
}

impl Default for ExecutionOptions {
    fn default() -> Self {
        Self {
            start_node_id: None,
            resume_from_failure: false,
            input_overrides: BTreeMap::new(),
            mode: ExecutionMode::Auto,
            previous_run: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum StepState {
    Auto,
    AwaitingStep,
    StepRequested,
    Cancelled,
}

pub struct RunController {
    cancel: Arc<AtomicBool>,
    step_gate: Arc<(Mutex<StepState>, Condvar)>,
}

impl RunController {
    pub fn new(mode: &ExecutionMode) -> Self {
        let initial = match mode {
            ExecutionMode::Auto => StepState::Auto,
            ExecutionMode::Step => StepState::AwaitingStep,
        };
        Self {
            cancel: Arc::new(AtomicBool::new(false)),
            step_gate: Arc::new((Mutex::new(initial), Condvar::new())),
        }
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
        let (lock, cvar) = &*self.step_gate;
        let mut state = lock.lock().expect("step gate lock should not poison");
        *state = StepState::Cancelled;
        cvar.notify_all();
    }

    pub fn request_step(&self) {
        let (lock, cvar) = &*self.step_gate;
        let mut state = lock.lock().expect("step gate lock should not poison");
        if *state == StepState::AwaitingStep {
            *state = StepState::StepRequested;
            cvar.notify_one();
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }
}

impl Default for NodeDefinition {
    fn default() -> Self {
        Self {
            id: String::new(),
            kind: NodeKind::Constant,
            value: Value::Null,
            template: String::new(),
            duration_ms: 0,
            error: String::new(),
            prompt: String::new(),
            model: None,
            cwd: None,
            extra_args: Vec::new(),
            script: String::new(),
            sandbox_policy: Value::Null,
        }
    }
}

pub fn crate_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}
