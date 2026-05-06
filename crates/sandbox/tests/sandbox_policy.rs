use std::path::Path;

use sandbox::{
    BackendKind, CommandSpec, FilesystemAccess, FilesystemMode, NetworkMode, Platform,
    SandboxError, SandboxPolicy, SandboxResult, SandboxRunner,
};

#[test]
fn detects_host_platform() {
    let platform = Platform::current();

    if cfg!(target_os = "macos") {
        assert_eq!(platform, Platform::Macos);
    } else if cfg!(target_os = "linux") {
        assert_eq!(platform, Platform::Linux);
    } else if cfg!(target_os = "windows") {
        assert_eq!(platform, Platform::Windows);
    } else {
        assert_eq!(platform, Platform::Unsupported(std::env::consts::OS));
    }
}

#[test]
fn read_only_preset_is_restricted_and_protects_metadata() {
    let policy = SandboxPolicy::read_only("/workspace");

    assert_eq!(policy.filesystem.mode, FilesystemMode::Restricted);
    assert_eq!(policy.network.mode, NetworkMode::Restricted);
    assert!(policy.filesystem.entries.iter().any(
        |entry| entry.path == Path::new("/workspace") && entry.access == FilesystemAccess::Read
    ));

    for protected in [".git", ".manual", ".codex", ".agents"] {
        assert!(
            policy
                .filesystem
                .entries
                .iter()
                .any(|entry| entry.path.ends_with(protected)
                    && entry.access == FilesystemAccess::None),
            "missing protected metadata entry for {protected}"
        );
    }
}

#[test]
fn workspace_write_preset_allows_workspace_write_but_keeps_metadata_denied() {
    let policy = SandboxPolicy::workspace_write("/workspace");

    assert!(
        policy
            .filesystem
            .entries
            .iter()
            .any(|entry| entry.path == Path::new("/workspace")
                && entry.access == FilesystemAccess::Write)
    );

    assert!(
        policy
            .filesystem
            .entries
            .iter()
            .any(|entry| entry.path.ends_with(".git") && entry.access == FilesystemAccess::None)
    );
}

#[test]
fn runner_detects_backend_for_current_platform() {
    let runner = SandboxRunner::detect();

    match Platform::current() {
        Platform::Macos => assert_eq!(runner.backend().kind, BackendKind::MacosSeatbelt),
        Platform::Linux => assert_eq!(runner.backend().kind, BackendKind::LinuxBubblewrap),
        Platform::Windows => assert_eq!(runner.backend().kind, BackendKind::WindowsRestrictedToken),
        Platform::Unsupported(_) => assert_eq!(runner.backend().kind, BackendKind::Unsupported),
    }
}

#[test]
fn compiles_danger_full_access_to_direct_process() {
    let command = CommandSpec::new("echo")
        .arg("hello")
        .current_dir("/tmp")
        .env("A", "B");
    let plan = SandboxRunner::for_platform(Platform::Linux)
        .compile(&command, &SandboxPolicy::danger_full_access())
        .expect("danger-full-access should always compile");

    assert_eq!(plan.backend.kind, BackendKind::Direct);
    assert_eq!(plan.program, "echo");
    assert_eq!(plan.args, ["hello"]);
    assert_eq!(plan.current_dir, Some(Path::new("/tmp").to_path_buf()));
    assert_eq!(plan.env, [("A".to_string(), "B".to_string())].into());
}

#[test]
fn compiles_macos_read_only_to_fixed_sandbox_exec_plan() {
    let command = CommandSpec::new("pwd");
    let policy = SandboxPolicy::read_only("/workspace");
    let plan = SandboxRunner::for_platform(Platform::Macos)
        .compile(&command, &policy)
        .expect("macOS read-only policy should compile to a Seatbelt plan");

    assert_eq!(plan.backend.kind, BackendKind::MacosSeatbelt);
    assert_eq!(plan.program, "/usr/bin/sandbox-exec");
    assert_eq!(plan.args[0], "-p");
    assert!(plan.args[1].contains("(deny default)"));
    assert!(plan.args[1].contains("(allow file-read*"));
    assert!(plan.args[1].contains("/workspace/.git"));
    assert!(plan.args.iter().any(|arg| arg == "--"));
    assert!(plan.args.iter().any(|arg| arg == "pwd"));
}

#[test]
fn compiles_macos_network_enabled_policy_to_network_allow_rule() {
    let command = CommandSpec::new("pwd");

    let restricted_plan = SandboxRunner::for_platform(Platform::Macos)
        .compile(&command, &SandboxPolicy::read_only("/workspace"))
        .expect("restricted macOS policy should compile");
    assert!(!restricted_plan.args[1].contains("(allow network*)"));

    let mut network_enabled = SandboxPolicy::read_only("/workspace");
    network_enabled.network.mode = NetworkMode::Enabled;

    let enabled_plan = SandboxRunner::for_platform(Platform::Macos)
        .compile(&command, &network_enabled)
        .expect("network-enabled macOS policy should compile");
    assert!(enabled_plan.args[1].contains("(allow network*)"));
}

#[test]
fn compiles_linux_workspace_write_to_bubblewrap_plan() {
    let command = CommandSpec::new("pwd");
    let policy = SandboxPolicy::workspace_write("/workspace");
    let plan = SandboxRunner::for_platform(Platform::Linux)
        .compile(&command, &policy)
        .expect("Linux workspace-write policy should compile to a bubblewrap plan");

    assert_eq!(plan.backend.kind, BackendKind::LinuxBubblewrap);
    assert_eq!(plan.program, "bwrap");
    assert!(plan.args.iter().any(|arg| arg == "--unshare-user"));
    assert!(plan.args.windows(2).any(|pair| pair == ["--uid", "0"]));
    assert!(plan.args.windows(2).any(|pair| pair == ["--gid", "0"]));
    assert!(plan.args.iter().any(|arg| arg == "--unshare-pid"));
    assert!(plan.args.iter().any(|arg| arg == "--unshare-net"));
    assert!(
        plan.args
            .windows(2)
            .any(|pair| pair == ["--bind", "/workspace"])
    );
    assert!(plan.args.iter().any(|arg| arg == "pwd"));
}

#[test]
fn compiles_windows_workspace_write_to_restricted_token_plan() {
    let command = CommandSpec::new("cmd").arg("/C").arg("echo ok");
    let policy = SandboxPolicy::workspace_write("C:\\workspace");
    let plan = SandboxRunner::for_platform(Platform::Windows)
        .compile(&command, &policy)
        .expect("Windows workspace-write policy should compile to a restricted token plan");

    assert_eq!(plan.backend.kind, BackendKind::WindowsRestrictedToken);
    assert_eq!(plan.program, "manual-windows-sandbox-runner");
    assert!(plan.args.iter().any(|arg| arg == "--restricted-token"));
    assert!(plan.args.iter().any(|arg| arg == "--allow-write"));
    assert!(plan.args.iter().any(|arg| arg == "C:\\workspace"));
    assert!(plan.args.iter().any(|arg| arg == "cmd"));
}

#[test]
fn compiles_windows_network_policy_into_runner_flags() {
    let command = CommandSpec::new("cmd").arg("/C").arg("echo ok");

    let restricted_plan = SandboxRunner::for_platform(Platform::Windows)
        .compile(&command, &SandboxPolicy::read_only("C:\\workspace"))
        .expect("restricted Windows policy should compile");
    assert!(
        restricted_plan
            .args
            .iter()
            .any(|arg| arg == "--deny-network")
    );

    let mut network_enabled = SandboxPolicy::read_only("C:\\workspace");
    network_enabled.network.mode = NetworkMode::Enabled;

    let enabled_plan = SandboxRunner::for_platform(Platform::Windows)
        .compile(&command, &network_enabled)
        .expect("network-enabled Windows policy should compile");
    assert!(!enabled_plan.args.iter().any(|arg| arg == "--deny-network"));
}

#[test]
fn enforced_policy_does_not_silently_fall_back_when_backend_is_missing() {
    let command = CommandSpec::new("echo").arg("hello");
    let result = SandboxRunner::for_backend_availability(Platform::Linux, false)
        .run(&command, &SandboxPolicy::read_only("/workspace"));

    assert!(matches!(
        result,
        Err(SandboxError::BackendUnavailable {
            backend: BackendKind::LinuxBubblewrap,
            ..
        })
    ));
}

#[test]
fn enforced_policy_marked_available_attempts_the_backend_process() {
    let command = CommandSpec::new("echo")
        .arg("hello")
        .current_dir("/definitely/missing/manual-sandbox-current-dir");
    let result = SandboxRunner::for_backend_availability(Platform::Macos, true)
        .run(&command, &SandboxPolicy::read_only("/workspace"));

    assert!(
        matches!(result, Err(SandboxError::Io(_))),
        "expected the backend process to be attempted, got {result:?}"
    );
}

#[test]
fn sandbox_result_success_only_accepts_zero_exit_status() {
    assert!(
        SandboxResult {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: Some(0),
        }
        .success()
    );

    assert!(
        !SandboxResult {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: Some(1),
        }
        .success()
    );

    assert!(
        !SandboxResult {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
        }
        .success()
    );
}

#[test]
fn sandbox_errors_have_actionable_display_messages() {
    let unavailable = SandboxError::BackendUnavailable {
        backend: BackendKind::LinuxBubblewrap,
        platform: Platform::Linux,
        detail: "missing bwrap".to_string(),
    };
    assert_eq!(
        unavailable.to_string(),
        "sandbox backend LinuxBubblewrap is unavailable on Linux: missing bwrap"
    );

    assert_eq!(
        SandboxError::UnsupportedPlatform("haiku".to_string()).to_string(),
        "unsupported sandbox platform: haiku"
    );
    assert_eq!(
        SandboxError::Io("spawn failed".to_string()).to_string(),
        "sandbox process failed: spawn failed"
    );
}

#[test]
fn direct_process_run_captures_output_and_status() {
    let command = if cfg!(target_os = "windows") {
        CommandSpec::new("cmd")
            .arg("/C")
            .arg("echo out&&>&2 echo err&&exit /B 7")
    } else {
        CommandSpec::new("sh")
            .arg("-c")
            .arg("printf out; printf err >&2; exit 7")
    };

    let result = SandboxRunner::for_platform(Platform::current())
        .run(&command, &SandboxPolicy::danger_full_access())
        .expect("direct process should run");

    assert_eq!(result.stdout.trim_end_matches(['\r', '\n']), "out");
    assert_eq!(result.stderr.trim_end_matches(['\r', '\n']), "err");
    assert_eq!(result.exit_code, Some(7));
    assert!(!result.success());
}

#[test]
fn direct_process_run_passes_stdin_to_child_process() {
    let command = if cfg!(target_os = "windows") {
        CommandSpec::new("cmd")
            .arg("/C")
            .arg("more")
            .stdin("stdin payload")
    } else {
        CommandSpec::new("sh")
            .arg("-c")
            .arg("cat")
            .stdin("stdin payload")
    };

    let result = SandboxRunner::for_platform(Platform::current())
        .run(&command, &SandboxPolicy::danger_full_access())
        .expect("direct process should receive stdin");

    assert!(result.success());
    assert_eq!(
        result.stdout.trim_end_matches(['\r', '\n']),
        "stdin payload"
    );
    assert_eq!(result.stderr, "");
}
