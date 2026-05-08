use std::process::Command;

use manual_agent::{Agent, AgentCommand, CommandRequest};
use manual_worflow::{
    ManualAgentNode, NodeInput, RustScript, ScriptModule, ScriptNode, ScriptRunner, Workflow,
    WorkflowError, WorkflowNode, WorkflowValue,
};

struct ConstantNode {
    value: WorkflowValue,
}

impl WorkflowNode for ConstantNode {
    fn run(&self, _input: &NodeInput) -> Result<WorkflowValue, WorkflowError> {
        Ok(self.value.clone())
    }
}

struct DemoScriptRunner;

impl ScriptRunner for DemoScriptRunner {
    fn run_script(
        &self,
        script: &RustScript,
        input: &NodeInput,
    ) -> Result<WorkflowValue, WorkflowError> {
        match script.source() {
            "collect_metrics" => Ok(object([
                ("visitors", WorkflowValue::Number(1200.0)),
                ("conversions", WorkflowValue::Number(84.0)),
                (
                    "module",
                    WorkflowValue::String(script.modules()[0].name().to_owned()),
                ),
                (
                    "topic",
                    lookup_object_string(input, "brief", "topic").unwrap_or(WorkflowValue::Null),
                ),
            ])),
            "collect_notes" => Ok(WorkflowValue::List(vec![
                WorkflowValue::String("signup traffic rose".into()),
                WorkflowValue::String("docs page has high drop-off".into()),
                WorkflowValue::String("agent should recommend one focused action".into()),
            ])),
            "render_report" => Ok(object([
                (
                    "title",
                    lookup_object_string(input, "brief", "topic").unwrap_or(WorkflowValue::Null),
                ),
                (
                    "summary",
                    lookup_agent_stdout(input, "agent_summary").unwrap_or(WorkflowValue::Null),
                ),
                (
                    "inputs_seen",
                    WorkflowValue::List(vec![
                        WorkflowValue::String("brief".into()),
                        WorkflowValue::String("metrics".into()),
                        WorkflowValue::String("notes".into()),
                        WorkflowValue::String("agent_summary".into()),
                    ]),
                ),
            ])),
            other => Ok(WorkflowValue::String(format!("unhandled script: {other}"))),
        }
    }
}

struct DemoAgentCommand {
    agent: Agent,
}

impl DemoAgentCommand {
    fn new() -> Self {
        Self {
            agent: Agent::new(
                "manual-agent.demo",
                "Manual Agent Demo",
                "Summarize workflow input deterministically.",
            ),
        }
    }
}

impl AgentCommand for DemoAgentCommand {
    fn agent(&self) -> &Agent {
        &self.agent
    }

    fn command(&self, request: &CommandRequest) -> Command {
        let mut command = Command::new("sh");
        command.args([
            "-c",
            "printf 'agent-summary: %s' \"$1\"",
            "sh",
            request.prompt(),
        ]);
        command
    }
}

#[test]
fn manual_complex_workflow_runs_end_to_end() {
    let mut workflow = Workflow::new();

    workflow
        .add_task(
            "brief",
            ConstantNode {
                value: object([
                    ("topic", WorkflowValue::String("May growth review".into())),
                    ("audience", WorkflowValue::String("operators".into())),
                ]),
            },
        )
        .unwrap();

    workflow
        .add_task(
            "metrics",
            ScriptNode::new(
                RustScript::new("collect_metrics").with_module(ScriptModule::new(
                    "analytics",
                    "pub fn conversion_rate(visitors: f64, conversions: f64) -> f64 { conversions / visitors }",
                )),
                DemoScriptRunner,
            ),
        )
        .unwrap();

    workflow
        .add_task(
            "notes",
            ScriptNode::new(
                RustScript::new("collect_notes").with_module(ScriptModule::new(
                    "research",
                    "pub fn notes() -> Vec<String> { vec![] }",
                )),
                DemoScriptRunner,
            ),
        )
        .unwrap();

    workflow
        .add_task(
            "agent_summary",
            ManualAgentNode::new(DemoAgentCommand::new(), "Summarize metrics and notes."),
        )
        .unwrap();

    workflow
        .add_task(
            "report",
            ScriptNode::new(
                RustScript::new("render_report").with_module(ScriptModule::new(
                    "reporting",
                    "pub fn render() -> WorkflowValue { WorkflowValue::Null }",
                )),
                DemoScriptRunner,
            ),
        )
        .unwrap();

    workflow.add_dependency("metrics", "brief").unwrap();
    workflow.add_dependency("notes", "brief").unwrap();
    workflow.add_dependency("agent_summary", "metrics").unwrap();
    workflow.add_dependency("agent_summary", "notes").unwrap();
    workflow.add_dependency("report", "brief").unwrap();
    workflow.add_dependency("report", "metrics").unwrap();
    workflow.add_dependency("report", "notes").unwrap();
    workflow.add_dependency("report", "agent_summary").unwrap();

    let plan = workflow.execution_plan().unwrap();
    println!("execution stages: {:?}", plan.stages());

    let output = workflow.execute().unwrap();
    println!("workflow output: {:#?}", output.values());

    assert_eq!(
        output.get("report"),
        Some(&object([
            ("title", WorkflowValue::String("May growth review".into())),
            (
                "summary",
                WorkflowValue::String(
                    "agent-summary: Summarize metrics and notes.\n\nInput: {NodeId(\"metrics\"): Object({\"conversions\": Number(84.0), \"module\": String(\"analytics\"), \"topic\": String(\"May growth review\"), \"visitors\": Number(1200.0)}), NodeId(\"notes\"): List([String(\"signup traffic rose\"), String(\"docs page has high drop-off\"), String(\"agent should recommend one focused action\")])}".into(),
                ),
            ),
            (
                "inputs_seen",
                WorkflowValue::List(vec![
                    WorkflowValue::String("brief".into()),
                    WorkflowValue::String("metrics".into()),
                    WorkflowValue::String("notes".into()),
                    WorkflowValue::String("agent_summary".into()),
                ]),
            ),
        ]))
    );
}

fn object(entries: impl IntoIterator<Item = (&'static str, WorkflowValue)>) -> WorkflowValue {
    WorkflowValue::Object(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}

fn lookup_object_string(input: &NodeInput, node: &str, field: &str) -> Option<WorkflowValue> {
    let WorkflowValue::Object(values) = input.get(node)? else {
        return None;
    };

    values.get(field).cloned()
}

fn lookup_agent_stdout(input: &NodeInput, node: &str) -> Option<WorkflowValue> {
    let WorkflowValue::Object(values) = input.get(node)? else {
        return None;
    };

    values.get("stdout").cloned()
}
