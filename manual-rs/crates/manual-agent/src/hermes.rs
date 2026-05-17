use std::process::Command;

use crate::{Agent, AgentCommand, CommandRequest, command_with_optional_sandbox};

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
        let mut args = vec!["chat".to_owned(), "--quiet".to_owned()];

        if let Some(model) = request.model() {
            args.push("--model".to_owned());
            args.push(model.to_owned());
        }

        args.extend(request.extra_args().iter().cloned());
        args.push("--query".to_owned());
        args.push(request.prompt().to_owned());
        command_with_optional_sandbox(request, BINARY, args)
    }
}
