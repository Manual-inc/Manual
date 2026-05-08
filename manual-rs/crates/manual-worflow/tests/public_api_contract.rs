use manual_agent::{Agent, AgentCommand, CommandRequest};
use manual_worflow::{
    MAX_NODE_ID_LEN, ManualAgentNode, NodeId, NodeInput, NodeKind, RustScript, ScriptModule,
    ScriptNode, ScriptRunner, Workflow, WorkflowDefinition, WorkflowError, WorkflowNode,
    WorkflowValue,
};
use std::process::Command;

#[test]
fn node_id_rejects_empty_whitespace_and_overlong_values() {
    assert_eq!(NodeId::new(""), Err(WorkflowError::EmptyNodeId));
    assert_eq!(NodeId::new("   "), Err(WorkflowError::EmptyNodeId));

    let too_long = "x".repeat(MAX_NODE_ID_LEN + 1);
    assert_eq!(
        NodeId::new(too_long),
        Err(WorkflowError::NodeIdTooLong {
            max: MAX_NODE_ID_LEN
        })
    );
}

#[test]
fn workflow_rejects_duplicate_nodes() {
    let mut workflow = Workflow::new();

    workflow.add_node("fetch").unwrap();

    assert_eq!(
        workflow.add_node("fetch"),
        Err(WorkflowError::DuplicateNode {
            node: NodeId::new("fetch").unwrap()
        })
    );
}

#[test]
fn workflow_rejects_dependencies_for_unknown_nodes() {
    let mut workflow = Workflow::new();

    workflow.add_node("parse").unwrap();

    assert_eq!(
        workflow.add_dependency("parse", "fetch"),
        Err(WorkflowError::UnknownNode {
            node: NodeId::new("fetch").unwrap()
        })
    );
}

#[test]
fn execution_plan_groups_independent_nodes_into_stages() {
    let mut workflow = Workflow::new();

    workflow.add_node("fetch_profile").unwrap();
    workflow.add_node("fetch_repos").unwrap();
    workflow.add_node("merge").unwrap();
    workflow.add_node("publish").unwrap();
    workflow.add_dependency("merge", "fetch_profile").unwrap();
    workflow.add_dependency("merge", "fetch_repos").unwrap();
    workflow.add_dependency("publish", "merge").unwrap();

    let plan = workflow.execution_plan().unwrap();

    assert_eq!(
        plan.stages(),
        &[
            vec![
                NodeId::new("fetch_profile").unwrap(),
                NodeId::new("fetch_repos").unwrap(),
            ],
            vec![NodeId::new("merge").unwrap()],
            vec![NodeId::new("publish").unwrap()],
        ]
    );
}

#[test]
fn workflow_definition_deserializes_api_shape_and_produces_execution_plan() {
    let definition: WorkflowDefinition = serde_json::from_value(serde_json::json!({
        "id": "lead-review",
        "nodes": [
            {
                "id": "lead_payload",
                "kind": "constant",
                "value": {
                    "lead_count": 128,
                    "qualified_count": 42
                }
            },
            {
                "id": "score",
                "kind": "template",
                "template": "qualified leads: {{lead_payload.qualified_count}} / {{lead_payload.lead_count}}"
            }
        ],
        "dependencies": [
            {
                "node": "score",
                "depends_on": "lead_payload"
            }
        ]
    }))
    .unwrap();

    assert_eq!(definition.id, "lead-review");
    assert_eq!(definition.nodes.len(), 2);
    assert_eq!(definition.nodes[0].kind, NodeKind::Constant);
    assert_eq!(
        definition.execution_plan().unwrap().stages(),
        &[
            vec![NodeId::new("lead_payload").unwrap()],
            vec![NodeId::new("score").unwrap()]
        ]
    );
}

#[test]
fn workflow_definition_executes_api_nodes_and_emits_run_events() {
    let definition: WorkflowDefinition = serde_json::from_value(serde_json::json!({
        "id": "lead-review",
        "nodes": [
            {
                "id": "lead_payload",
                "kind": "constant",
                "value": {
                    "lead_count": 128,
                    "qualified_count": 42
                }
            },
            {
                "id": "score",
                "kind": "template",
                "template": "qualified leads: {{lead_payload.qualified_count}} / {{lead_payload.lead_count}}"
            }
        ],
        "dependencies": [
            {
                "node": "score",
                "depends_on": "lead_payload"
            }
        ]
    }))
    .unwrap();

    let run = definition.execute("run-1").unwrap();

    assert!(run.completed());
    assert_eq!(
        run.events(),
        &[
            serde_json::json!({
                "run_id": "run-1",
                "sequence": 0,
                "type": "workflow_started",
                "workflow_id": "lead-review"
            }),
            serde_json::json!({
                "run_id": "run-1",
                "sequence": 1,
                "type": "node_started",
                "node_id": "lead_payload"
            }),
            serde_json::json!({
                "run_id": "run-1",
                "sequence": 2,
                "type": "node_completed",
                "node_id": "lead_payload",
                "result": {
                    "lead_count": 128,
                    "qualified_count": 42
                }
            }),
            serde_json::json!({
                "run_id": "run-1",
                "sequence": 3,
                "type": "node_started",
                "node_id": "score"
            }),
            serde_json::json!({
                "run_id": "run-1",
                "sequence": 4,
                "type": "node_completed",
                "node_id": "score",
                "result": "qualified leads: 42 / 128"
            }),
            serde_json::json!({
                "run_id": "run-1",
                "sequence": 5,
                "type": "workflow_completed",
                "workflow_id": "lead-review"
            })
        ]
    );
}

#[test]
fn workflow_definition_emits_failure_events_when_a_node_fails() {
    let definition: WorkflowDefinition = serde_json::from_value(serde_json::json!({
        "id": "failing-review",
        "nodes": [
            {
                "id": "explode",
                "kind": "fail",
                "error": "boom"
            },
            {
                "id": "after",
                "kind": "template",
                "template": "unreachable"
            }
        ],
        "dependencies": [
            {
                "node": "after",
                "depends_on": "explode"
            }
        ]
    }))
    .unwrap();
    let mut events = Vec::new();

    let error = definition
        .execute_with_events("run-1", |event| events.push(event))
        .unwrap_err();

    assert_eq!(
        error,
        WorkflowError::NodeExecutionFailed {
            node: NodeId::new("explode").unwrap(),
            message: "boom".into(),
        }
    );
    assert_eq!(
        events,
        [
            serde_json::json!({
                "run_id": "run-1",
                "sequence": 0,
                "type": "workflow_started",
                "workflow_id": "failing-review"
            }),
            serde_json::json!({
                "run_id": "run-1",
                "sequence": 1,
                "type": "node_started",
                "node_id": "explode"
            }),
            serde_json::json!({
                "run_id": "run-1",
                "sequence": 2,
                "type": "node_failed",
                "node_id": "explode",
                "error": "boom"
            }),
            serde_json::json!({
                "run_id": "run-1",
                "sequence": 3,
                "type": "workflow_failed",
                "workflow_id": "failing-review",
                "error": "boom"
            })
        ]
    );
}

struct ConstantNode {
    value: WorkflowValue,
}

impl WorkflowNode for ConstantNode {
    fn run(&self, _input: &NodeInput) -> Result<WorkflowValue, WorkflowError> {
        Ok(self.value.clone())
    }
}

struct JoinNode;

impl WorkflowNode for JoinNode {
    fn run(&self, input: &NodeInput) -> Result<WorkflowValue, WorkflowError> {
        let profile = input.get("fetch_profile").unwrap();
        let repos = input.get("fetch_repos").unwrap();

        Ok(WorkflowValue::String(format!("{profile:?}+{repos:?}")))
    }
}

#[test]
fn execute_passes_upstream_outputs_to_dependent_nodes() {
    let mut workflow = Workflow::new();

    workflow
        .add_task(
            "fetch_profile",
            ConstantNode {
                value: WorkflowValue::String("profile".into()),
            },
        )
        .unwrap();
    workflow
        .add_task(
            "fetch_repos",
            ConstantNode {
                value: WorkflowValue::List(vec![
                    WorkflowValue::String("manual".into()),
                    WorkflowValue::String("sandbox".into()),
                ]),
            },
        )
        .unwrap();
    workflow.add_task("merge", JoinNode).unwrap();
    workflow.add_dependency("merge", "fetch_profile").unwrap();
    workflow.add_dependency("merge", "fetch_repos").unwrap();

    let output = workflow.execute().unwrap();

    assert_eq!(
        output.get("fetch_profile"),
        Some(&WorkflowValue::String("profile".into()))
    );
    assert_eq!(
        output.get("merge"),
        Some(&WorkflowValue::String(
            "String(\"profile\")+List([String(\"manual\"), String(\"sandbox\")])".into()
        ))
    );
}

struct InspectingScriptRunner;

impl ScriptRunner for InspectingScriptRunner {
    fn run_script(
        &self,
        script: &RustScript,
        input: &NodeInput,
    ) -> Result<WorkflowValue, WorkflowError> {
        let upstream = input.get("source").unwrap();

        Ok(WorkflowValue::Object(
            [
                (
                    "source".into(),
                    WorkflowValue::String(script.source().to_owned()),
                ),
                (
                    "module".into(),
                    WorkflowValue::String(script.modules()[0].name().to_owned()),
                ),
                ("upstream".into(), upstream.clone()),
            ]
            .into(),
        ))
    }
}

#[test]
fn script_node_executes_rust_script_with_dependency_modules_and_input() {
    let mut workflow = Workflow::new();
    let script = RustScript::new("pub fn run() -> WorkflowValue { helper::value() }").with_module(
        ScriptModule::new("helper", "pub fn value() -> WorkflowValue { ... }"),
    );

    workflow
        .add_task(
            "source",
            ConstantNode {
                value: WorkflowValue::String("input value".into()),
            },
        )
        .unwrap();
    workflow
        .add_task("script", ScriptNode::new(script, InspectingScriptRunner))
        .unwrap();
    workflow.add_dependency("script", "source").unwrap();

    let output = workflow.execute().unwrap();

    assert_eq!(
        output.get("script"),
        Some(&WorkflowValue::Object(
            [
                (
                    "source".into(),
                    WorkflowValue::String(
                        "pub fn run() -> WorkflowValue { helper::value() }".into()
                    ),
                ),
                ("module".into(), WorkflowValue::String("helper".into())),
                (
                    "upstream".into(),
                    WorkflowValue::String("input value".into())
                ),
            ]
            .into()
        ))
    );
}

struct EchoAgentCommand {
    agent: Agent,
}

impl EchoAgentCommand {
    fn new() -> Self {
        Self {
            agent: Agent::new("echo", "Echo Agent", "Echo prompt for tests."),
        }
    }
}

impl AgentCommand for EchoAgentCommand {
    fn agent(&self) -> &Agent {
        &self.agent
    }

    fn command(&self, request: &CommandRequest) -> Command {
        let mut command = Command::new("sh");
        command.args(["-c", "printf %s \"$1\"", "sh", request.prompt()]);
        command
    }
}

#[test]
fn manual_agent_node_executes_agent_command_with_upstream_input() {
    let mut workflow = Workflow::new();

    workflow
        .add_task(
            "source",
            ConstantNode {
                value: WorkflowValue::String("agent input".into()),
            },
        )
        .unwrap();
    workflow
        .add_task(
            "agent",
            ManualAgentNode::new(EchoAgentCommand::new(), "Summarize the input"),
        )
        .unwrap();
    workflow.add_dependency("agent", "source").unwrap();

    let output = workflow.execute().unwrap();

    assert_eq!(
        output.get("agent"),
        Some(&WorkflowValue::Object(
            [
                ("status_code".into(), WorkflowValue::Number(0.0)),
                (
                    "stdout".into(),
                    WorkflowValue::String(
                        "Summarize the input\n\nInput: {NodeId(\"source\"): String(\"agent input\")}"
                            .into()
                    ),
                ),
                ("stderr".into(), WorkflowValue::String(String::new())),
            ]
            .into()
        ))
    );
}
