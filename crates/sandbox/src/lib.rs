use std::collections::BTreeMap;
use std::fmt;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

const PROTECTED_METADATA_DIRS: &[&str] = &[".git", ".manual", ".codex", ".agents"];
const MACOS_RUNTIME_READ_PATHS: &[&str] = &[
    "/bin",
    "/usr/bin",
    "/usr/lib",
    "/System/Library",
    "/System/Volumes/Preboot/Cryptexes/OS/System/Library",
    "/System/Volumes/Preboot/Cryptexes/OS/usr/lib",
];
const LINUX_RUNTIME_READ_PATHS: &[&str] = &["/bin", "/usr/bin", "/lib", "/lib64", "/usr/lib"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Macos,
    Linux,
    Windows,
    Unsupported(&'static str),
}

impl Platform {
    pub fn current() -> Self {
        match std::env::consts::OS {
            "macos" => Self::Macos,
            "linux" => Self::Linux,
            "windows" => Self::Windows,
            other => Self::Unsupported(other),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Direct,
    MacosSeatbelt,
    LinuxBubblewrap,
    WindowsRestrictedToken,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendReport {
    pub platform: Platform,
    pub kind: BackendKind,
    pub available: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilesystemMode {
    Restricted,
    Unrestricted,
    External,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilesystemAccess {
    Read,
    Write,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesystemEntry {
    pub path: PathBuf,
    pub access: FilesystemAccess,
}

impl FilesystemEntry {
    pub fn new(path: impl Into<PathBuf>, access: FilesystemAccess) -> Self {
        Self {
            path: path.into(),
            access,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesystemPolicy {
    pub mode: FilesystemMode,
    pub entries: Vec<FilesystemEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkMode {
    Restricted,
    Enabled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkPolicy {
    pub mode: NetworkMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxPreset {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxPolicy {
    pub preset: SandboxPreset,
    pub filesystem: FilesystemPolicy,
    pub network: NetworkPolicy,
}

impl SandboxPolicy {
    pub fn read_only(workspace_root: impl Into<PathBuf>) -> Self {
        let workspace_root = workspace_root.into();
        Self {
            preset: SandboxPreset::ReadOnly,
            filesystem: FilesystemPolicy {
                mode: FilesystemMode::Restricted,
                entries: workspace_entries(workspace_root, FilesystemAccess::Read),
            },
            network: NetworkPolicy {
                mode: NetworkMode::Restricted,
            },
        }
    }

    pub fn workspace_write(workspace_root: impl Into<PathBuf>) -> Self {
        let workspace_root = workspace_root.into();
        Self {
            preset: SandboxPreset::WorkspaceWrite,
            filesystem: FilesystemPolicy {
                mode: FilesystemMode::Restricted,
                entries: workspace_entries(workspace_root, FilesystemAccess::Write),
            },
            network: NetworkPolicy {
                mode: NetworkMode::Restricted,
            },
        }
    }

    pub fn danger_full_access() -> Self {
        Self {
            preset: SandboxPreset::DangerFullAccess,
            filesystem: FilesystemPolicy {
                mode: FilesystemMode::Unrestricted,
                entries: Vec::new(),
            },
            network: NetworkPolicy {
                mode: NetworkMode::Enabled,
            },
        }
    }
}

fn workspace_entries(workspace_root: PathBuf, access: FilesystemAccess) -> Vec<FilesystemEntry> {
    let mut entries = vec![FilesystemEntry::new(workspace_root.clone(), access)];

    for protected in PROTECTED_METADATA_DIRS {
        entries.push(FilesystemEntry::new(
            workspace_root.join(protected),
            FilesystemAccess::None,
        ));
    }

    entries
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
    pub current_dir: Option<PathBuf>,
    pub env: BTreeMap<String, String>,
    pub stdin: Option<String>,
}

impl CommandSpec {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            current_dir: None,
            env: BTreeMap::new(),
            stdin: None,
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn current_dir(mut self, current_dir: impl Into<PathBuf>) -> Self {
        self.current_dir = Some(current_dir.into());
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    pub fn stdin(mut self, stdin: impl Into<String>) -> Self {
        self.stdin = Some(stdin.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledSandboxPlan {
    pub backend: BackendReport,
    pub program: String,
    pub args: Vec<String>,
    pub current_dir: Option<PathBuf>,
    pub env: BTreeMap<String, String>,
    pub stdin: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

impl SandboxResult {
    pub fn success(&self) -> bool {
        self.exit_code == Some(0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxError {
    BackendUnavailable {
        backend: BackendKind,
        platform: Platform,
        detail: String,
    },
    UnsupportedPlatform(String),
    Io(String),
}

impl fmt::Display for SandboxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BackendUnavailable {
                backend,
                platform,
                detail,
            } => write!(
                f,
                "sandbox backend {backend:?} is unavailable on {platform:?}: {detail}"
            ),
            Self::UnsupportedPlatform(platform) => {
                write!(f, "unsupported sandbox platform: {platform}")
            }
            Self::Io(message) => write!(f, "sandbox process failed: {message}"),
        }
    }
}

impl std::error::Error for SandboxError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxRunner {
    platform: Platform,
    backend_available: bool,
}

impl SandboxRunner {
    pub fn detect() -> Self {
        let platform = Platform::current();
        Self {
            platform,
            backend_available: backend_available(platform),
        }
    }

    pub fn for_platform(platform: Platform) -> Self {
        Self {
            platform,
            backend_available: true,
        }
    }

    pub fn for_backend_availability(platform: Platform, backend_available: bool) -> Self {
        Self {
            platform,
            backend_available,
        }
    }

    pub fn backend(&self) -> BackendReport {
        backend_report(self.platform, self.backend_available)
    }

    pub fn compile(
        &self,
        command: &CommandSpec,
        policy: &SandboxPolicy,
    ) -> Result<CompiledSandboxPlan, SandboxError> {
        if policy.preset == SandboxPreset::DangerFullAccess {
            return Ok(CompiledSandboxPlan {
                backend: BackendReport {
                    platform: self.platform,
                    kind: BackendKind::Direct,
                    available: true,
                    detail: "sandbox disabled by explicit danger-full-access policy".to_string(),
                },
                program: command.program.clone(),
                args: command.args.clone(),
                current_dir: command.current_dir.clone(),
                env: command.env.clone(),
                stdin: command.stdin.clone(),
            });
        }

        match self.platform {
            Platform::Macos => Ok(compile_macos(command, policy, self.backend_available)),
            Platform::Linux => Ok(compile_linux(command, policy, self.backend_available)),
            Platform::Windows => Ok(compile_windows(command, policy, self.backend_available)),
            Platform::Unsupported(os) => Err(SandboxError::UnsupportedPlatform(os.to_string())),
        }
    }

    pub fn run(
        &self,
        command: &CommandSpec,
        policy: &SandboxPolicy,
    ) -> Result<SandboxResult, SandboxError> {
        let plan = self.compile(command, policy)?;

        if policy.preset != SandboxPreset::DangerFullAccess && !plan.backend.available {
            return Err(SandboxError::BackendUnavailable {
                backend: plan.backend.kind,
                platform: plan.backend.platform,
                detail: plan.backend.detail,
            });
        }

        let mut process = Command::new(&plan.program);
        process.args(&plan.args);
        process.envs(&plan.env);

        if let Some(current_dir) = &plan.current_dir {
            process.current_dir(current_dir);
        }

        let output = if let Some(stdin) = &plan.stdin {
            process.stdin(Stdio::piped());
            process.stdout(Stdio::piped());
            process.stderr(Stdio::piped());

            let mut child = process
                .spawn()
                .map_err(|error| SandboxError::Io(error.to_string()))?;

            if let Some(child_stdin) = child.stdin.as_mut() {
                child_stdin
                    .write_all(stdin.as_bytes())
                    .map_err(|error| SandboxError::Io(error.to_string()))?;
            }

            child
                .wait_with_output()
                .map_err(|error| SandboxError::Io(error.to_string()))?
        } else {
            process
                .output()
                .map_err(|error| SandboxError::Io(error.to_string()))?
        };

        Ok(SandboxResult {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code(),
        })
    }
}

fn backend_report(platform: Platform, available: bool) -> BackendReport {
    let (kind, detail) = match platform {
        Platform::Macos => (
            BackendKind::MacosSeatbelt,
            "requires /usr/bin/sandbox-exec and generated SBPL policy",
        ),
        Platform::Linux => (
            BackendKind::LinuxBubblewrap,
            "requires bubblewrap plus follow-up seccomp/no_new_privs helper",
        ),
        Platform::Windows => (
            BackendKind::WindowsRestrictedToken,
            "requires restricted token, ACL setup, and process runner",
        ),
        Platform::Unsupported(_) => (BackendKind::Unsupported, "no backend for this platform"),
    };

    BackendReport {
        platform,
        kind,
        available,
        detail: detail.to_string(),
    }
}

fn backend_available(platform: Platform) -> bool {
    match platform {
        Platform::Macos => Path::new("/usr/bin/sandbox-exec").is_file(),
        Platform::Linux => command_exists("bwrap"),
        Platform::Windows => false,
        Platform::Unsupported(_) => false,
    }
}

fn command_exists(program: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };

    std::env::split_paths(&path).any(|dir| dir.join(program).is_file())
}

fn compile_macos(
    command: &CommandSpec,
    policy: &SandboxPolicy,
    backend_available: bool,
) -> CompiledSandboxPlan {
    let sbpl = macos_sbpl(policy);
    let mut args = vec![
        "-p".to_string(),
        sbpl,
        "--".to_string(),
        command.program.clone(),
    ];
    args.extend(command.args.clone());

    CompiledSandboxPlan {
        backend: backend_report(Platform::Macos, backend_available),
        program: "/usr/bin/sandbox-exec".to_string(),
        args,
        current_dir: command.current_dir.clone(),
        env: command.env.clone(),
        stdin: command.stdin.clone(),
    }
}

fn macos_sbpl(policy: &SandboxPolicy) -> String {
    let mut policy_lines = vec![
        "(version 1)".to_string(),
        "(deny default)".to_string(),
        "(allow process*)".to_string(),
        "(allow sysctl*)".to_string(),
        "(allow file-read-metadata)".to_string(),
        "(allow file-read-data (literal \"/\"))".to_string(),
    ];

    for path in MACOS_RUNTIME_READ_PATHS {
        let quoted = sbpl_quote_path(Path::new(path));
        policy_lines.push(format!(
            "(allow file-read* file-map-executable (subpath {quoted}))"
        ));
    }

    for entry in &policy.filesystem.entries {
        let quoted = sbpl_quote_path(&entry.path);
        match entry.access {
            FilesystemAccess::Read => {
                policy_lines.push(format!("(allow file-read* (subpath {quoted}))"));
                policy_lines.push(format!("(allow file-map-executable (subpath {quoted}))"));
            }
            FilesystemAccess::Write => {
                policy_lines.push(format!("(allow file-read* (subpath {quoted}))"));
                policy_lines.push(format!("(allow file-write* (subpath {quoted}))"));
                policy_lines.push(format!("(allow file-map-executable (subpath {quoted}))"));
            }
            FilesystemAccess::None => {
                policy_lines.push(format!("(deny file-read* file-write* (subpath {quoted}))"));
            }
        }
    }

    if policy.network.mode == NetworkMode::Enabled {
        policy_lines.push("(allow network*)".to_string());
    }

    policy_lines.join("\n")
}

fn sbpl_quote_path(path: &Path) -> String {
    let normalized = path.display().to_string().replace('\\', "/");

    format!(
        "\"{}\"",
        normalized.replace('\\', "\\\\").replace('"', "\\\"")
    )
}

fn compile_linux(
    command: &CommandSpec,
    policy: &SandboxPolicy,
    backend_available: bool,
) -> CompiledSandboxPlan {
    let mut args = vec![
        "--unshare-user".to_string(),
        "--uid".to_string(),
        "0".to_string(),
        "--gid".to_string(),
        "0".to_string(),
        "--unshare-pid".to_string(),
        "--die-with-parent".to_string(),
        "--proc".to_string(),
        "/proc".to_string(),
        "--dev".to_string(),
        "/dev".to_string(),
    ];

    if policy.network.mode == NetworkMode::Restricted {
        args.push("--unshare-net".to_string());
    }

    for path in LINUX_RUNTIME_READ_PATHS {
        let runtime_path = Path::new(path);
        if runtime_path.exists() {
            args.push("--ro-bind".to_string());
            args.push((*path).to_string());
            args.push((*path).to_string());
        }
    }

    for entry in &policy.filesystem.entries {
        match entry.access {
            FilesystemAccess::Read => {
                args.push("--ro-bind".to_string());
                args.push(entry.path.display().to_string());
                args.push(entry.path.display().to_string());
            }
            FilesystemAccess::Write => {
                args.push("--bind".to_string());
                args.push(entry.path.display().to_string());
                args.push(entry.path.display().to_string());
            }
            FilesystemAccess::None => {
                if entry.path.exists() || path_is_under_writable_entry(policy, &entry.path) {
                    args.push("--tmpfs".to_string());
                    args.push(entry.path.display().to_string());
                    args.push("--remount-ro".to_string());
                    args.push(entry.path.display().to_string());
                }
            }
        }
    }

    args.push("--".to_string());
    args.push(command.program.clone());
    args.extend(command.args.clone());

    CompiledSandboxPlan {
        backend: backend_report(Platform::Linux, backend_available),
        program: "bwrap".to_string(),
        args,
        current_dir: command.current_dir.clone(),
        env: command.env.clone(),
        stdin: command.stdin.clone(),
    }
}

fn compile_windows(
    command: &CommandSpec,
    policy: &SandboxPolicy,
    backend_available: bool,
) -> CompiledSandboxPlan {
    let mut args = vec!["--restricted-token".to_string()];

    for entry in &policy.filesystem.entries {
        match entry.access {
            FilesystemAccess::Read => args.push("--allow-read".to_string()),
            FilesystemAccess::Write => args.push("--allow-write".to_string()),
            FilesystemAccess::None => args.push("--deny".to_string()),
        }
        args.push(entry.path.display().to_string());
    }

    if policy.network.mode == NetworkMode::Restricted {
        args.push("--deny-network".to_string());
    }

    args.push("--".to_string());
    args.push(command.program.clone());
    args.extend(command.args.clone());

    CompiledSandboxPlan {
        backend: backend_report(Platform::Windows, backend_available),
        program: "manual-windows-sandbox-runner".to_string(),
        args,
        current_dir: command.current_dir.clone(),
        env: command.env.clone(),
        stdin: command.stdin.clone(),
    }
}

fn path_is_under_writable_entry(policy: &SandboxPolicy, path: &Path) -> bool {
    policy.filesystem.entries.iter().any(|entry| {
        entry.access == FilesystemAccess::Write
            && path != entry.path
            && path.starts_with(&entry.path)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::ffi::OsString;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Mutex;
    use std::sync::MutexGuard;
    use std::time::SystemTime;
    use std::time::UNIX_EPOCH;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct PathEnvGuard {
        original: Option<OsString>,
        _guard: MutexGuard<'static, ()>,
    }

    impl PathEnvGuard {
        fn set(path: OsString) -> Self {
            let guard = ENV_LOCK.lock().expect("PATH environment lock poisoned");
            let original = std::env::var_os("PATH");

            unsafe {
                std::env::set_var("PATH", path);
            }

            Self {
                original,
                _guard: guard,
            }
        }
    }

    impl Drop for PathEnvGuard {
        fn drop(&mut self) {
            unsafe {
                match &self.original {
                    Some(original) => std::env::set_var("PATH", original),
                    None => std::env::remove_var("PATH"),
                }
            }
        }
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is before UNIX_EPOCH")
            .as_nanos();

        std::env::temp_dir().join(format!(
            "manual-sandbox-{name}-{}-{timestamp}",
            std::process::id()
        ))
    }

    #[test]
    fn linux_backend_availability_follows_path_lookup() {
        let temp_root = unique_temp_dir("path");
        let fake_bin = temp_root.join("bin");
        let empty_bin = temp_root.join("empty");

        fs::create_dir_all(&fake_bin).expect("fake bin dir should be created");
        fs::create_dir_all(&empty_bin).expect("empty bin dir should be created");
        fs::write(fake_bin.join("bwrap"), "").expect("fake bwrap should be created");

        let fake_path = std::env::join_paths([fake_bin.as_path()]).expect("valid fake PATH");
        let _fake_path_guard = PathEnvGuard::set(fake_path);
        assert!(command_exists("bwrap"));
        assert!(backend_available(Platform::Linux));
        drop(_fake_path_guard);

        let empty_path = std::env::join_paths([empty_bin.as_path()]).expect("valid empty PATH");
        let _empty_path_guard = PathEnvGuard::set(empty_path);
        assert!(!command_exists("bwrap"));
        assert!(!backend_available(Platform::Linux));
        drop(_empty_path_guard);

        fs::remove_dir_all(&temp_root).expect("temp backend PATH should be removed");
    }

    #[test]
    fn unavailable_backend_platforms_report_unavailable() {
        assert!(!backend_available(Platform::Windows));
        assert!(!backend_available(Platform::Unsupported("haiku")));
    }
}
