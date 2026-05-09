use std::env;
use std::fmt;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::thread;
use std::time::Duration;

use clap::{Parser, Subcommand};
use serde_json::{Value, json};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), CliError> {
    let cli = Cli::parse();
    let mut client = AppServerClient::launch(resolve_server_bin(cli.server_bin.as_deref())?)?;

    match cli.command {
        CommandGroup::Workflow { command } => handle_workflow(command, &mut client),
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
            let result = client.request("workflow.create", json!({ "workflow": workflow }))?;
            print_json(&result)
        }
        WorkflowCommand::Get { workflow_id } => {
            let result = client.request("workflow.get", json!({ "workflow_id": workflow_id }))?;
            print_json(&result)
        }
        WorkflowCommand::List => {
            let result = client.request("workflow.list", Value::Null)?;
            print_json(&result)
        }
        WorkflowCommand::Update {
            workflow_id,
            workflow,
        } => {
            let workflow = read_json_file(&workflow)?;
            let result = client.request(
                "workflow.update",
                json!({
                    "workflow_id": workflow_id,
                    "workflow": workflow,
                }),
            )?;
            print_json(&result)
        }
        WorkflowCommand::Delete { workflow_id } => {
            let result =
                client.request("workflow.delete", json!({ "workflow_id": workflow_id }))?;
            print_json(&result)
        }
        WorkflowCommand::Start { workflow_id } => {
            let result = client.request("workflow.start", json!({ "workflow_id": workflow_id }))?;
            print_json(&result)
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
        } => {
            let started =
                client.request("workflow.start", json!({ "workflow_id": workflow_id }))?;
            print_json(&started)?;
            let run_id = started
                .get("run_id")
                .and_then(Value::as_str)
                .ok_or(CliError::InvalidResponse("missing run_id".to_owned()))?
                .to_owned();
            print_events(client, run_id, 0, true, interval_ms)
        }
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
#[command(name = "manual-cli")]
#[command(about = "Command line client for the Manual app-server JSON-RPC API")]
struct Cli {
    #[arg(long, env = "MANUAL_APP_SERVER_BIN", value_name = "PATH")]
    server_bin: Option<PathBuf>,

    #[command(subcommand)]
    command: CommandGroup,
}

#[derive(Subcommand)]
enum CommandGroup {
    #[command(about = "Manage and run workflows through app-server")]
    Workflow {
        #[command(subcommand)]
        command: WorkflowCommand,
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
    #[command(about = "Delete a stored workflow")]
    Delete { workflow_id: String },
    #[command(about = "Start a workflow and print the run id")]
    Start { workflow_id: String },
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
    },
}

struct AppServerClient {
    _child: Child,
    stdin: ChildStdin,
    stdout: io::BufReader<ChildStdout>,
    next_id: u64,
}

impl AppServerClient {
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

fn resolve_server_bin(explicit: Option<&Path>) -> Result<PathBuf, CliError> {
    if let Some(path) = explicit {
        return Ok(path.to_owned());
    }

    if let Ok(path) = env::var("MANUAL_APP_SERVER_BIN") {
        return Ok(PathBuf::from(path));
    }

    let cwd = env::current_dir()?;
    let candidates = [
        cwd.join("manual-rs/target/debug/app-server"),
        cwd.join("../manual-rs/target/debug/app-server"),
        cwd.join("../../manual-rs/target/debug/app-server"),
    ];

    candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        .ok_or(CliError::ServerBinaryNotFound)
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
                "app-server binary not found; pass --server-bin or set MANUAL_APP_SERVER_BIN"
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
