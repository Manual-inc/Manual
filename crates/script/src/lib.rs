use std::fmt;
use std::fs;
use std::io;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

static WORKSPACE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptRequest {
    pub rust_source: String,
    pub input_json: String,
    pub dependencies: Vec<ScriptDependency>,
}

impl ScriptRequest {
    pub fn new(rust_source: impl Into<String>, input_json: impl Into<String>) -> Self {
        Self {
            rust_source: rust_source.into(),
            input_json: input_json.into(),
            dependencies: Vec::new(),
        }
    }

    pub fn with_dependency(mut self, dependency: ScriptDependency) -> Self {
        self.dependencies.push(dependency);
        self
    }

    pub fn with_dependencies<I>(mut self, dependencies: I) -> Self
    where
        I: IntoIterator<Item = ScriptDependency>,
    {
        self.dependencies.extend(dependencies);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptDependency {
    pub name: String,
    pub source: ScriptDependencySource,
}

impl ScriptDependency {
    pub fn version(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source: ScriptDependencySource::Version(version.into()),
        }
    }

    pub fn path(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            name: name.into(),
            source: ScriptDependencySource::Path(path.into()),
        }
    }

    pub fn manifest_value(name: impl Into<String>, manifest_value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source: ScriptDependencySource::ManifestValue(manifest_value.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptDependencySource {
    Version(String),
    Path(PathBuf),
    ManifestValue(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

impl ScriptOutput {
    pub fn success(&self) -> bool {
        self.exit_code == Some(0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptError {
    Io(String),
    CompileFailed {
        stdout: String,
        stderr: String,
        exit_code: Option<i32>,
    },
}

impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message) => write!(f, "script process failed: {message}"),
            Self::CompileFailed {
                stderr, exit_code, ..
            } => write!(
                f,
                "script compilation failed with exit code {exit_code:?}: {stderr}"
            ),
        }
    }
}

impl std::error::Error for ScriptError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptRunner {
    cargo: PathBuf,
    rustc: PathBuf,
    temp_root: PathBuf,
    edition: String,
}

impl Default for ScriptRunner {
    fn default() -> Self {
        Self {
            cargo: std::env::var_os("CARGO")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("cargo")),
            rustc: std::env::var_os("RUSTC")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("rustc")),
            temp_root: std::env::temp_dir(),
            edition: "2024".to_string(),
        }
    }
}

impl ScriptRunner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_cargo(mut self, cargo: impl Into<PathBuf>) -> Self {
        self.cargo = cargo.into();
        self
    }

    pub fn with_rustc(mut self, rustc: impl Into<PathBuf>) -> Self {
        self.rustc = rustc.into();
        self
    }

    pub fn with_temp_root(mut self, temp_root: impl Into<PathBuf>) -> Self {
        self.temp_root = temp_root.into();
        self
    }

    pub fn compile(&self, request: &ScriptRequest) -> Result<CompiledScript, ScriptError> {
        if !request.dependencies.is_empty() {
            return self.compile_with_cargo(request);
        }

        self.compile_with_rustc(request)
    }

    fn compile_with_rustc(&self, request: &ScriptRequest) -> Result<CompiledScript, ScriptError> {
        let workspace = ScriptWorkspace::create(&self.temp_root)?;
        let user_source_path = workspace.path().join("script.rs");
        let wrapper_path = workspace.path().join("main.rs");
        let binary_path = workspace.path().join(binary_name());

        fs::write(&user_source_path, &request.rust_source).map_err(to_io_error)?;
        fs::write(&wrapper_path, wrapper_source(&user_source_path)).map_err(to_io_error)?;

        let compile_output = Command::new(&self.rustc)
            .arg("--edition")
            .arg(&self.edition)
            .arg(&wrapper_path)
            .arg("-o")
            .arg(&binary_path)
            .output()
            .map_err(to_io_error)?;

        if !compile_output.status.success() {
            return Err(ScriptError::CompileFailed {
                stdout: String::from_utf8_lossy(&compile_output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&compile_output.stderr).into_owned(),
                exit_code: compile_output.status.code(),
            });
        }

        Ok(CompiledScript {
            workspace,
            binary_path,
            input_json: request.input_json.clone(),
        })
    }

    fn compile_with_cargo(&self, request: &ScriptRequest) -> Result<CompiledScript, ScriptError> {
        let workspace = ScriptWorkspace::create(&self.temp_root)?;
        let src_dir = workspace.path().join("src");
        let user_source_path = src_dir.join("script.rs");
        let wrapper_path = src_dir.join("main.rs");
        let manifest_path = workspace.path().join("Cargo.toml");
        let binary_path = workspace
            .path()
            .join("target")
            .join("debug")
            .join(cargo_binary_name());

        fs::create_dir(&src_dir).map_err(to_io_error)?;
        fs::write(&user_source_path, &request.rust_source).map_err(to_io_error)?;
        fs::write(&wrapper_path, wrapper_source(&user_source_path)).map_err(to_io_error)?;
        fs::write(
            &manifest_path,
            cargo_manifest(&request.dependencies, &self.edition),
        )
        .map_err(to_io_error)?;

        let compile_output = Command::new(&self.cargo)
            .arg("build")
            .arg("--quiet")
            .arg("--manifest-path")
            .arg(&manifest_path)
            .output()
            .map_err(to_io_error)?;

        if !compile_output.status.success() {
            return Err(ScriptError::CompileFailed {
                stdout: String::from_utf8_lossy(&compile_output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&compile_output.stderr).into_owned(),
                exit_code: compile_output.status.code(),
            });
        }

        Ok(CompiledScript {
            workspace,
            binary_path,
            input_json: request.input_json.clone(),
        })
    }

    pub fn run(&self, request: &ScriptRequest) -> Result<ScriptOutput, ScriptError> {
        let compiled = self.compile(request)?;

        let mut child = Command::new(compiled.binary_path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(to_io_error)?;

        if let Some(stdin) = child.stdin.as_mut() {
            stdin
                .write_all(compiled.input_json().as_bytes())
                .map_err(to_io_error)?;
        }

        let output = child.wait_with_output().map_err(to_io_error)?;

        Ok(ScriptOutput {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code(),
        })
    }
}

#[derive(Debug)]
pub struct CompiledScript {
    workspace: ScriptWorkspace,
    binary_path: PathBuf,
    input_json: String,
}

impl CompiledScript {
    pub fn binary_path(&self) -> &Path {
        &self.binary_path
    }

    pub fn workspace_path(&self) -> &Path {
        self.workspace.path()
    }

    pub fn input_json(&self) -> &str {
        &self.input_json
    }
}

#[derive(Debug)]
struct ScriptWorkspace {
    path: PathBuf,
}

impl ScriptWorkspace {
    fn create(temp_root: &Path) -> Result<Self, ScriptError> {
        let path = temp_root.join(format!(
            "manual-script-{}-{}-{}",
            std::process::id(),
            timestamp_nanos(),
            WORKSPACE_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));

        fs::create_dir(&path).map_err(to_io_error)?;

        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ScriptWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn wrapper_source(user_source_path: &Path) -> String {
    format!(
        r#"
mod manual_script {{
    include!({});

    pub fn __manual_run(input_json: &str) -> impl std::fmt::Display {{
        main(input_json)
    }}
}}

fn main() {{
    let mut input_json = String::new();
    let mut stdin = std::io::stdin();

    if let Err(error) = std::io::Read::read_to_string(&mut stdin, &mut input_json) {{
        eprintln!("failed to read JSON input: {{error}}");
        std::process::exit(125);
    }}

    let output = manual_script::__manual_run(&input_json);
    print!("{{output}}");
}}
"#,
        rust_string_literal(&user_source_path.display().to_string())
    )
}

fn cargo_manifest(dependencies: &[ScriptDependency], edition: &str) -> String {
    let mut manifest = format!(
        r#"[package]
name = "manual-script-runner"
version = "0.1.0"
edition = {}

[dependencies]
"#,
        toml_string_literal(edition)
    );

    for dependency in dependencies {
        manifest.push_str(&dependency.name);
        manifest.push_str(" = ");
        manifest.push_str(&dependency_manifest_value(dependency));
        manifest.push('\n');
    }

    manifest.push_str("\n[workspace]\n");
    manifest
}

fn dependency_manifest_value(dependency: &ScriptDependency) -> String {
    match &dependency.source {
        ScriptDependencySource::Version(version) => toml_string_literal(version),
        ScriptDependencySource::Path(path) => {
            format!(
                "{{ path = {} }}",
                toml_string_literal(&path.display().to_string())
            )
        }
        ScriptDependencySource::ManifestValue(value) => value.clone(),
    }
}

fn rust_string_literal(value: &str) -> String {
    quoted_string_literal(value)
}

fn toml_string_literal(value: &str) -> String {
    quoted_string_literal(value)
}

fn quoted_string_literal(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');

    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other => escaped.push(other),
        }
    }

    escaped.push('"');
    escaped
}

fn binary_name() -> &'static str {
    if cfg!(windows) {
        "script.exe"
    } else {
        "script"
    }
}

fn cargo_binary_name() -> &'static str {
    if cfg!(windows) {
        "manual-script-runner.exe"
    } else {
        "manual-script-runner"
    }
}

fn timestamp_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

fn to_io_error(error: io::Error) -> ScriptError {
    ScriptError::Io(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_name_matches_host_executable_convention() {
        if cfg!(windows) {
            assert_eq!(binary_name(), "script.exe");
        } else {
            assert_eq!(binary_name(), "script");
        }
    }

    #[test]
    fn timestamp_uses_current_unix_time() {
        assert!(timestamp_nanos() > 1);
    }

    #[test]
    fn cargo_manifest_renders_supported_dependency_sources() {
        let manifest = cargo_manifest(
            &[
                ScriptDependency::version("serde_json", "1"),
                ScriptDependency::path("helper-package", "/tmp/helper"),
                ScriptDependency::manifest_value(
                    "regex",
                    r#"{ version = "1", default-features = false }"#,
                ),
            ],
            "2024",
        );

        assert!(manifest.contains("edition = \"2024\""));
        assert!(manifest.contains("serde_json = \"1\""));
        assert!(manifest.contains("helper-package = { path = \"/tmp/helper\" }"));
        assert!(manifest.contains(r#"regex = { version = "1", default-features = false }"#));
        assert!(manifest.contains("[workspace]"));
    }
}
