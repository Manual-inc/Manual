#![cfg(feature = "manual-cli-tests")]

use manual_agent::{
    Agent, AgentCommand, CommandRequest, claude::Claude, codex::Codex, hermes::Hermes, pi::Pi,
};

#[test]
#[ignore = "runs real CLI agents; invoke explicitly with --ignored and --features manual-cli-tests"]
fn runs_same_request_against_all_cli_agents() {
    let request = CommandRequest::new("Reply with exactly: manual-agent-smoke-test");
    let agents: Vec<(&str, Box<dyn AgentCommand>)> = vec![
        (
            "claude",
            Box::new(Claude::new(Agent::new(
                "claude",
                "Claude",
                "Use Claude Code.",
            ))),
        ),
        (
            "codex",
            Box::new(Codex::new(Agent::new("codex", "Codex", "Use Codex CLI."))),
        ),
        (
            "hermes",
            Box::new(Hermes::new(Agent::new(
                "hermes",
                "Hermes",
                "Use Hermes Agent.",
            ))),
        ),
        (
            "pi",
            Box::new(Pi::new(Agent::new("pi", "Pi", "Use Pi CLI."))),
        ),
    ];

    for (name, agent) in agents {
        let command = agent.command(&request);
        println!("===== {name} command =====");
        println!("{command:?}");

        let output = agent.run(&request).expect("CLI process should start");
        println!("===== {name} status =====");
        println!("{:?}", output.status_code);
        println!("===== {name} stdout =====");
        println!("{}", output.stdout);
        println!("===== {name} stderr =====");
        println!("{}", output.stderr);

        assert_eq!(
            output.status_code,
            Some(0),
            "{name} CLI should exit successfully"
        );
    }
}
