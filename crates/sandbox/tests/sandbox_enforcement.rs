use std::fs;
use std::io::Read;
use std::net::TcpListener;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use sandbox::{CommandSpec, SandboxPolicy, SandboxPreset, SandboxResult, SandboxRunner};

const STRICT_ENV: &str = "MANUAL_SANDBOX_STRICT";

#[test]
fn strict_read_only_allows_workspace_reads() {
    let Some(fixture) = StrictFixture::new("read-only-allows-read") else {
        return;
    };
    let result = fixture.run_probe(
        SandboxPolicy::read_only(&fixture.workspace),
        [
            "read-file".to_string(),
            fixture.workspace.join("allowed.txt").display().to_string(),
            "workspace-data".to_string(),
        ],
    );

    assert_success("read-only workspace read should be allowed", result);
}

#[test]
fn strict_read_only_rejects_workspace_writes() {
    let Some(fixture) = StrictFixture::new("read-only-rejects-write") else {
        return;
    };
    let target = fixture.workspace.join("blocked-write.txt");
    let result = fixture.run_probe(
        SandboxPolicy::read_only(&fixture.workspace),
        [
            "write-file".to_string(),
            target.display().to_string(),
            "blocked".to_string(),
        ],
    );

    assert_failure("read-only workspace write should be denied", result);
    assert!(
        !target.exists(),
        "denied write unexpectedly created {target:?}"
    );
}

#[test]
fn strict_workspace_write_allows_workspace_writes() {
    let Some(fixture) = StrictFixture::new("workspace-write-allows-write") else {
        return;
    };
    let target = fixture.workspace.join("created-by-sandbox.txt");
    let result = fixture.run_probe(
        SandboxPolicy::workspace_write(&fixture.workspace),
        [
            "write-file".to_string(),
            target.display().to_string(),
            "created".to_string(),
        ],
    );

    assert_success("workspace-write should allow workspace writes", result);
    assert_eq!(
        fs::read_to_string(&target).expect("sandbox-created file should be readable"),
        "created"
    );
}

#[test]
fn strict_restricted_policy_rejects_reads_outside_workspace() {
    let Some(fixture) = StrictFixture::new("rejects-outside-read") else {
        return;
    };
    let result = fixture.run_probe(
        SandboxPolicy::read_only(&fixture.workspace),
        [
            "read-file".to_string(),
            fixture.outside.join("secret.txt").display().to_string(),
            "outside-secret".to_string(),
        ],
    );

    assert_failure("outside-workspace read should be denied", result);
}

#[test]
fn strict_workspace_write_rejects_protected_metadata_reads() {
    let Some(fixture) = StrictFixture::new("rejects-metadata-read") else {
        return;
    };
    let result = fixture.run_probe(
        SandboxPolicy::workspace_write(&fixture.workspace),
        [
            "read-file".to_string(),
            fixture
                .workspace
                .join(".git")
                .join("config")
                .display()
                .to_string(),
            "metadata-secret".to_string(),
        ],
    );

    assert_failure("protected metadata read should be denied", result);
}

#[test]
fn strict_workspace_write_rejects_protected_metadata_writes() {
    let Some(fixture) = StrictFixture::new("rejects-metadata-write") else {
        return;
    };
    let target = fixture.workspace.join(".git").join("created");
    let result = fixture.run_probe(
        SandboxPolicy::workspace_write(&fixture.workspace),
        [
            "write-file".to_string(),
            target.display().to_string(),
            "blocked".to_string(),
        ],
    );

    assert_failure("protected metadata write should be denied", result);
    assert!(!target.exists(), "denied metadata write created {target:?}");
}

#[test]
fn strict_danger_full_access_allows_local_tcp_probe() {
    let Some(fixture) = StrictFixture::new("danger-full-access-network") else {
        return;
    };
    let listener = TcpListener::bind("127.0.0.1:0").expect("local TCP listener should bind");
    let addr = listener
        .local_addr()
        .expect("local TCP listener should expose address");
    let result = fixture.run_probe(
        SandboxPolicy::danger_full_access(),
        ["connect-tcp".to_string(), addr.to_string()],
    );

    assert_success("danger-full-access local TCP probe should connect", result);
    assert_listener_received_connection(listener);
}

#[test]
fn strict_restricted_network_rejects_local_tcp_probe() {
    let Some(fixture) = StrictFixture::new("restricted-network") else {
        return;
    };
    let listener = TcpListener::bind("127.0.0.1:0").expect("local TCP listener should bind");
    listener
        .set_nonblocking(true)
        .expect("local TCP listener should become nonblocking");
    let addr = listener
        .local_addr()
        .expect("local TCP listener should expose address");
    let result = fixture.run_probe(
        SandboxPolicy::read_only(&fixture.workspace),
        ["connect-tcp".to_string(), addr.to_string()],
    );

    assert_failure("restricted network should deny local TCP probe", result);
    assert!(
        listener.accept().is_err(),
        "denied network probe still reached listener"
    );
}

struct StrictFixture {
    root: PathBuf,
    workspace: PathBuf,
    outside: PathBuf,
    probe_exe: PathBuf,
}

impl StrictFixture {
    fn new(name: &str) -> Option<Self> {
        if !strict_mode_enabled() {
            eprintln!("skipping strict sandbox enforcement test; set {STRICT_ENV}=1 to enable");
            return None;
        }

        let root = unique_temp_dir(name);
        let workspace = root.join("workspace");
        let outside = root.join("outside");
        let probe_dir = workspace.join("probe-bin");
        let probe_exe = probe_dir.join(current_exe_file_name());

        fs::create_dir_all(&workspace).expect("workspace should be created");
        fs::create_dir_all(&outside).expect("outside dir should be created");
        fs::create_dir_all(workspace.join(".git")).expect("protected metadata dir should exist");
        fs::create_dir_all(&probe_dir).expect("probe bin dir should exist");
        fs::write(workspace.join("allowed.txt"), "workspace-data")
            .expect("workspace fixture should be written");
        fs::write(workspace.join(".git").join("config"), "metadata-secret")
            .expect("metadata fixture should be written");
        fs::write(outside.join("secret.txt"), "outside-secret")
            .expect("outside fixture should be written");
        fs::copy(probe_binary(), &probe_exe)
            .expect("probe executable should be copied inside workspace");

        let root = fs::canonicalize(root).expect("strict fixture root should canonicalize");
        let workspace =
            fs::canonicalize(workspace).expect("strict fixture workspace should canonicalize");
        let outside =
            fs::canonicalize(outside).expect("strict fixture outside should canonicalize");
        let probe_exe =
            fs::canonicalize(probe_exe).expect("strict fixture probe exe should canonicalize");

        Some(Self {
            root,
            workspace,
            outside,
            probe_exe,
        })
    }

    fn run_probe<const N: usize>(&self, policy: SandboxPolicy, args: [String; N]) -> SandboxResult {
        let runner = SandboxRunner::detect();
        let backend = runner.backend();
        if policy.preset != SandboxPreset::DangerFullAccess {
            assert!(
                backend.available,
                "strict sandbox enforcement requires real backend {:?} on {:?}: {}",
                backend.kind, backend.platform, backend.detail
            );
        }

        let command = CommandSpec::new(self.probe_exe.display().to_string())
            .args(args)
            .current_dir(&self.workspace);

        runner
            .run(&command, &policy)
            .expect("strict sandbox probe should launch")
    }
}

impl Drop for StrictFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn strict_mode_enabled() -> bool {
    std::env::var_os(STRICT_ENV).is_some()
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before UNIX_EPOCH")
        .as_nanos();

    std::env::temp_dir().join(format!(
        "manual-sandbox-strict-{name}-{}-{timestamp}",
        std::process::id()
    ))
}

fn current_exe_file_name() -> PathBuf {
    probe_binary()
        .file_name()
        .expect("probe executable should have a file name")
        .into()
}

fn probe_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_manual-sandbox-probe"))
}

fn assert_success(context: &str, result: SandboxResult) {
    assert!(
        result.success(),
        "{context}\nexit: {:?}\nstdout:\n{}\nstderr:\n{}",
        result.exit_code,
        result.stdout,
        result.stderr
    );
}

fn assert_failure(context: &str, result: SandboxResult) {
    assert!(
        !result.success(),
        "{context}\nexit: {:?}\nstdout:\n{}\nstderr:\n{}",
        result.exit_code,
        result.stdout,
        result.stderr
    );
}

fn assert_listener_received_connection(listener: TcpListener) {
    listener
        .set_nonblocking(false)
        .expect("listener should be blocking for positive probe");
    let (mut stream, _) = listener
        .accept()
        .expect("listener should receive positive TCP probe");
    let _ = stream.read(&mut [0; 1]);
}
