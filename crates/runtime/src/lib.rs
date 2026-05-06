use std::fmt;
use std::path::PathBuf;

use agent::AgentCommand;
use sandbox::{
    BackendReport, CommandSpec, FilesystemAccess, FilesystemEntry, FilesystemMode, SandboxError,
    SandboxPolicy, SandboxResult, SandboxRunner,
};
use script::{CompiledScript, ScriptError, ScriptRequest, ScriptRunner};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeSandbox {
    ReadOnly { workspace_root: PathBuf },
    WorkspaceWrite { workspace_root: PathBuf },
    DangerFullAccess,
    Policy(SandboxPolicy),
}

impl RuntimeSandbox {
    pub fn read_only(workspace_root: impl Into<PathBuf>) -> Self {
        Self::ReadOnly {
            workspace_root: workspace_root.into(),
        }
    }

    pub fn workspace_write(workspace_root: impl Into<PathBuf>) -> Self {
        Self::WorkspaceWrite {
            workspace_root: workspace_root.into(),
        }
    }

    pub fn danger_full_access() -> Self {
        Self::DangerFullAccess
    }

    pub fn policy(policy: SandboxPolicy) -> Self {
        Self::Policy(policy)
    }

    pub fn to_policy(&self) -> SandboxPolicy {
        match self {
            Self::ReadOnly { workspace_root } => SandboxPolicy::read_only(workspace_root),
            Self::WorkspaceWrite { workspace_root } => {
                SandboxPolicy::workspace_write(workspace_root)
            }
            Self::DangerFullAccess => SandboxPolicy::danger_full_access(),
            Self::Policy(policy) => policy.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeTargetKind {
    Script,
    Agent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeTarget {
    Script { rust_source: String },
    Agent { command: AgentCommand },
}

impl RuntimeTarget {
    pub fn script(rust_source: impl Into<String>) -> Self {
        Self::Script {
            rust_source: rust_source.into(),
        }
    }

    pub fn agent(command: AgentCommand) -> Self {
        Self::Agent { command }
    }

    pub fn kind(&self) -> RuntimeTargetKind {
        match self {
            Self::Script { .. } => RuntimeTargetKind::Script,
            Self::Agent { .. } => RuntimeTargetKind::Agent,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeRequest {
    pub input_json: String,
    pub sandbox: RuntimeSandbox,
    pub target: RuntimeTarget,
}

impl RuntimeRequest {
    pub fn new(
        input_json: impl Into<String>,
        sandbox: RuntimeSandbox,
        target: RuntimeTarget,
    ) -> Self {
        Self {
            input_json: input_json.into(),
            sandbox,
            target,
        }
    }

    pub fn script(
        input_json: impl Into<String>,
        rust_source: impl Into<String>,
        sandbox: RuntimeSandbox,
    ) -> Self {
        Self::new(input_json, sandbox, RuntimeTarget::script(rust_source))
    }

    pub fn agent(
        input_json: impl Into<String>,
        command: AgentCommand,
        sandbox: RuntimeSandbox,
    ) -> Self {
        Self::new(input_json, sandbox, RuntimeTarget::agent(command))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeOutput {
    pub target: RuntimeTargetKind,
    pub sandbox: BackendReport,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

impl RuntimeOutput {
    pub fn success(&self) -> bool {
        self.exit_code == Some(0)
    }
}

#[derive(Debug)]
pub struct RuntimePlan {
    pub target: RuntimeTargetKind,
    pub sandbox: BackendReport,
    pub command: CommandSpec,
    pub policy: SandboxPolicy,
    _compiled_script: Option<CompiledScript>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    Sandbox(SandboxError),
    Script(ScriptError),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sandbox(error) => write!(f, "runtime sandbox failed: {error}"),
            Self::Script(error) => write!(f, "runtime script failed: {error}"),
        }
    }
}

impl std::error::Error for RuntimeError {}

impl From<SandboxError> for RuntimeError {
    fn from(error: SandboxError) -> Self {
        Self::Sandbox(error)
    }
}

impl From<ScriptError> for RuntimeError {
    fn from(error: ScriptError) -> Self {
        Self::Script(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeRunner {
    sandbox_runner: SandboxRunner,
    script_runner: ScriptRunner,
}

impl Default for RuntimeRunner {
    fn default() -> Self {
        Self::detect()
    }
}

impl RuntimeRunner {
    pub fn detect() -> Self {
        Self {
            sandbox_runner: SandboxRunner::detect(),
            script_runner: ScriptRunner::default(),
        }
    }

    pub fn for_sandbox_runner(sandbox_runner: SandboxRunner) -> Self {
        Self {
            sandbox_runner,
            script_runner: ScriptRunner::default(),
        }
    }

    pub fn with_script_runner(mut self, script_runner: ScriptRunner) -> Self {
        self.script_runner = script_runner;
        self
    }

    pub fn compile(&self, request: &RuntimeRequest) -> Result<RuntimePlan, RuntimeError> {
        match &request.target {
            RuntimeTarget::Script { rust_source } => self.compile_script(request, rust_source),
            RuntimeTarget::Agent { command } => self.compile_agent(request, command),
        }
    }

    pub fn run(&self, request: &RuntimeRequest) -> Result<RuntimeOutput, RuntimeError> {
        let plan = self.compile(request)?;
        self.run_plan(plan)
    }

    fn compile_script(
        &self,
        request: &RuntimeRequest,
        rust_source: &str,
    ) -> Result<RuntimePlan, RuntimeError> {
        let script_request = ScriptRequest::new(rust_source, &request.input_json);
        let compiled = self.script_runner.compile(&script_request)?;
        let mut policy = request.sandbox.to_policy();

        if policy.filesystem.mode == FilesystemMode::Restricted {
            policy.filesystem.entries.push(FilesystemEntry::new(
                compiled.workspace_path(),
                FilesystemAccess::Read,
            ));
        }

        let command = CommandSpec::new(compiled.binary_path().display().to_string())
            .stdin(compiled.input_json());
        let plan = self.sandbox_runner.compile(&command, &policy)?;

        Ok(RuntimePlan {
            target: RuntimeTargetKind::Script,
            sandbox: plan.backend,
            command,
            policy,
            _compiled_script: Some(compiled),
        })
    }

    fn compile_agent(
        &self,
        request: &RuntimeRequest,
        command: &AgentCommand,
    ) -> Result<RuntimePlan, RuntimeError> {
        let mut spec = CommandSpec::new(command.program())
            .args(command.args().iter().cloned())
            .stdin(&request.input_json);

        if let Some(current_dir) = command.current_dir() {
            spec = spec.current_dir(current_dir);
        }

        let policy = request.sandbox.to_policy();
        let plan = self.sandbox_runner.compile(&spec, &policy)?;

        Ok(RuntimePlan {
            target: RuntimeTargetKind::Agent,
            sandbox: plan.backend,
            command: spec,
            policy,
            _compiled_script: None,
        })
    }

    fn run_plan(&self, plan: RuntimePlan) -> Result<RuntimeOutput, RuntimeError> {
        let result = self.sandbox_runner.run(&plan.command, &plan.policy)?;

        Ok(runtime_output(plan.target, plan.sandbox, result))
    }
}

fn runtime_output(
    target: RuntimeTargetKind,
    sandbox: BackendReport,
    result: SandboxResult,
) -> RuntimeOutput {
    RuntimeOutput {
        target,
        sandbox,
        stdout: result.stdout,
        stderr: result.stderr,
        exit_code: result.exit_code,
    }
}
