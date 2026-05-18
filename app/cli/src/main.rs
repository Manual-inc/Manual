use std::env;
use std::fmt;
use std::fs;
use std::io::{self, BufRead, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use clap::{Parser, Subcommand};
use serde_json::{Map, Value, json};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), CliError> {
    let cli = Cli::parse();
    let mut client = if let Some(server_bin) = cli.server_bin.as_deref() {
        AppServerClient::stdio(resolve_server_bin(Some(server_bin))?)?
    } else if let Some(server_url) = cli.server_url {
        let auth_token = cli.auth_token.ok_or_else(|| {
            CliError::InvalidResponse("--auth-token is required with --server-url".to_owned())
        })?;
        AppServerClient::http(server_url, auth_token)
    } else {
        AppServerClient::daemon(
            resolve_server_bin(None)?,
            cli.discovery_file.unwrap_or_else(default_discovery_file),
        )?
    };

    match cli.command {
        CommandGroup::Workflow { command } => handle_workflow(command, &mut client),
        CommandGroup::Node { command } => handle_node(command, &mut client),
        CommandGroup::Agent { command } => handle_agent(command, &mut client),
        CommandGroup::Manual { command } => handle_manual(command, &mut client),
        CommandGroup::Optimization { command } => handle_optimization(command, &mut client),
        CommandGroup::Sandbox { command } => handle_sandbox(command, &mut client),
        CommandGroup::Skill { command } => handle_skill(command, &mut client),
        CommandGroup::Rpc { method, params } => {
            let params = match params {
                Some(path) => read_json_file(&path)?,
                None => Value::Null,
            };
            print_json(&client.request(&method, params)?)
        }
    }
}

fn handle_workflow(command: WorkflowCommand, client: &mut AppServerClient) -> Result<(), CliError> {
    match command {
        WorkflowCommand::Create { workflow } => {
            let workflow = read_json_file(&workflow)?;
            request_and_print(client, "workflow.create", json!({ "workflow": workflow }))
        }
        WorkflowCommand::Get { workflow_id } => request_and_print(
            client,
            "workflow.get",
            json!({ "workflow_id": workflow_id }),
        ),
        WorkflowCommand::List => request_and_print(client, "workflow.list", Value::Null),
        WorkflowCommand::Update {
            workflow_id,
            workflow,
        } => {
            let workflow = read_json_file(&workflow)?;
            request_and_print(
                client,
                "workflow.update",
                json!({
                    "workflow_id": workflow_id,
                    "workflow": workflow,
                }),
            )
        }
        WorkflowCommand::Patch {
            workflow_id,
            operations,
        } => {
            let operations = read_json_file(&operations)?;
            request_and_print(
                client,
                "workflow.patch",
                json!({
                    "workflow_id": workflow_id,
                    "operations": operations,
                }),
            )
        }
        WorkflowCommand::Delete { workflow_id } => request_and_print(
            client,
            "workflow.delete",
            json!({ "workflow_id": workflow_id }),
        ),
        WorkflowCommand::Start {
            workflow_id,
            start_node,
            resume_from_failure,
            inputs,
            mode,
            resume_run_id,
        } => {
            let input_overrides = read_optional_json_object(inputs.as_ref())?;
            let mut params = json!({
                "workflow_id": workflow_id,
                "resume_from_failure": resume_from_failure,
                "input_overrides": input_overrides,
                "mode": mode,
            });
            if let Some(node) = start_node {
                params["start_node_id"] = json!(node);
            }
            if let Some(rid) = resume_run_id {
                params["resume_run_id"] = json!(rid);
            }
            request_and_print(client, "workflow.start", params)
        }
        WorkflowCommand::Events {
            run_id,
            cursor,
            watch,
            interval_ms,
        } => print_events(client, run_id, cursor, watch, interval_ms),
        WorkflowCommand::Run {
            workflow_id,
            interval_ms,
            start_node,
            resume_from_failure,
            inputs,
            mode,
            resume_run_id,
        } => {
            let input_overrides = read_optional_json_object(inputs.as_ref())?;
            let mut params = json!({
                "workflow_id": workflow_id,
                "resume_from_failure": resume_from_failure,
                "input_overrides": input_overrides,
                "mode": mode,
            });
            if let Some(node) = start_node {
                params["start_node_id"] = json!(node);
            }
            if let Some(rid) = resume_run_id {
                params["resume_run_id"] = json!(rid);
            }
            let started = client.request("workflow.start", params)?;
            print_json(&started)?;
            let run_id = started
                .get("run_id")
                .and_then(Value::as_str)
                .ok_or(CliError::InvalidResponse("missing run_id".to_owned()))?
                .to_owned();
            print_events(client, run_id, 0, true, interval_ms)
        }
        WorkflowCommand::Stop { run_id } => {
            let result = client.request("workflow.stop", json!({ "run_id": run_id }))?;
            print_json(&result)
        }
        WorkflowCommand::Resume {
            run_id,
            start_node,
            resume_from_failure,
            inputs,
            mode,
        } => {
            let input_overrides = read_optional_json_object(inputs.as_ref())?;
            let mut params = json!({
                "run_id": run_id,
                "resume_from_failure": resume_from_failure,
                "input_overrides": input_overrides,
                "mode": mode,
            });
            if let Some(node) = start_node {
                params["start_node_id"] = json!(node);
            }
            request_and_print(client, "workflow.resume", params)
        }
        WorkflowCommand::ComposeFromRegistry { node_id } => request_and_print(
            client,
            "workflow.compose_from_registry",
            json!({ "node_id": node_id }),
        ),
    }
}

fn handle_node(command: NodeCommand, client: &mut AppServerClient) -> Result<(), CliError> {
    match command {
        NodeCommand::Create {
            node,
            name,
            description,
        } => {
            let mut params = Map::new();
            params.insert("node".to_owned(), read_json_file(&node)?);
            if let Some(name) = name {
                params.insert("name".to_owned(), json!(name));
            }
            if let Some(description) = description {
                params.insert("description".to_owned(), json!(description));
            }
            request_and_print(client, "node.create", Value::Object(params))
        }
        NodeCommand::Get { node_id } => {
            request_and_print(client, "node.get", json!({ "node_id": node_id }))
        }
        NodeCommand::List => request_and_print(client, "node.list", Value::Null),
        NodeCommand::Update {
            node_id,
            node,
            name,
            description,
        } => {
            let mut params = Map::new();
            params.insert("node_id".to_owned(), json!(node_id));
            if let Some(node) = node {
                params.insert("node".to_owned(), read_json_file(&node)?);
            }
            if let Some(name) = name {
                params.insert("name".to_owned(), json!(name));
            }
            if let Some(description) = description {
                params.insert("description".to_owned(), json!(description));
            }
            request_and_print(client, "node.update", Value::Object(params))
        }
        NodeCommand::Delete { node_id } => {
            request_and_print(client, "node.delete", json!({ "node_id": node_id }))
        }
        NodeCommand::Schema { kind } => {
            request_and_print(client, "node.schema", json!({ "kind": kind }))
        }
        NodeCommand::Run { node, inputs } => {
            let mut params = Map::new();
            params.insert("node".to_owned(), read_json_file(&node)?);
            if let Some(inputs) = inputs {
                params.insert("inputs".to_owned(), read_json_file(&inputs)?);
            }
            request_and_print(client, "node.run", Value::Object(params))
        }
        NodeCommand::RunGet { run_id } => {
            request_and_print(client, "node.run.get", json!({ "run_id": run_id }))
        }
        NodeCommand::RunEvents { run_id, cursor } => request_and_print(
            client,
            "node.run.events",
            json!({
                "run_id": run_id,
                "cursor": cursor,
            }),
        ),
        NodeCommand::TestcaseSave {
            run_id,
            expected_output,
            criteria,
        } => {
            let mut params = Map::new();
            params.insert("run_id".to_owned(), json!(run_id));
            if let Some(expected_output) = expected_output {
                params.insert(
                    "expected_output".to_owned(),
                    read_json_file(&expected_output)?,
                );
            }
            if let Some(criteria) = criteria {
                params.insert("criteria".to_owned(), read_json_file(&criteria)?);
            }
            request_and_print(client, "node.testcase.save", Value::Object(params))
        }
        NodeCommand::TestcaseVerify { node_id } => request_and_print(
            client,
            "node.testcase.verify",
            json!({ "node_id": node_id }),
        ),
    }
}

fn handle_agent(command: AgentCommand, client: &mut AppServerClient) -> Result<(), CliError> {
    match command {
        AgentCommand::List { params } => {
            request_and_print(client, "agent.list", read_optional_json(params.as_ref())?)
        }
    }
}

fn handle_manual(command: ManualCommand, client: &mut AppServerClient) -> Result<(), CliError> {
    match command {
        ManualCommand::Create { params } => {
            request_and_print(client, "manual.create", read_json_file(&params)?)
        }
        ManualCommand::Get { manual_id } => request_and_print(
            client,
            "manual.get",
            params_with_optional_id("manual_id", manual_id),
        ),
        ManualCommand::List {
            status,
            query,
            tag,
            params,
        } => {
            let mut payload = read_optional_json_map(params.as_ref())?;
            insert_optional_string(&mut payload, "status", status);
            insert_optional_string(&mut payload, "query", query);
            insert_optional_string(&mut payload, "tag", tag);
            request_and_print(client, "manual.list", Value::Object(payload))
        }
        ManualCommand::Update {
            manual_id,
            changes,
            execution_affecting,
            params,
        } => {
            let mut payload = read_optional_json_map(params.as_ref())?;
            payload.insert("manual_id".to_owned(), json!(manual_id));
            if let Some(changes) = changes {
                payload.insert("changes".to_owned(), read_json_file(&changes)?);
            }
            if execution_affecting {
                payload.insert("execution_affecting".to_owned(), Value::Bool(true));
            }
            request_and_print(client, "manual.update", Value::Object(payload))
        }
        ManualCommand::Clone { manual_id } => request_and_print(
            client,
            "manual.clone",
            params_with_optional_id("manual_id", manual_id),
        ),
        ManualCommand::Archive { manual_id } => request_and_print(
            client,
            "manual.archive",
            params_with_optional_id("manual_id", manual_id),
        ),
        ManualCommand::Delete { manual_id } => request_and_print(
            client,
            "manual.delete",
            params_with_optional_id("manual_id", manual_id),
        ),
        ManualCommand::Activate { manual_id } => request_and_print(
            client,
            "manual.activate",
            params_with_optional_id("manual_id", manual_id),
        ),
        ManualCommand::Versions { manual_id } => request_and_print(
            client,
            "manual.versions",
            params_with_optional_id("manual_id", manual_id),
        ),
    }
}

fn handle_optimization(
    command: OptimizationCommand,
    client: &mut AppServerClient,
) -> Result<(), CliError> {
    match command {
        OptimizationCommand::RecordRun { params } => {
            request_and_print(client, "optimization.record_run", read_json_file(&params)?)
        }
        OptimizationCommand::Analyze { params } => request_and_print(
            client,
            "optimization.analyze",
            read_optional_json(params.as_ref())?,
        ),
        OptimizationCommand::Compare { params } => request_and_print(
            client,
            "optimization.compare",
            read_optional_json(params.as_ref())?,
        ),
        OptimizationCommand::Report { params } => request_and_print(
            client,
            "optimization.report",
            read_optional_json(params.as_ref())?,
        ),
    }
}

fn handle_sandbox(command: SandboxCommand, client: &mut AppServerClient) -> Result<(), CliError> {
    match command {
        SandboxCommand::Create { params } => {
            request_and_print(client, "sandbox.create", read_json_file(&params)?)
        }
        SandboxCommand::Update {
            sandbox_id,
            changes,
            params,
        } => {
            let mut payload = read_optional_json_map(params.as_ref())?;
            insert_optional_string(&mut payload, "sandbox_id", Some(sandbox_id));
            if let Some(changes) = changes {
                payload.insert("changes".to_owned(), read_json_file(&changes)?);
            }
            request_and_print(client, "sandbox.update", Value::Object(payload))
        }
        SandboxCommand::Evaluate {
            sandbox_id,
            operation,
            target,
            params,
        } => {
            let mut payload = read_optional_json_map(params.as_ref())?;
            insert_optional_string(&mut payload, "sandbox_id", Some(sandbox_id));
            payload.insert(
                "operation".to_owned(),
                json!(normalize_sandbox_operation(&operation)),
            );
            payload.insert("target".to_owned(), json!(target));
            request_and_print(client, "sandbox.evaluate", Value::Object(payload))
        }
        SandboxCommand::Get { sandbox_id } => request_and_print(
            client,
            "sandbox.get",
            params_with_optional_id("sandbox_id", sandbox_id),
        ),
        SandboxCommand::List { params } => {
            request_and_print(client, "sandbox.list", read_optional_json(params.as_ref())?)
        }
    }
}

fn handle_skill(command: SkillCommand, client: &mut AppServerClient) -> Result<(), CliError> {
    match command {
        SkillCommand::Configure { params } => {
            request_and_print(client, "skill.configure", read_json_file(&params)?)
        }
        SkillCommand::Candidates { params } => {
            request_and_print(client, "skill.candidates", read_json_file(&params)?)
        }
        SkillCommand::RecordExecution {
            step_id,
            execution,
            params,
        } => {
            let mut payload = read_optional_json_map(params.as_ref())?;
            payload.insert("step_id".to_owned(), json!(step_id));
            if let Some(execution) = execution {
                merge_object_fields(
                    &mut payload,
                    unwrap_nested_field(read_json_file(&execution)?, "execution"),
                )?;
            }
            request_and_print(client, "skill.record_execution", Value::Object(payload))
        }
        SkillCommand::Verify { step_id } => request_and_print(
            client,
            "skill.verify",
            params_with_optional_id("step_id", step_id),
        ),
        SkillCommand::AgentCapabilities => {
            request_and_print(client, "skill.agent_capabilities", Value::Null)
        }
    }
}

fn request_and_print(
    client: &mut AppServerClient,
    method: &str,
    params: Value,
) -> Result<(), CliError> {
    let result = client.request(method, params)?;
    print_json(&result)
}

fn read_optional_json(path: Option<&PathBuf>) -> Result<Value, CliError> {
    match path {
        Some(path) => read_json_file(path),
        None => Ok(Value::Null),
    }
}

fn read_optional_json_map(path: Option<&PathBuf>) -> Result<Map<String, Value>, CliError> {
    match read_optional_json(path)? {
        Value::Null => Ok(Map::new()),
        Value::Object(map) => Ok(map),
        _ => Err(CliError::InvalidResponse(
            "expected a JSON object".to_owned(),
        )),
    }
}

fn read_optional_json_object(path: Option<&PathBuf>) -> Result<Value, CliError> {
    Ok(Value::Object(read_optional_json_map(path)?))
}

fn insert_optional_string(map: &mut Map<String, Value>, key: &str, value: Option<String>) {
    if let Some(value) = value {
        map.insert(key.to_owned(), json!(value));
    }
}

fn params_with_optional_id(key: &str, value: Option<String>) -> Value {
    let mut params = Map::new();
    insert_optional_string(&mut params, key, value);
    Value::Object(params)
}

fn unwrap_nested_field(value: Value, field: &str) -> Value {
    match value {
        Value::Object(mut object) if object.len() == 1 => {
            object.remove(field).unwrap_or(Value::Object(object))
        }
        _ => value,
    }
}

fn merge_object_fields(target: &mut Map<String, Value>, value: Value) -> Result<(), CliError> {
    match value {
        Value::Object(object) => {
            target.extend(object);
            Ok(())
        }
        _ => Err(CliError::InvalidResponse(
            "expected a JSON object".to_owned(),
        )),
    }
}

fn normalize_sandbox_operation(operation: &str) -> &str {
    // Why this exists: docs/wiki/architecture/manual-cli-command-surface.md keeps
    // CLI-friendly sandbox verbs aligned with the app-server policy vocabulary.
    match operation {
        "read" => "read_file",
        "write" => "write_file",
        "command" | "exec" => "execute",
        "env" => "read_env",
        other => other,
    }
}

fn print_events(
    client: &mut AppServerClient,
    run_id: String,
    mut cursor: usize,
    watch: bool,
    interval_ms: u64,
) -> Result<(), CliError> {
    loop {
        let result = client.request(
            "workflow.events",
            json!({
                "run_id": run_id,
                "cursor": cursor,
            }),
        )?;
        print_json(&result)?;

        if !watch || result.get("completed").and_then(Value::as_bool) == Some(true) {
            return Ok(());
        }

        cursor = result
            .get("next_cursor")
            .and_then(Value::as_u64)
            .ok_or(CliError::InvalidResponse("missing next_cursor".to_owned()))?
            as usize;
        thread::sleep(Duration::from_millis(interval_ms));
    }
}

#[derive(Parser)]
#[command(name = "manual")]
#[command(about = "Command line client for the Manual app-server JSON-RPC API")]
struct Cli {
    #[arg(long, value_name = "PATH")]
    server_bin: Option<PathBuf>,
    #[arg(long, env = "MANUAL_APP_SERVER_URL", value_name = "URL")]
    server_url: Option<String>,
    #[arg(long, env = "MANUAL_APP_SERVER_TOKEN", value_name = "TOKEN")]
    auth_token: Option<String>,
    #[arg(long, env = "MANUAL_APP_SERVER_DISCOVERY", value_name = "PATH")]
    discovery_file: Option<PathBuf>,

    #[command(subcommand)]
    command: CommandGroup,
}

// Why this exists: docs/wiki/architecture/manual-cli-command-surface.md documents
// that the CLI should expose dedicated app-server method groups instead of forcing
// most product features through raw JSON-RPC calls.
#[derive(Subcommand)]
enum CommandGroup {
    #[command(about = "Manage and run workflows through app-server")]
    Workflow {
        #[command(subcommand)]
        command: WorkflowCommand,
    },
    #[command(about = "Manage node templates, runs, and test cases through app-server")]
    Node {
        #[command(subcommand)]
        command: NodeCommand,
    },
    #[command(about = "Inspect local agent availability through app-server")]
    Agent {
        #[command(subcommand)]
        command: AgentCommand,
    },
    #[command(about = "Manage manuals through app-server")]
    Manual {
        #[command(subcommand)]
        command: ManualCommand,
    },
    #[command(about = "Inspect optimization runs and reports through app-server")]
    Optimization {
        #[command(subcommand)]
        command: OptimizationCommand,
    },
    #[command(about = "Manage sandbox policies through app-server")]
    Sandbox {
        #[command(subcommand)]
        command: SandboxCommand,
    },
    #[command(about = "Manage skill-routing records through app-server")]
    Skill {
        #[command(subcommand)]
        command: SkillCommand,
    },
    #[command(about = "Send a raw JSON-RPC method with optional params JSON")]
    Rpc {
        method: String,
        #[arg(value_name = "PARAMS_JSON")]
        params: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum WorkflowCommand {
    #[command(about = "Create a workflow from a JSON definition")]
    Create {
        #[arg(value_name = "WORKFLOW_JSON")]
        workflow: PathBuf,
    },
    #[command(about = "Fetch a workflow definition")]
    Get { workflow_id: String },
    #[command(about = "List stored workflows")]
    List,
    #[command(about = "Replace an existing workflow with a JSON definition")]
    Update {
        workflow_id: String,
        #[arg(value_name = "WORKFLOW_JSON")]
        workflow: PathBuf,
    },
    #[command(about = "Apply workflow patch operations from a JSON array")]
    Patch {
        workflow_id: String,
        #[arg(value_name = "OPERATIONS_JSON")]
        operations: PathBuf,
    },
    #[command(about = "Delete a stored workflow")]
    Delete { workflow_id: String },
    #[command(about = "Start a workflow and print the run id")]
    Start {
        workflow_id: String,
        #[arg(long, value_name = "NODE_ID", help = "Start execution from this node")]
        start_node: Option<String>,
        #[arg(long, help = "Resume from the first failed node of a previous run")]
        resume_from_failure: bool,
        #[arg(
            long,
            value_name = "PATH",
            help = "JSON file with node_id -> value overrides"
        )]
        inputs: Option<PathBuf>,
        #[arg(
            long,
            default_value = "auto",
            value_name = "MODE",
            help = "Execution mode: auto or step"
        )]
        mode: String,
        #[arg(long, value_name = "RUN_ID", help = "Previous run ID to resume from")]
        resume_run_id: Option<String>,
    },
    #[command(about = "Fetch or watch workflow run events")]
    Events {
        run_id: String,
        #[arg(long, default_value_t = 0)]
        cursor: usize,
        #[arg(long)]
        watch: bool,
        #[arg(long, default_value_t = 100)]
        interval_ms: u64,
    },
    #[command(about = "Start a workflow and watch events until completion")]
    Run {
        workflow_id: String,
        #[arg(long, default_value_t = 100)]
        interval_ms: u64,
        #[arg(long, value_name = "NODE_ID")]
        start_node: Option<String>,
        #[arg(long)]
        resume_from_failure: bool,
        #[arg(long, value_name = "PATH")]
        inputs: Option<PathBuf>,
        #[arg(long, default_value = "auto", value_name = "MODE")]
        mode: String,
        #[arg(long, value_name = "RUN_ID")]
        resume_run_id: Option<String>,
    },
    #[command(about = "Stop a running workflow run")]
    Stop { run_id: String },
    #[command(about = "Resume a paused or failed workflow run")]
    Resume {
        run_id: String,
        #[arg(long, value_name = "NODE_ID")]
        start_node: Option<String>,
        #[arg(long)]
        resume_from_failure: bool,
        #[arg(long, value_name = "PATH")]
        inputs: Option<PathBuf>,
        #[arg(long, default_value = "auto", value_name = "MODE")]
        mode: String,
    },
    #[command(about = "Compose a workflow candidate from a registered node template")]
    ComposeFromRegistry { node_id: String },
}

#[derive(Subcommand)]
enum NodeCommand {
    #[command(about = "Register a node template from a JSON node definition")]
    Create {
        #[arg(value_name = "NODE_JSON")]
        node: PathBuf,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        description: Option<String>,
    },
    #[command(about = "Fetch a registered node template")]
    Get { node_id: String },
    #[command(about = "List registered node templates")]
    List,
    #[command(about = "Update a registered node template")]
    Update {
        node_id: String,
        #[arg(long, value_name = "NODE_JSON")]
        node: Option<PathBuf>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        description: Option<String>,
    },
    #[command(about = "Delete a registered node template")]
    Delete { node_id: String },
    #[command(about = "Fetch schema information for a node kind")]
    Schema { kind: String },
    #[command(about = "Run a node definition with optional input overrides")]
    Run {
        #[arg(value_name = "NODE_JSON")]
        node: PathBuf,
        #[arg(long, value_name = "INPUTS_JSON")]
        inputs: Option<PathBuf>,
    },
    #[command(about = "Fetch a node run summary")]
    RunGet { run_id: String },
    #[command(about = "Fetch node run events")]
    RunEvents {
        run_id: String,
        #[arg(long, default_value_t = 0)]
        cursor: usize,
    },
    #[command(about = "Save a node test case from a previous run")]
    TestcaseSave {
        run_id: String,
        #[arg(long, value_name = "EXPECTED_OUTPUT_JSON")]
        expected_output: Option<PathBuf>,
        #[arg(long, value_name = "CRITERIA_JSON")]
        criteria: Option<PathBuf>,
    },
    #[command(about = "Verify saved node test cases")]
    TestcaseVerify { node_id: String },
}

#[derive(Subcommand)]
enum AgentCommand {
    #[command(about = "List local agent availability")]
    List {
        #[arg(long, value_name = "PARAMS_JSON")]
        params: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum ManualCommand {
    #[command(about = "Create a manual from a JSON payload")]
    Create {
        #[arg(value_name = "MANUAL_JSON")]
        params: PathBuf,
    },
    #[command(about = "Fetch a manual by id")]
    Get { manual_id: Option<String> },
    #[command(about = "List manuals with optional filters")]
    List {
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        query: Option<String>,
        #[arg(long)]
        tag: Option<String>,
        #[arg(long, value_name = "PARAMS_JSON")]
        params: Option<PathBuf>,
    },
    #[command(about = "Update a manual with a JSON changes object")]
    Update {
        manual_id: String,
        #[arg(long, value_name = "CHANGES_JSON")]
        changes: Option<PathBuf>,
        #[arg(long)]
        execution_affecting: bool,
        #[arg(long, value_name = "PARAMS_JSON")]
        params: Option<PathBuf>,
    },
    #[command(about = "Clone a manual")]
    Clone { manual_id: Option<String> },
    #[command(about = "Archive a manual")]
    Archive { manual_id: Option<String> },
    #[command(about = "Delete a manual")]
    Delete { manual_id: Option<String> },
    #[command(about = "Activate a manual")]
    Activate { manual_id: Option<String> },
    #[command(about = "Fetch manual version history")]
    Versions { manual_id: Option<String> },
}

#[derive(Subcommand)]
enum OptimizationCommand {
    #[command(about = "Record an optimization run from JSON input")]
    RecordRun {
        #[arg(value_name = "RUN_JSON")]
        params: PathBuf,
    },
    #[command(about = "Analyze optimization history")]
    Analyze {
        #[arg(long, value_name = "PARAMS_JSON")]
        params: Option<PathBuf>,
    },
    #[command(about = "Compare optimization runs")]
    Compare {
        #[arg(long, value_name = "PARAMS_JSON")]
        params: Option<PathBuf>,
    },
    #[command(about = "Render an optimization report")]
    Report {
        #[arg(long, value_name = "PARAMS_JSON")]
        params: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum SandboxCommand {
    #[command(about = "Create a sandbox from JSON input")]
    Create {
        #[arg(value_name = "SANDBOX_JSON")]
        params: PathBuf,
    },
    #[command(about = "Update a sandbox with a JSON changes object")]
    Update {
        sandbox_id: String,
        #[arg(long, value_name = "CHANGES_JSON")]
        changes: Option<PathBuf>,
        #[arg(long, value_name = "PARAMS_JSON")]
        params: Option<PathBuf>,
    },
    #[command(about = "Evaluate whether an operation is allowed by a sandbox")]
    Evaluate {
        sandbox_id: String,
        operation: String,
        target: String,
        #[arg(long, value_name = "PARAMS_JSON")]
        params: Option<PathBuf>,
    },
    #[command(about = "Fetch a sandbox")]
    Get { sandbox_id: Option<String> },
    #[command(about = "List sandboxes")]
    List {
        #[arg(long, value_name = "PARAMS_JSON")]
        params: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum SkillCommand {
    #[command(about = "Configure a skill-routing step from JSON input")]
    Configure {
        #[arg(value_name = "STEP_JSON")]
        params: PathBuf,
    },
    #[command(about = "List candidate skills for a task from JSON input")]
    Candidates {
        #[arg(value_name = "CANDIDATES_JSON")]
        params: PathBuf,
    },
    #[command(about = "Record a skill execution for a step")]
    RecordExecution {
        step_id: String,
        #[arg(long, value_name = "EXECUTION_JSON")]
        execution: Option<PathBuf>,
        #[arg(long, value_name = "PARAMS_JSON")]
        params: Option<PathBuf>,
    },
    #[command(about = "Verify whether a skill step used its assigned skill")]
    Verify { step_id: Option<String> },
    #[command(about = "List agent capability hints for skill routing")]
    AgentCapabilities,
}

enum AppServerClient {
    Stdio(StdioAppServerClient),
    Http(HttpAppServerClient),
}

impl AppServerClient {
    fn stdio(server_bin: PathBuf) -> Result<Self, CliError> {
        StdioAppServerClient::launch(server_bin).map(Self::Stdio)
    }

    fn http(server_url: String, auth_token: String) -> Self {
        Self::Http(HttpAppServerClient {
            server_url,
            auth_token,
        })
    }

    fn daemon(server_bin: PathBuf, discovery_file: PathBuf) -> Result<Self, CliError> {
        if let Some(discovery) = read_discovery_file(&discovery_file) {
            let client = HttpAppServerClient {
                server_url: discovery.server_url,
                auth_token: discovery.auth_token,
            };
            if client.health().is_ok() {
                return Ok(Self::Http(client));
            }
        }

        let auth_token = generate_auth_token();
        launch_daemon(&server_bin, &discovery_file, &auth_token)?;

        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        while std::time::Instant::now() < deadline {
            if let Some(discovery) = read_discovery_file(&discovery_file) {
                let client = HttpAppServerClient {
                    server_url: discovery.server_url,
                    auth_token: discovery.auth_token,
                };
                if client.health().is_ok() {
                    return Ok(Self::Http(client));
                }
            }

            thread::sleep(Duration::from_millis(50));
        }

        Err(CliError::InvalidResponse(
            "app-server daemon did not publish discovery information".to_owned(),
        ))
    }

    fn request(&mut self, method: &str, params: Value) -> Result<Value, CliError> {
        match self {
            Self::Stdio(client) => client.request(method, params),
            Self::Http(client) => client.request(method, params),
        }
    }
}

struct StdioAppServerClient {
    _child: Child,
    stdin: ChildStdin,
    stdout: io::BufReader<ChildStdout>,
    next_id: u64,
}

impl StdioAppServerClient {
    fn launch(server_bin: PathBuf) -> Result<Self, CliError> {
        let mut child = Command::new(&server_bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| CliError::LaunchServer {
                path: server_bin,
                source: error,
            })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| CliError::InvalidResponse("app-server stdin unavailable".to_owned()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| CliError::InvalidResponse("app-server stdout unavailable".to_owned()))?;

        Ok(Self {
            _child: child,
            stdin,
            stdout: io::BufReader::new(stdout),
            next_id: 1,
        })
    }

    fn request(&mut self, method: &str, params: Value) -> Result<Value, CliError> {
        let id = self.next_id;
        self.next_id += 1;

        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        serde_json::to_writer(&mut self.stdin, &request)?;
        self.stdin.write_all(b"\n")?;
        self.stdin.flush()?;

        let mut line = String::new();
        let bytes_read = self.stdout.read_line(&mut line)?;
        if bytes_read == 0 {
            return Err(CliError::InvalidResponse(
                "app-server returned an empty response".to_owned(),
            ));
        }

        let response: Value = serde_json::from_str(&line)?;
        if let Some(error) = response.get("error") {
            let code = error.get("code").and_then(Value::as_i64).unwrap_or(0);
            let message = error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("JSON-RPC error")
                .to_owned();
            return Err(CliError::Rpc { code, message });
        }

        response
            .get("result")
            .cloned()
            .ok_or_else(|| CliError::InvalidResponse("missing result".to_owned()))
    }
}

struct HttpAppServerClient {
    server_url: String,
    auth_token: String,
}

impl HttpAppServerClient {
    fn health(&self) -> Result<(), CliError> {
        let (host, port) = parse_http_url(&self.server_url)?;
        let mut stream = TcpStream::connect((host.as_str(), port))?;
        let request = format!("GET /health HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
        stream.write_all(request.as_bytes())?;
        stream.shutdown(std::net::Shutdown::Write)?;
        let mut response = String::new();
        stream.read_to_string(&mut response)?;
        if response.starts_with("HTTP/1.1 200 OK") {
            Ok(())
        } else {
            Err(CliError::InvalidResponse(
                "app-server health check failed".to_owned(),
            ))
        }
    }

    fn request(&mut self, method: &str, params: Value) -> Result<Value, CliError> {
        let (host, port) = parse_http_url(&self.server_url)?;
        let mut stream = TcpStream::connect((host.as_str(), port))?;
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });
        let body = request.to_string();
        let http_request = format!(
            "POST /rpc HTTP/1.1\r\nHost: {host}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            self.auth_token,
            body.len()
        );
        stream.write_all(http_request.as_bytes())?;
        stream.shutdown(std::net::Shutdown::Write)?;

        let mut response = String::new();
        stream.read_to_string(&mut response)?;
        if !response.starts_with("HTTP/1.1 200 OK") {
            return Err(CliError::InvalidResponse(
                response
                    .lines()
                    .next()
                    .unwrap_or("app-server HTTP error")
                    .to_owned(),
            ));
        }

        let (_, body) = response
            .split_once("\r\n\r\n")
            .ok_or_else(|| CliError::InvalidResponse("missing HTTP response body".to_owned()))?;
        let response: Value = serde_json::from_str(body)?;
        if let Some(error) = response.get("error") {
            let code = error.get("code").and_then(Value::as_i64).unwrap_or(0);
            let message = error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("JSON-RPC error")
                .to_owned();
            return Err(CliError::Rpc { code, message });
        }

        response
            .get("result")
            .cloned()
            .ok_or_else(|| CliError::InvalidResponse("missing result".to_owned()))
    }
}

fn resolve_server_bin(explicit: Option<&Path>) -> Result<PathBuf, CliError> {
    let env_path = env::var_os("MANUAL_APP_SERVER_BIN").map(PathBuf::from);
    let current_exe = env::current_exe().ok();
    let cwd = env::current_dir().ok();
    resolve_server_bin_from(
        explicit,
        env_path.as_deref(),
        current_exe.as_deref(),
        cwd.as_deref(),
    )
}

fn resolve_server_bin_from(
    explicit: Option<&Path>,
    env_path: Option<&Path>,
    current_exe: Option<&Path>,
    cwd: Option<&Path>,
) -> Result<PathBuf, CliError> {
    if let Some(path) = explicit {
        return Ok(path.to_owned());
    }

    if let Some(path) = env_path {
        return Ok(path.to_owned());
    }

    if let Some(current_exe) = current_exe {
        if let Some(bin_dir) = current_exe.parent() {
            let sibling = bin_dir.join(server_binary_name());
            if sibling.is_file() {
                return Ok(sibling);
            }
        }
    }

    let Some(cwd) = cwd else {
        return Err(CliError::ServerBinaryNotFound);
    };

    let candidates = [
        cwd.join("manual-rs/target/debug/manual-app-server"),
        cwd.join("../manual-rs/target/debug/manual-app-server"),
        cwd.join("../../manual-rs/target/debug/manual-app-server"),
        cwd.join("manual-rs/target/debug/app-server"),
        cwd.join("../manual-rs/target/debug/app-server"),
        cwd.join("../../manual-rs/target/debug/app-server"),
    ];

    candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        .ok_or(CliError::ServerBinaryNotFound)
}

fn server_binary_name() -> &'static str {
    if cfg!(windows) {
        "manual-app-server.exe"
    } else {
        "manual-app-server"
    }
}

struct Discovery {
    server_url: String,
    auth_token: String,
}

fn read_discovery_file(path: &Path) -> Option<Discovery> {
    let contents = fs::read_to_string(path).ok()?;
    let value: Value = serde_json::from_str(&contents).ok()?;
    Some(Discovery {
        server_url: value.get("url")?.as_str()?.to_owned(),
        auth_token: value.get("auth_token")?.as_str()?.to_owned(),
    })
}

fn launch_daemon(
    server_bin: &Path,
    discovery_file: &Path,
    auth_token: &str,
) -> Result<(), CliError> {
    let mut command = Command::new(server_bin);
    command
        .arg("--listen")
        .arg("127.0.0.1:0")
        .arg("--auth-token")
        .arg(auth_token)
        .arg("--discovery-file")
        .arg(discovery_file)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe extern "C" {
            fn setsid() -> i32;
        }

        unsafe {
            command.pre_exec(|| {
                if setsid() == -1 {
                    return Err(io::Error::last_os_error());
                }

                Ok(())
            });
        }
    }

    command
        .spawn()
        .map(|_| ())
        .map_err(|source| CliError::LaunchServer {
            path: server_bin.to_owned(),
            source,
        })
}

fn default_discovery_file() -> PathBuf {
    // Why this exists: docs/wiki/architecture/manual-app-architecture.md documents
    // that local Manual clients share one hidden home-directory state root by default.
    default_discovery_file_from(
        env::var("MANUAL_APP_SERVER_DISCOVERY")
            .ok()
            .map(PathBuf::from),
        env::var("HOME").ok().as_deref().map(Path::new),
        env::current_dir().ok().as_deref(),
    )
}

fn default_discovery_file_from(
    override_path: Option<PathBuf>,
    home_dir: Option<&Path>,
    current_dir: Option<&Path>,
) -> PathBuf {
    if let Some(path) = override_path {
        return path;
    }

    if let Some(home) = home_dir {
        return home.join(".manual").join("app-server.json");
    }

    if let Some(current_dir) = current_dir {
        return current_dir.join(".manual").join("app-server.json");
    }

    env::temp_dir().join("manual-app-server.json")
}

fn generate_auth_token() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("{:x}-{:x}", std::process::id(), nanos)
}

fn parse_http_url(url: &str) -> Result<(String, u16), CliError> {
    let address = url.strip_prefix("http://").ok_or_else(|| {
        CliError::InvalidResponse("only http:// app-server URLs are supported".to_owned())
    })?;
    let (host, port) = address.split_once(':').ok_or_else(|| {
        CliError::InvalidResponse("app-server URL must include a port".to_owned())
    })?;
    let port = port
        .trim_end_matches('/')
        .parse::<u16>()
        .map_err(|_| CliError::InvalidResponse("app-server URL port is invalid".to_owned()))?;

    if host != "127.0.0.1" && host != "localhost" {
        return Err(CliError::InvalidResponse(
            "app-server URL must point at localhost".to_owned(),
        ));
    }

    Ok((host.to_owned(), port))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn default_discovery_file_uses_hidden_manual_directory() {
        let path = default_discovery_file_from(
            None,
            Some(Path::new("/Users/example")),
            Some(Path::new("/workspace")),
        );

        assert_eq!(
            path,
            PathBuf::from("/Users/example/.manual/app-server.json")
        );
    }

    #[test]
    fn resolve_server_bin_prefers_sibling_manual_app_server() {
        let temp = std::env::temp_dir().join(format!(
            "manual-cli-sibling-server-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&temp).unwrap();

        let cli = temp.join("manual");
        let server = temp.join("manual-app-server");
        fs::write(&cli, "").unwrap();
        fs::write(&server, "").unwrap();

        let resolved =
            resolve_server_bin_from(None, None, Some(&cli), Some(Path::new("/workspace"))).unwrap();

        assert_eq!(resolved, server);
        fs::remove_dir_all(temp).unwrap();
    }
}

fn read_json_file(path: &Path) -> Result<Value, CliError> {
    if path == Path::new("-") {
        let stdin = io::stdin();
        return serde_json::from_reader(stdin.lock()).map_err(Into::into);
    }

    let contents = fs::read_to_string(path)?;
    serde_json::from_str(&contents).map_err(Into::into)
}

fn print_json(value: &Value) -> Result<(), CliError> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    serde_json::to_writer_pretty(&mut stdout, value)?;
    writeln!(stdout)?;
    Ok(())
}

#[derive(Debug)]
enum CliError {
    Io(io::Error),
    Json(serde_json::Error),
    LaunchServer { path: PathBuf, source: io::Error },
    ServerBinaryNotFound,
    InvalidResponse(String),
    Rpc { code: i64, message: String },
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::Io(error) => write!(formatter, "{error}"),
            CliError::Json(error) => write!(formatter, "{error}"),
            CliError::LaunchServer { path, source } => {
                write!(
                    formatter,
                    "failed to launch app-server at {}: {source}",
                    path.display()
                )
            }
            CliError::ServerBinaryNotFound => write!(
                formatter,
                "manual-app-server binary not found; pass --server-bin or set MANUAL_APP_SERVER_BIN"
            ),
            CliError::InvalidResponse(message) => write!(formatter, "{message}"),
            CliError::Rpc { code, message } => {
                write!(formatter, "app-server error {code}: {message}")
            }
        }
    }
}

impl std::error::Error for CliError {}

impl From<io::Error> for CliError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for CliError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}
