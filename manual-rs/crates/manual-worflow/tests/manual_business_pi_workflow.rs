use std::env;
use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use manual_agent::{Agent, pi::Pi};
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

struct CompiledRustScriptRunner;

impl ScriptRunner for CompiledRustScriptRunner {
    fn run_script(
        &self,
        script: &RustScript,
        input: &NodeInput,
    ) -> Result<WorkflowValue, WorkflowError> {
        let workspace = unique_temp_dir();
        fs::create_dir_all(&workspace).map_err(io_error)?;

        for module in script.modules() {
            fs::write(
                workspace.join(format!("{}.rs", module.name())),
                module.source(),
            )
            .map_err(io_error)?;
        }

        let module_declarations = script
            .modules()
            .iter()
            .map(|module| format!("mod {};\n", module.name()))
            .collect::<String>();
        let main_rs = workspace.join("main.rs");
        fs::write(
            &main_rs,
            format!("{module_declarations}\n{}", script.source()),
        )
        .map_err(io_error)?;

        let binary = workspace.join("script-bin");
        let compile = Command::new("rustc")
            .arg("--edition=2024")
            .arg(&main_rs)
            .arg("-o")
            .arg(&binary)
            .output()
            .map_err(io_error)?;

        if !compile.status.success() {
            return Err(WorkflowError::AgentCommandFailed {
                status_code: compile.status.code(),
                stderr: String::from_utf8_lossy(&compile.stderr).into_owned(),
            });
        }

        let run = Command::new(&binary)
            .env("MANUAL_WORKFLOW_INPUT", format!("{:?}", input.values()))
            .output()
            .map_err(io_error)?;

        if !run.status.success() {
            return Err(WorkflowError::AgentCommandFailed {
                status_code: run.status.code(),
                stderr: String::from_utf8_lossy(&run.stderr).into_owned(),
            });
        }

        Ok(parse_script_stdout(&String::from_utf8_lossy(&run.stdout)))
    }
}

#[test]
#[ignore = "manual test: compiles Rust scripts and calls the local pi CLI"]
fn business_pipeline_health_workflow_runs_with_real_scripts_and_pi_agent() {
    let mut workflow = Workflow::new();

    workflow
        .add_task(
            "weekly_context",
            ConstantNode {
                value: object([
                    ("week", WorkflowValue::String("2026-W19".into())),
                    ("business", WorkflowValue::String("B2B SaaS".into())),
                    (
                        "goal",
                        WorkflowValue::String("Decide one light intervention for next week".into()),
                    ),
                ]),
            },
        )
        .unwrap();

    workflow
        .add_task(
            "sales_health",
            ScriptNode::new(
                RustScript::new(
                    r#"
fn main() {
    let leads = 128.0;
    let qualified = 42.0;
    let demos = 18.0;
    let conversion = scoring::percent(qualified, leads);
    let demo_rate = scoring::percent(demos, qualified);

    println!("metric=sales_pipeline");
    println!("leads={leads}");
    println!("qualified={qualified}");
    println!("demos={demos}");
    println!("conversion_rate={conversion:.1}");
    println!("demo_rate={demo_rate:.1}");
    println!("signal=lead quality is acceptable but demo booking needs attention");
}
"#,
                )
                .with_module(ScriptModule::new(
                    "scoring",
                    r#"
pub fn percent(part: f64, whole: f64) -> f64 {
    if whole == 0.0 { 0.0 } else { part / whole * 100.0 }
}
"#,
                )),
                CompiledRustScriptRunner,
            ),
        )
        .unwrap();

    workflow
        .add_task(
            "support_health",
            ScriptNode::new(
                RustScript::new(
                    r#"
fn main() {
    let open_tickets = 37.0;
    let stale_tickets = 9.0;
    let stale_rate = support_math::percent(stale_tickets, open_tickets);

    println!("metric=support_queue");
    println!("open_tickets={open_tickets}");
    println!("stale_tickets={stale_tickets}");
    println!("stale_rate={stale_rate:.1}");
    println!("signal=stale tickets are the clearest retention risk");
}
"#,
                )
                .with_module(ScriptModule::new(
                    "support_math",
                    r#"
pub fn percent(part: f64, whole: f64) -> f64 {
    if whole == 0.0 { 0.0 } else { part / whole * 100.0 }
}
"#,
                )),
                CompiledRustScriptRunner,
            ),
        )
        .unwrap();

    workflow
        .add_task(
            "pi_recommendation",
            ManualAgentNode::new(
                Pi::new(Agent::new(
                    "pi.pipeline_advisor",
                    "Pipeline Advisor",
                    "Give concise operational recommendations.",
                )),
                "You are reviewing a tiny weekly business health workflow. Based only on the input, return exactly two short bullet points: one risk and one next action.",
            ),
        )
        .unwrap();

    workflow
        .add_task(
            "operator_digest",
            ScriptNode::new(
                RustScript::new(
                    r#"
fn main() {
    let input = std::env::var("MANUAL_WORKFLOW_INPUT").unwrap_or_default();
    println!("digest=weekly pipeline health digest");
    println!("input_bytes={}", input.len());
    println!("ready_for_ops=true");
}
"#,
                ),
                CompiledRustScriptRunner,
            ),
        )
        .unwrap();

    workflow
        .add_dependency("sales_health", "weekly_context")
        .unwrap();
    workflow
        .add_dependency("support_health", "weekly_context")
        .unwrap();
    workflow
        .add_dependency("pi_recommendation", "sales_health")
        .unwrap();
    workflow
        .add_dependency("pi_recommendation", "support_health")
        .unwrap();
    workflow
        .add_dependency("operator_digest", "weekly_context")
        .unwrap();
    workflow
        .add_dependency("operator_digest", "sales_health")
        .unwrap();
    workflow
        .add_dependency("operator_digest", "support_health")
        .unwrap();
    workflow
        .add_dependency("operator_digest", "pi_recommendation")
        .unwrap();

    let plan = workflow.execution_plan().unwrap();
    println!("business workflow stages: {:?}", plan.stages());

    let output = workflow.execute().unwrap();
    println!("business workflow output: {:#?}", output.values());

    assert!(matches!(
        output.get("pi_recommendation"),
        Some(WorkflowValue::Object(_))
    ));
    let Some(WorkflowValue::Object(digest)) = output.get("operator_digest") else {
        panic!("operator_digest should return an object");
    };

    assert_eq!(
        digest.get("digest"),
        Some(&WorkflowValue::String(
            "weekly pipeline health digest".into()
        ))
    );
    assert_eq!(
        digest.get("ready_for_ops"),
        Some(&WorkflowValue::Bool(true))
    );
    assert!(matches!(
        digest.get("input_bytes"),
        Some(WorkflowValue::Number(bytes)) if *bytes > 0.0
    ));
}

fn object(entries: impl IntoIterator<Item = (&'static str, WorkflowValue)>) -> WorkflowValue {
    WorkflowValue::Object(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}

fn parse_script_stdout(stdout: &str) -> WorkflowValue {
    WorkflowValue::Object(
        stdout
            .lines()
            .filter_map(|line| line.split_once('='))
            .map(|(key, value)| (key.to_owned(), parse_value(value)))
            .collect(),
    )
}

fn parse_value(value: &str) -> WorkflowValue {
    match value {
        "true" => WorkflowValue::Bool(true),
        "false" => WorkflowValue::Bool(false),
        _ => value
            .parse::<f64>()
            .map(WorkflowValue::Number)
            .unwrap_or_else(|_| WorkflowValue::String(value.to_owned())),
    }
}

fn unique_temp_dir() -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();

    env::temp_dir().join(format!("manual-workflow-script-{nanos}"))
}

fn io_error(error: std::io::Error) -> WorkflowError {
    WorkflowError::AgentCommandIo {
        message: error.to_string(),
    }
}
