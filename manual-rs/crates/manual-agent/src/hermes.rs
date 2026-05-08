use std::process::Command;

use crate::{Agent, AgentCommand, CommandRequest, apply_cwd};

pub const BINARY: &str = "hermes";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Hermes {
    agent: Agent,
}

impl Hermes {
    pub fn new(agent: Agent) -> Self {
        Self { agent }
    }
}

impl AgentCommand for Hermes {
    fn agent(&self) -> &Agent {
        &self.agent
    }

    fn command(&self, request: &CommandRequest) -> Command {
        let mut command = Command::new(BINARY);
        command.args(["chat", "--quiet"]);

        if let Some(model) = request.model() {
            command.args(["--model", model]);
        }

        command.args(request.extra_args());
        command.args(["--query", request.prompt()]);
        apply_cwd(&mut command, request);
        command
    }
}
