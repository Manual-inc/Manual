use std::process::Command;

use crate::{Agent, AgentCommand, CommandRequest, apply_cwd};

pub const BINARY: &str = "claude";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Claude {
    agent: Agent,
}

impl Claude {
    pub fn new(agent: Agent) -> Self {
        Self { agent }
    }
}

impl AgentCommand for Claude {
    fn agent(&self) -> &Agent {
        &self.agent
    }

    fn command(&self, request: &CommandRequest) -> Command {
        let mut command = Command::new(BINARY);
        command.arg("--print");

        if let Some(model) = request.model() {
            command.args(["--model", model]);
        }

        command.args(request.extra_args());
        command.arg(request.prompt());
        apply_cwd(&mut command, request);
        command
    }
}
