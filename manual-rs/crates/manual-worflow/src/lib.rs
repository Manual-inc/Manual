use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use manual_agent::{Agent, AgentCommand, CommandRequest, pi::Pi};
use serde::Deserialize;
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

#[derive(Clone, Debug, Deserialize, PartialEq)]
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
        mut emit: impl FnMut(Value),
    ) -> Result<(), WorkflowError> {
        let mut sequence = 0;
        let mut outputs = BTreeMap::new();

        emit_event(
            &mut sequence,
            &mut emit,
            json!({
                "run_id": run_id,
                "type": "workflow_started",
                "workflow_id": self.id,
            }),
        );

        let plan = match self.execution_plan() {
            Ok(plan) => plan,
            Err(error) => {
                emit_workflow_failed(run_id, &self.id, &mut sequence, &mut emit, &error);
                return Err(error);
            }
        };

        for stage in plan.stages() {
            let stage_nodes = stage
                .iter()
                .map(|node_id| {
                    self.nodes
                        .iter()
                        .find(|node| node.id == node_id.as_str())
                        .expect("execution plan should only include defined nodes")
                })
                .collect::<Vec<_>>();

            for node in &stage_nodes {
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
                for node in stage_nodes {
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
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
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
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Constant,
    Delay,
    Fail,
    Pi,
    Template,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct DependencyDefinition {
    pub node: String,
    pub depends_on: String,
}

#[derive(Clone, Debug, PartialEq)]
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
        if event["type"] == "workflow_completed" || event["type"] == "workflow_failed" {
            self.completed = true;
        }

        self.events.push(event);
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
        NodeKind::Pi => execute_pi_node(node, outputs),
        NodeKind::Template => Ok(render_template(&node.template, outputs).into()),
    }
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
    let prompt = format!("{}\n\nInput: {}", node.prompt, json!(outputs));
    let mut request = CommandRequest::new(prompt);

    if let Some(model) = node.model.as_deref().filter(|model| !model.is_empty()) {
        request = request.with_model(model);
    }

    let output = pi
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

pub fn crate_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}
