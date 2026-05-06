use std::path::PathBuf;

use crate::AgentCommand;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudeRequest {
    prompt: String,
    model: Option<String>,
    permission_mode: Option<String>,
    workdir: Option<PathBuf>,
    include_partial_messages: bool,
    include_hook_events: bool,
    extra_args: Vec<String>,
}

impl ClaudeRequest {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            model: None,
            permission_mode: None,
            workdir: None,
            include_partial_messages: false,
            include_hook_events: false,
            extra_args: Vec::new(),
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_permission_mode(mut self, permission_mode: impl Into<String>) -> Self {
        self.permission_mode = Some(permission_mode.into());
        self
    }

    pub fn with_workdir(mut self, workdir: impl Into<PathBuf>) -> Self {
        self.workdir = Some(workdir.into());
        self
    }

    pub fn include_partial_messages(mut self, include_partial_messages: bool) -> Self {
        self.include_partial_messages = include_partial_messages;
        self
    }

    pub fn include_hook_events(mut self, include_hook_events: bool) -> Self {
        self.include_hook_events = include_hook_events;
        self
    }

    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        self.extra_args.push(arg.into());
        self
    }
}

pub fn help_command() -> AgentCommand {
    AgentCommand::new("claude").arg("-h")
}

pub fn jsonl_command(request: ClaudeRequest) -> AgentCommand {
    let mut command = AgentCommand::new("claude").with_args([
        "--print",
        "--output-format",
        "stream-json",
        "--verbose",
    ]);

    if let Some(model) = request.model {
        command = command.arg("--model").arg(model);
    }

    if let Some(permission_mode) = request.permission_mode {
        command = command.arg("--permission-mode").arg(permission_mode);
    }

    if let Some(workdir) = request.workdir {
        command = command.with_current_dir(workdir);
    }

    if request.include_partial_messages {
        command = command.arg("--include-partial-messages");
    }

    if request.include_hook_events {
        command = command.arg("--include-hook-events");
    }

    command.with_args(request.extra_args).arg(request.prompt)
}
