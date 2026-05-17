use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};

pub mod claude;
pub mod codex;
pub mod hermes;
pub mod pi;

pub fn crate_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Agent {
    id: String,
    name: String,
    instructions: String,
}

impl Agent {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        instructions: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            instructions: instructions.into(),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn instructions(&self) -> &str {
        &self.instructions
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandRequest {
    prompt: String,
    cwd: Option<PathBuf>,
    model: Option<String>,
    extra_args: Vec<String>,
    sandbox_policy: Option<Value>,
}

impl CommandRequest {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            cwd: None,
            model: None,
            extra_args: Vec::new(),
            sandbox_policy: None,
        }
    }

    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    pub fn cwd(&self) -> Option<&str> {
        self.cwd.as_ref().and_then(|path| path.to_str())
    }

    pub fn cwd_path(&self) -> Option<&Path> {
        self.cwd.as_deref()
    }

    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    pub fn extra_args(&self) -> &[String] {
        &self.extra_args
    }

    pub fn sandbox_policy(&self) -> Option<&Value> {
        self.sandbox_policy.as_ref()
    }

    pub fn with_cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_extra_arg(mut self, arg: impl Into<String>) -> Self {
        self.extra_args.push(arg.into());
        self
    }

    pub fn with_sandbox_policy(mut self, sandbox_policy: Value) -> Self {
        self.sandbox_policy = Some(sandbox_policy);
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandOutput {
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub trait AgentCommand {
    fn agent(&self) -> &Agent;

    fn command(&self, request: &CommandRequest) -> Command;

    fn run(&self, request: &CommandRequest) -> std::io::Result<CommandOutput> {
        let output = self.command(request).output()?;

        Ok(CommandOutput {
            status_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

pub(crate) fn apply_cwd(command: &mut Command, request: &CommandRequest) {
    if let Some(cwd) = request.cwd_path() {
        command.current_dir(cwd);
    }
}

pub(crate) fn command_with_optional_sandbox(
    request: &CommandRequest,
    program: &str,
    args: Vec<String>,
) -> Command {
    let mut command = if let Some(sandbox) = request.sandbox_policy() {
        manual_sandbox::sandboxed_command(sandbox, program, &args)
            .expect("sandbox command should be buildable")
    } else {
        let mut command = Command::new(program);
        command.args(args);
        command
    };
    apply_cwd(&mut command, request);
    command
}

pub fn list_agent_availability(params: &Value) -> Value {
    // Why this exists: docs/wiki/architecture/manual-app-architecture.md keeps
    // local agent discovery in the agent crate while app-server only exposes it.
    let candidates = string_array_param(params, "candidates").unwrap_or_else(|| {
        ["claude", "codex", "pi", "hermes"]
            .map(str::to_owned)
            .to_vec()
    });
    let path_dirs = string_array_param(params, "path_dirs");
    let agents = candidates
        .into_iter()
        .map(|name| {
            let executable = command_path(&name, path_dirs.as_deref());
            json!({
                "name": name,
                "available": executable.is_some(),
                "path": executable,
            })
        })
        .collect::<Vec<_>>();
    json!({ "agents": agents })
}

fn string_array_param(params: &Value, key: &str) -> Option<Vec<String>> {
    params[key].as_array().map(|values| {
        values
            .iter()
            .filter_map(|value| value.as_str().map(str::to_owned))
            .collect()
    })
}

fn command_path(command: &str, path_dirs: Option<&[String]>) -> Option<String> {
    let dirs = path_dirs.map(|dirs| dirs.to_vec()).unwrap_or_else(|| {
        std::env::var_os("PATH")
            .map(split_paths)
            .unwrap_or_default()
    });
    dirs.into_iter()
        .map(|dir| Path::new(&dir).join(command))
        .find(|path| is_executable_file(path))
        .map(|path| path.display().to_string())
}

fn split_paths(paths: std::ffi::OsString) -> Vec<String> {
    std::env::split_paths(&paths)
        .map(|path| path.display().to_string())
        .collect()
}

#[cfg(unix)]
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable_file(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|metadata| metadata.is_file())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_preserves_public_identity_and_instructions() {
        let agent = Agent::new("agent.primary", "Primary Agent", "Answer carefully.");

        assert_eq!(agent.id(), "agent.primary");
        assert_eq!(agent.name(), "Primary Agent");
        assert_eq!(agent.instructions(), "Answer carefully.");
    }

    #[test]
    fn command_request_defaults_to_prompt_only() {
        let request = CommandRequest::new("hello");

        assert_eq!(request.prompt(), "hello");
        assert_eq!(request.cwd(), None);
        assert_eq!(request.model(), None);
        assert!(request.extra_args().is_empty());
        assert!(request.sandbox_policy().is_none());
    }

    #[test]
    fn command_request_accepts_common_options() {
        let request = CommandRequest::new("hello")
            .with_cwd("/tmp/manual")
            .with_model("fast-model")
            .with_extra_arg("--flag")
            .with_sandbox_policy(serde_json::json!({ "allow_network": [] }));

        assert_eq!(request.cwd(), Some("/tmp/manual"));
        assert_eq!(request.model(), Some("fast-model"));
        assert_eq!(request.extra_args(), ["--flag"]);
        assert!(request.sandbox_policy().is_some());
    }

    #[test]
    fn codex_cli_builds_non_interactive_command() {
        let cli = codex::Codex::new(Agent::new("codex", "Codex", "Use Codex CLI."));
        let command = cli.command(&CommandRequest::new("hello").with_model("gpt-5"));

        assert_eq!(command.get_program(), "codex");
        assert_eq!(
            command_args(&command),
            ["exec", "--model", "gpt-5", "hello"]
        );
    }

    #[test]
    fn claude_cli_builds_non_interactive_command() {
        let cli = claude::Claude::new(Agent::new("claude", "Claude", "Use Claude Code."));
        let command = cli.command(&CommandRequest::new("hello").with_model("sonnet"));

        assert_eq!(command.get_program(), "claude");
        assert_eq!(
            command_args(&command),
            ["--print", "--model", "sonnet", "hello"]
        );
    }

    #[test]
    fn hermes_cli_builds_non_interactive_command() {
        let cli = hermes::Hermes::new(Agent::new("hermes", "Hermes", "Use Hermes Agent."));
        let command = cli.command(&CommandRequest::new("hello").with_model("anthropic/claude"));

        assert_eq!(command.get_program(), "hermes");
        assert_eq!(
            command_args(&command),
            [
                "chat",
                "--quiet",
                "--model",
                "anthropic/claude",
                "--query",
                "hello"
            ]
        );
    }

    #[test]
    fn pi_cli_builds_non_interactive_command() {
        let cli = pi::Pi::new(Agent::new("pi", "Pi", "Use Pi CLI."));
        let command = cli.command(&CommandRequest::new("hello").with_model("openai/gpt-4o"));

        assert_eq!(command.get_program(), "pi");
        assert_eq!(
            command_args(&command),
            ["--print", "--model", "openai/gpt-4o", "hello"]
        );
    }

    #[test]
    fn agent_availability_uses_supplied_path_dirs() {
        let agents = list_agent_availability(&serde_json::json!({
            "candidates": ["definitely-missing-manual-agent"],
            "path_dirs": ["/tmp"]
        }));

        assert_eq!(
            agents["agents"][0]["name"],
            "definitely-missing-manual-agent"
        );
        assert_eq!(agents["agents"][0]["available"], false);
    }

    fn command_args(command: &std::process::Command) -> Vec<String> {
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }
}
