use std::process::Command;

use crate::{Agent, AgentCommand, CommandRequest, command_with_optional_sandbox};

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
        let mut args = vec!["--print".to_owned()];

        if let Some(model) = request.model() {
            args.push("--model".to_owned());
            args.push(model.to_owned());
        }

        args.extend(request.extra_args().iter().cloned());
        args.push(request.prompt().to_owned());
        command_with_optional_sandbox(request, BINARY, args)
    }
}
