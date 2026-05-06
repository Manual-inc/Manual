use std::io;
use std::io::Cursor;
use std::path::Path;

use agent::AgentCommand;
use agent::JsonlLines;
use agent::claude;
use agent::claude::ClaudeRequest;
use agent::codex;
use agent::codex::CodexRequest;

fn args(command: &AgentCommand) -> Vec<&str> {
    command.args().iter().map(String::as_str).collect()
}

#[test]
fn codex_help_command_uses_cli_help() {
    let command = codex::help_command();

    assert_eq!(command.program(), "codex");
    assert_eq!(args(&command), ["-h"]);
}

#[test]
fn codex_jsonl_command_uses_exec_json_stream() {
    let command = codex::jsonl_command(CodexRequest::new("summarize this repository"));

    assert_eq!(command.program(), "codex");
    assert_eq!(
        args(&command),
        ["exec", "--json", "summarize this repository"]
    );
}

#[test]
fn codex_jsonl_command_accepts_execution_controls() {
    let command = codex::jsonl_command(
        CodexRequest::new("inspect the workspace")
            .with_model("gpt-5.5")
            .with_sandbox("workspace-write")
            .with_workdir("/tmp/manual")
            .with_arg("--ephemeral"),
    );

    assert_eq!(
        args(&command),
        [
            "exec",
            "--json",
            "--model",
            "gpt-5.5",
            "--sandbox",
            "workspace-write",
            "--cd",
            "/tmp/manual",
            "--ephemeral",
            "inspect the workspace"
        ]
    );
}

#[test]
fn claude_help_command_uses_cli_help() {
    let command = claude::help_command();

    assert_eq!(command.program(), "claude");
    assert_eq!(args(&command), ["-h"]);
}

#[test]
fn claude_jsonl_command_uses_print_stream_json() {
    let command = claude::jsonl_command(ClaudeRequest::new("summarize this repository"));

    assert_eq!(command.program(), "claude");
    assert_eq!(
        args(&command),
        [
            "--print",
            "--output-format",
            "stream-json",
            "--verbose",
            "summarize this repository"
        ]
    );
}

#[test]
fn claude_jsonl_command_accepts_execution_controls() {
    let command = claude::jsonl_command(
        ClaudeRequest::new("inspect the workspace")
            .with_model("sonnet")
            .with_permission_mode("dontAsk")
            .with_workdir("/tmp/manual")
            .include_partial_messages(true)
            .include_hook_events(true),
    );

    assert_eq!(
        args(&command),
        [
            "--print",
            "--output-format",
            "stream-json",
            "--verbose",
            "--model",
            "sonnet",
            "--permission-mode",
            "dontAsk",
            "--include-partial-messages",
            "--include-hook-events",
            "inspect the workspace"
        ]
    );
    assert_eq!(command.current_dir(), Some(Path::new("/tmp/manual")));
}

#[test]
fn jsonl_lines_yield_newline_trimmed_events() -> io::Result<()> {
    let reader = Cursor::new(
        br#"{"type":"started"}
{"type":"finished"}
"#,
    );
    let events = JsonlLines::new(reader).collect::<io::Result<Vec<_>>>()?;

    assert_eq!(events, [r#"{"type":"started"}"#, r#"{"type":"finished"}"#]);
    Ok(())
}

#[test]
fn spawned_jsonl_child_reads_next_line_from_stdout() -> io::Result<()> {
    let command = AgentCommand::new("sh").with_args([
        "-c",
        "printf '%s\\n' '{\"type\":\"started\"}' '{\"type\":\"finished\"}'",
    ]);
    let mut child = command.spawn_jsonl()?;

    assert_eq!(
        child.next_line()?,
        Some(r#"{"type":"started"}"#.to_string())
    );
    assert_eq!(
        child.next_line()?,
        Some(r#"{"type":"finished"}"#.to_string())
    );
    assert_eq!(child.next_line()?, None);

    let status = child.wait()?;
    assert!(status.success());

    Ok(())
}

#[test]
fn spawned_jsonl_child_iterates_over_stdout_lines() -> io::Result<()> {
    let command = AgentCommand::new("sh").with_args([
        "-c",
        "printf '%s\\n' '{\"type\":\"delta\"}' '{\"type\":\"result\"}'",
    ]);
    let mut child = command.spawn_jsonl()?;

    assert_eq!(
        child.next().transpose()?,
        Some(r#"{"type":"delta"}"#.to_string())
    );
    assert_eq!(
        child.next().transpose()?,
        Some(r#"{"type":"result"}"#.to_string())
    );
    assert_eq!(child.next().transpose()?, None);

    let status = child.wait()?;
    assert!(status.success());

    Ok(())
}

#[test]
fn spawned_jsonl_child_wait_reports_process_exit_status() -> io::Result<()> {
    let command = AgentCommand::new("sh").with_args(["-c", "exit 7"]);
    let child = command.spawn_jsonl()?;
    let status = child.wait()?;

    assert!(!status.success());
    assert_eq!(status.code(), Some(7));

    Ok(())
}
