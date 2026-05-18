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
use serde_json::{Value, json};

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
        WorkflowCommand::Start {
            workflow_id,
            start_node,
            resume_from_failure,
            inputs,
            mode,
            resume_run_id,
        } => {
            let input_overrides = if let Some(path) = inputs {
                read_json_file(&path)?
            } else {
                serde_json::Value::Object(serde_json::Map::new())
            };
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
            let result = client.request("workflow.start", params)?;
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
            start_node,
            resume_from_failure,
            inputs,
            mode,
            resume_run_id,
        } => {
            let input_overrides = if let Some(path) = inputs {
                read_json_file(&path)?
            } else {
                serde_json::Value::Object(serde_json::Map::new())
            };
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
            let input_overrides = if let Some(path) = inputs {
                read_json_file(&path)?
            } else {
                serde_json::Value::Object(serde_json::Map::new())
            };
            let mut params = json!({
                "run_id": run_id,
                "resume_from_failure": resume_from_failure,
                "input_overrides": input_overrides,
                "mode": mode,
            });
            if let Some(node) = start_node {
                params["start_node_id"] = json!(node);
            }
            let result = client.request("workflow.resume", params)?;
            print_json(&result)
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
    Start {
        workflow_id: String,
        #[arg(long, value_name = "NODE_ID", help = "Start execution from this node")]
        start_node: Option<String>,
        #[arg(long, help = "Resume from the first failed node of a previous run")]
        resume_from_failure: bool,
        #[arg(long, value_name = "PATH", help = "JSON file with node_id -> value overrides")]
        inputs: Option<PathBuf>,
        #[arg(long, default_value = "auto", value_name = "MODE", help = "Execution mode: auto or step")]
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
        env::var("MANUAL_APP_SERVER_DISCOVERY").ok().map(PathBuf::from),
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

        let resolved = resolve_server_bin_from(None, None, Some(&cli), Some(Path::new("/workspace")))
            .unwrap();

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
