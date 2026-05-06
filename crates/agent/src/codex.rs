use std::path::PathBuf;

use crate::AgentCommand;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexRequest {
    prompt: String,
    model: Option<String>,
    sandbox: Option<String>,
    workdir: Option<PathBuf>,
    extra_args: Vec<String>,
}

impl CodexRequest {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            model: None,
            sandbox: None,
            workdir: None,
            extra_args: Vec::new(),
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_sandbox(mut self, sandbox: impl Into<String>) -> Self {
        self.sandbox = Some(sandbox.into());
        self
    }

    pub fn with_workdir(mut self, workdir: impl Into<PathBuf>) -> Self {
        self.workdir = Some(workdir.into());
        self
    }

    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        self.extra_args.push(arg.into());
        self
    }
}

pub fn help_command() -> AgentCommand {
    AgentCommand::new("codex").arg("-h")
}

pub fn exec_help_command() -> AgentCommand {
    AgentCommand::new("codex").with_args(["exec", "-h"])
}

pub fn jsonl_command(request: CodexRequest) -> AgentCommand {
    let mut command = AgentCommand::new("codex").with_args(["exec", "--json"]);

    if let Some(model) = request.model {
        command = command.arg("--model").arg(model);
    }

    if let Some(sandbox) = request.sandbox {
        command = command.arg("--sandbox").arg(sandbox);
    }

    if let Some(workdir) = request.workdir {
        command = command.arg("--cd").arg(workdir.display().to_string());
    }

    command.with_args(request.extra_args).arg(request.prompt)
}
