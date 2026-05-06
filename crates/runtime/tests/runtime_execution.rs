use std::path::Path;
use std::path::PathBuf;

use agent::AgentCommand;
use runtime::{
    RuntimeError, RuntimeOutput, RuntimeRequest, RuntimeRunner, RuntimeSandbox, RuntimeTarget,
    RuntimeTargetKind,
};
use sandbox::{
    BackendKind, BackendReport, FilesystemAccess, FilesystemEntry, FilesystemMode, NetworkMode,
    Platform, SandboxError, SandboxPolicy, SandboxPreset, SandboxRunner,
};
use script::ScriptError;
use script::ScriptRunner;

fn echo_stdin_command() -> AgentCommand {
    if cfg!(target_os = "windows") {
        AgentCommand::new("cmd").with_args(["/C", "more"])
    } else {
        AgentCommand::new("sh").with_args(["-c", "cat"])
    }
}

fn failing_agent_command() -> AgentCommand {
    if cfg!(target_os = "windows") {
        AgentCommand::new("cmd")
            .arg("/C")
            .arg("echo agent failed 1>&2 && exit /B 7")
    } else {
        AgentCommand::new("sh")
            .arg("-c")
            .arg("printf 'agent failed' >&2; exit 7")
    }
}

fn policy_entry(policy: &SandboxPolicy, path: impl AsRef<Path>) -> Option<FilesystemAccess> {
    let path = path.as_ref();

    policy
        .filesystem
        .entries
        .iter()
        .find(|entry| entry.path == path)
        .map(|entry| entry.access)
}

fn backend_report(exit_code: Option<i32>) -> RuntimeOutput {
    RuntimeOutput {
        target: RuntimeTargetKind::Agent,
        sandbox: BackendReport {
            platform: Platform::Linux,
            kind: BackendKind::Direct,
            available: true,
            detail: "test backend".to_string(),
        },
        stdout: String::new(),
        stderr: String::new(),
        exit_code,
    }
}

#[test]
fn read_only_runtime_sandbox_defines_restricted_read_policy() {
    let policy = RuntimeSandbox::read_only("/workspace").to_policy();

    assert_eq!(policy.preset, SandboxPreset::ReadOnly);
    assert_eq!(policy.filesystem.mode, FilesystemMode::Restricted);
    assert_eq!(policy.network.mode, NetworkMode::Restricted);
    assert_eq!(
        policy_entry(&policy, "/workspace"),
        Some(FilesystemAccess::Read)
    );

    for protected in [".git", ".manual", ".codex", ".agents"] {
        assert_eq!(
            policy_entry(&policy, Path::new("/workspace").join(protected)),
            Some(FilesystemAccess::None),
            "protected metadata entry should be denied for {protected}"
        );
    }
}

#[test]
fn workspace_write_runtime_sandbox_defines_restricted_write_policy() {
    let policy = RuntimeSandbox::workspace_write("/workspace").to_policy();

    assert_eq!(policy.preset, SandboxPreset::WorkspaceWrite);
    assert_eq!(policy.filesystem.mode, FilesystemMode::Restricted);
    assert_eq!(policy.network.mode, NetworkMode::Restricted);
    assert_eq!(
        policy_entry(&policy, "/workspace"),
        Some(FilesystemAccess::Write)
    );
    assert_eq!(
        policy_entry(&policy, "/workspace/.git"),
        Some(FilesystemAccess::None)
    );
}

#[test]
fn danger_full_access_runtime_sandbox_defines_unrestricted_policy() {
    let policy = RuntimeSandbox::danger_full_access().to_policy();

    assert_eq!(policy.preset, SandboxPreset::DangerFullAccess);
    assert_eq!(policy.filesystem.mode, FilesystemMode::Unrestricted);
    assert!(policy.filesystem.entries.is_empty());
    assert_eq!(policy.network.mode, NetworkMode::Enabled);
}

#[test]
fn custom_policy_runtime_sandbox_preserves_policy_and_clones_outputs() {
    let mut custom = SandboxPolicy::read_only("/workspace");
    custom.network.mode = NetworkMode::Enabled;
    custom
        .filesystem
        .entries
        .push(FilesystemEntry::new("/extra-input", FilesystemAccess::Read));
    let runtime_sandbox = RuntimeSandbox::policy(custom.clone());

    let mut resolved = runtime_sandbox.to_policy();
    assert_eq!(resolved, custom);

    resolved
        .filesystem
        .entries
        .push(FilesystemEntry::new("/mutated", FilesystemAccess::Write));

    assert_eq!(runtime_sandbox.to_policy(), custom);
}

#[test]
fn runs_rust_script_with_input_through_the_configured_sandbox() {
    let request = RuntimeRequest::script(
        r#"{"name":"manual"}"#,
        r#"
fn main(input_json: &str) -> String {
    format!("script saw {input_json}")
}
"#,
        RuntimeSandbox::danger_full_access(),
    );

    let output =
        RuntimeRunner::for_sandbox_runner(SandboxRunner::for_platform(Platform::current()))
            .run(&request)
            .expect("script runtime should execute");

    assert_eq!(output.target, RuntimeTargetKind::Script);
    assert_eq!(output.sandbox.kind, BackendKind::Direct);
    assert!(output.success());
    assert_eq!(output.stdout, r#"script saw {"name":"manual"}"#);
    assert_eq!(output.stderr, "");
}

#[test]
fn runs_agent_command_with_input_through_the_configured_sandbox() {
    let request = RuntimeRequest::agent(
        r#"{"task":"echo"}"#,
        echo_stdin_command(),
        RuntimeSandbox::danger_full_access(),
    );

    let output =
        RuntimeRunner::for_sandbox_runner(SandboxRunner::for_platform(Platform::current()))
            .run(&request)
            .expect("agent runtime should execute");

    assert_eq!(output.target, RuntimeTargetKind::Agent);
    assert_eq!(output.sandbox.kind, BackendKind::Direct);
    assert!(output.success());
    assert_eq!(output.stdout, r#"{"task":"echo"}"#);
    assert_eq!(output.stderr, "");
}

#[test]
fn read_only_agent_plan_uses_enforced_backend_and_stdin() {
    let request = RuntimeRequest::agent(
        r#"{"task":"inspect"}"#,
        echo_stdin_command(),
        RuntimeSandbox::read_only("/workspace"),
    );

    let plan = RuntimeRunner::for_sandbox_runner(SandboxRunner::for_backend_availability(
        Platform::Linux,
        true,
    ))
    .compile(&request)
    .expect("read-only runtime request should compile");

    assert_eq!(plan.target, RuntimeTargetKind::Agent);
    assert_eq!(plan.sandbox.kind, BackendKind::LinuxBubblewrap);
    assert!(plan.sandbox.available);
    assert_eq!(plan.policy.preset, SandboxPreset::ReadOnly);
    assert_eq!(
        policy_entry(&plan.policy, "/workspace"),
        Some(FilesystemAccess::Read)
    );
    assert_eq!(plan.command.stdin.as_deref(), Some(r#"{"task":"inspect"}"#));
}

#[test]
fn workspace_write_agent_plan_preserves_current_dir_and_write_policy() {
    let request = RuntimeRequest::agent(
        r#"{"task":"patch"}"#,
        echo_stdin_command().with_current_dir("/workspace/project"),
        RuntimeSandbox::workspace_write("/workspace"),
    );

    let plan = RuntimeRunner::for_sandbox_runner(SandboxRunner::for_backend_availability(
        Platform::Macos,
        true,
    ))
    .compile(&request)
    .expect("workspace-write runtime request should compile");

    assert_eq!(plan.target, RuntimeTargetKind::Agent);
    assert_eq!(plan.sandbox.kind, BackendKind::MacosSeatbelt);
    assert_eq!(
        plan.command.current_dir,
        Some(PathBuf::from("/workspace/project"))
    );
    assert_eq!(plan.policy.preset, SandboxPreset::WorkspaceWrite);
    assert_eq!(
        policy_entry(&plan.policy, "/workspace"),
        Some(FilesystemAccess::Write)
    );
    assert_eq!(
        policy_entry(&plan.policy, "/workspace/.agents"),
        Some(FilesystemAccess::None)
    );
}

#[test]
fn danger_full_access_plan_uses_direct_backend_and_unrestricted_policy() {
    let request = RuntimeRequest::agent(
        r#"{"task":"unsafe"}"#,
        echo_stdin_command(),
        RuntimeSandbox::danger_full_access(),
    );

    let plan = RuntimeRunner::for_sandbox_runner(SandboxRunner::for_backend_availability(
        Platform::Linux,
        false,
    ))
    .compile(&request)
    .expect("danger-full-access should compile without a platform sandbox backend");

    assert_eq!(plan.sandbox.kind, BackendKind::Direct);
    assert!(plan.sandbox.available);
    assert_eq!(plan.policy.preset, SandboxPreset::DangerFullAccess);
    assert_eq!(plan.policy.filesystem.mode, FilesystemMode::Unrestricted);
    assert_eq!(plan.policy.network.mode, NetworkMode::Enabled);
    assert_eq!(plan.command.stdin.as_deref(), Some(r#"{"task":"unsafe"}"#));
}

#[test]
fn custom_policy_agent_plan_uses_exact_policy() {
    let mut policy = SandboxPolicy::workspace_write("/workspace");
    policy.network.mode = NetworkMode::Enabled;
    policy.filesystem.entries.push(FilesystemEntry::new(
        "/readonly-cache",
        FilesystemAccess::Read,
    ));
    let request = RuntimeRequest::agent(
        r#"{"task":"custom"}"#,
        echo_stdin_command(),
        RuntimeSandbox::policy(policy.clone()),
    );

    let plan = RuntimeRunner::for_sandbox_runner(SandboxRunner::for_backend_availability(
        Platform::Windows,
        true,
    ))
    .compile(&request)
    .expect("custom runtime policy should compile");

    assert_eq!(plan.sandbox.kind, BackendKind::WindowsRestrictedToken);
    assert_eq!(plan.policy, policy);
}

#[test]
fn restricted_script_plan_adds_compiled_workspace_read_entry() {
    let request = RuntimeRequest::script(
        r#"{"task":"script"}"#,
        r#"
fn main(input_json: &str) -> String {
    input_json.to_string()
}
"#,
        RuntimeSandbox::read_only("/workspace"),
    );

    let plan = RuntimeRunner::for_sandbox_runner(SandboxRunner::for_backend_availability(
        Platform::Linux,
        true,
    ))
    .compile(&request)
    .expect("restricted script runtime request should compile");
    let script_binary = PathBuf::from(&plan.command.program);
    let script_workspace = script_binary
        .parent()
        .expect("compiled script binary should live inside a workspace");

    assert_eq!(plan.target, RuntimeTargetKind::Script);
    assert_eq!(plan.sandbox.kind, BackendKind::LinuxBubblewrap);
    assert_eq!(
        policy_entry(&plan.policy, script_workspace),
        Some(FilesystemAccess::Read)
    );
    assert_eq!(plan.command.stdin.as_deref(), Some(r#"{"task":"script"}"#));
}

#[test]
fn unrestricted_script_plan_does_not_add_compiled_workspace_entry() {
    let request = RuntimeRequest::script(
        "{}",
        r#"
fn main(input_json: &str) -> String {
    input_json.to_string()
}
"#,
        RuntimeSandbox::danger_full_access(),
    );

    let plan = RuntimeRunner::for_sandbox_runner(SandboxRunner::for_platform(Platform::current()))
        .compile(&request)
        .expect("unrestricted script runtime request should compile");

    assert_eq!(plan.sandbox.kind, BackendKind::Direct);
    assert!(plan.policy.filesystem.entries.is_empty());
}

#[test]
fn enforced_runtime_does_not_fallback_when_backend_is_missing() {
    let request = RuntimeRequest::agent(
        "{}",
        echo_stdin_command(),
        RuntimeSandbox::workspace_write("/workspace"),
    );

    let error = RuntimeRunner::for_sandbox_runner(SandboxRunner::for_backend_availability(
        Platform::Linux,
        false,
    ))
    .run(&request)
    .expect_err("enforced runtime should not run when the backend is missing");

    assert!(matches!(
        error,
        RuntimeError::Sandbox(SandboxError::BackendUnavailable {
            backend: BackendKind::LinuxBubblewrap,
            platform: Platform::Linux,
            ..
        })
    ));
}

#[test]
fn script_compile_error_is_reported_before_sandbox_backend_errors() {
    let request = RuntimeRequest::script(
        "{}",
        r#"
fn main(input_json: &str) -> String {
    missing_symbol(input_json)
}
"#,
        RuntimeSandbox::read_only("/workspace"),
    );

    let error = RuntimeRunner::for_sandbox_runner(SandboxRunner::for_backend_availability(
        Platform::Linux,
        false,
    ))
    .run(&request)
    .expect_err("invalid script source should fail at compile time");

    assert!(matches!(
        error,
        RuntimeError::Script(script::ScriptError::CompileFailed { .. })
    ));
}

#[test]
fn agent_runtime_captures_nonzero_status_and_stderr() {
    let request = RuntimeRequest::agent(
        "{}",
        failing_agent_command(),
        RuntimeSandbox::danger_full_access(),
    );

    let output =
        RuntimeRunner::for_sandbox_runner(SandboxRunner::for_platform(Platform::current()))
            .run(&request)
            .expect("agent process should launch");

    assert_eq!(output.target, RuntimeTargetKind::Agent);
    assert!(!output.success());
    assert_eq!(output.exit_code, Some(7));
    assert_eq!(output.stdout, "");
    assert_eq!(output.stderr.trim_end_matches(['\r', '\n']), "agent failed");
}

#[test]
fn runtime_error_display_includes_wrapped_error_message() {
    assert_eq!(
        RuntimeError::Sandbox(SandboxError::Io("spawn failed".to_string())).to_string(),
        "runtime sandbox failed: sandbox process failed: spawn failed"
    );
    assert_eq!(
        RuntimeError::Script(ScriptError::Io("rustc missing".to_string())).to_string(),
        "runtime script failed: script process failed: rustc missing"
    );
}

#[test]
fn detected_runtime_uses_current_platform_for_direct_plan() {
    let request = RuntimeRequest::agent(
        "{}",
        echo_stdin_command(),
        RuntimeSandbox::danger_full_access(),
    );

    let plan = RuntimeRunner::detect()
        .compile(&request)
        .expect("detected runtime should compile a direct plan");

    assert_eq!(plan.sandbox.kind, BackendKind::Direct);
    assert_eq!(plan.sandbox.platform, Platform::current());
}

#[test]
fn custom_script_runner_is_used_for_script_compilation() {
    let request = RuntimeRequest::script(
        "{}",
        r#"
fn main(input_json: &str) -> String {
    input_json.to_string()
}
"#,
        RuntimeSandbox::danger_full_access(),
    );

    let error = RuntimeRunner::for_sandbox_runner(SandboxRunner::for_platform(Platform::current()))
        .with_script_runner(
            ScriptRunner::default().with_rustc("/definitely/missing/manual-runtime-rustc"),
        )
        .run(&request)
        .expect_err("custom missing rustc should be used");

    assert!(matches!(error, RuntimeError::Script(ScriptError::Io(_))));
}

#[test]
fn runtime_output_success_only_accepts_zero_exit_code() {
    assert!(backend_report(Some(0)).success());
    assert!(!backend_report(Some(1)).success());
    assert!(!backend_report(None).success());
}

#[test]
fn runtime_request_exposes_target_and_input_for_planning() {
    let request = RuntimeRequest::new(
        r#"{"task":"inspect"}"#,
        RuntimeSandbox::danger_full_access(),
        RuntimeTarget::agent(AgentCommand::new("codex").arg("inspect")),
    );

    assert_eq!(request.input_json, r#"{"task":"inspect"}"#);
    assert_eq!(request.target.kind(), RuntimeTargetKind::Agent);
    assert_eq!(
        request.sandbox.to_policy(),
        RuntimeSandbox::danger_full_access().to_policy()
    );
}
