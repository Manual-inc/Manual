use std::path::{Path, PathBuf};
use std::process::Command;

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
}

impl CommandRequest {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            cwd: None,
            model: None,
            extra_args: Vec::new(),
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
    }

    #[test]
    fn command_request_accepts_common_options() {
        let request = CommandRequest::new("hello")
            .with_cwd("/tmp/manual")
            .with_model("fast-model")
            .with_extra_arg("--flag");

        assert_eq!(request.cwd(), Some("/tmp/manual"));
        assert_eq!(request.model(), Some("fast-model"));
        assert_eq!(request.extra_args(), ["--flag"]);
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

    fn command_args(command: &std::process::Command) -> Vec<String> {
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }
}
