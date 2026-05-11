use std::process::Command;

use crate::{Agent, AgentCommand, CommandRequest, apply_cwd};

pub const BINARY: &str = "pi";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pi {
    agent: Agent,
}

impl Pi {
    pub fn new(agent: Agent) -> Self {
        Self { agent }
    }
}

impl AgentCommand for Pi {
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
