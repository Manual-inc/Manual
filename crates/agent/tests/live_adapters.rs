//! Manual live tests for the real Codex and Claude CLIs.
//!
//! Run with:
//! `cargo test -p agent --test live_adapters -- --ignored --nocapture`

use std::io;

use agent::AgentCommand;
use agent::claude;
use agent::claude::ClaudeRequest;
use agent::codex;
use agent::codex::CodexRequest;

const CODEX_MARKER: &str = "MANUAL_LIVE_CODEX_OK";
const CLAUDE_MARKER: &str = "MANUAL_LIVE_CLAUDE_OK";
const MAX_JSONL_EVENTS: usize = 256;

fn live_codex_command() -> AgentCommand {
    codex::jsonl_command(
        CodexRequest::new(format!(
            "Reply with exactly {CODEX_MARKER}. Do not run tools."
        ))
        .with_sandbox("read-only")
        .with_arg("--ephemeral")
        .with_arg("--skip-git-repo-check"),
    )
}

fn live_claude_command() -> AgentCommand {
    claude::jsonl_command(
        ClaudeRequest::new(format!(
            "Reply with exactly {CLAUDE_MARKER}. Do not use tools."
        ))
        .with_permission_mode("dontAsk")
        .with_arg("--bare")
        .with_arg("--no-session-persistence")
        .with_arg("--max-budget-usd")
        .with_arg("0.10"),
    )
}

fn collect_jsonl_stdout(command: AgentCommand) -> io::Result<Vec<String>> {
    let mut child = command.spawn_jsonl()?;
    let mut lines = Vec::new();
    let mut event_count = 0;

    while let Some(line) = child.next_line()? {
        event_count += 1;

        if event_count > MAX_JSONL_EVENTS {
            return Err(io::Error::other(format!(
                "agent command produced more than {MAX_JSONL_EVENTS} JSONL events"
            )));
        }

        if !line.trim().is_empty() {
            lines.push(line);
        }
    }

    let status = child.wait()?;

    if !status.success() {
        return Err(io::Error::other(format!(
            "agent command exited with {status}; captured stdout:\n{}",
            lines.join("\n")
        )));
    }

    Ok(lines)
}

fn assert_jsonl_contains_marker(lines: &[String], marker: &str) {
    assert!(!lines.is_empty(), "expected at least one JSONL event");

    assert!(
        lines
            .iter()
            .all(|line| line.trim_start().starts_with('{') && line.trim_end().ends_with('}')),
        "expected every stdout line to be a JSON object, got:\n{}",
        lines.join("\n")
    );

    assert!(
        lines.iter().any(|line| line.contains(marker)),
        "expected JSONL stream to contain marker {marker}, got:\n{}",
        lines.join("\n")
    );
}

#[test]
#[ignore = "runs the real Codex CLI, requires auth/network, and may spend tokens"]
fn live_codex_jsonl_stream_runs_non_interactively() -> io::Result<()> {
    let lines = collect_jsonl_stdout(live_codex_command())?;
    assert_jsonl_contains_marker(&lines, CODEX_MARKER);

    Ok(())
}

#[test]
#[ignore = "runs the real Claude CLI, requires auth/network, and may spend tokens"]
fn live_claude_jsonl_stream_runs_non_interactively() -> io::Result<()> {
    let lines = collect_jsonl_stdout(live_claude_command())?;
    assert_jsonl_contains_marker(&lines, CLAUDE_MARKER);

    Ok(())
}

#[test]
#[ignore = "manual live-adapter support check"]
fn live_codex_command_shape_matches_manual_invocation() {
    let command = live_codex_command();

    assert_eq!(command.program(), "codex");
    assert_eq!(
        command.args(),
        [
            "exec",
            "--json",
            "--sandbox",
            "read-only",
            "--ephemeral",
            "--skip-git-repo-check",
            &format!("Reply with exactly {CODEX_MARKER}. Do not run tools.")
        ]
    );
}

#[test]
#[ignore = "manual live-adapter support check"]
fn live_claude_command_shape_matches_manual_invocation() {
    let command = live_claude_command();

    assert_eq!(command.program(), "claude");
    assert_eq!(
        command.args(),
        [
            "--print",
            "--output-format",
            "stream-json",
            "--verbose",
            "--permission-mode",
            "dontAsk",
            "--bare",
            "--no-session-persistence",
            "--max-budget-usd",
            "0.10",
            &format!("Reply with exactly {CLAUDE_MARKER}. Do not use tools.")
        ]
    );
}

#[test]
#[ignore = "manual live-adapter support check"]
fn live_jsonl_helper_reports_failed_process_status() {
    let result = collect_jsonl_stdout(
        AgentCommand::new("sh")
            .with_args(["-c", "printf '%s\\n' '{\"type\":\"before-exit\"}'; exit 7"]),
    );

    let error = result.expect_err("failed process status should be reported");
    assert!(
        error.to_string().contains("exit status: 7"),
        "unexpected error: {error}"
    );
}
