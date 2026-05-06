use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(String);

impl NodeId {
    pub fn new(value: impl Into<String>) -> Result<Self, NodeIdError> {
        let value = value.into();

        if value.trim().is_empty() {
            return Err(NodeIdError::Empty);
        }

        if value.chars().any(char::is_whitespace) {
            return Err(NodeIdError::ContainsWhitespace(value));
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeIdError {
    Empty,
    ContainsWhitespace(String),
}

impl fmt::Display for NodeIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "node id cannot be empty"),
            Self::ContainsWhitespace(value) => {
                write!(f, "node id cannot contain whitespace: {value}")
            }
        }
    }
}

impl std::error::Error for NodeIdError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NodeKind {
    Trigger,
    LlmTask,
    CodeTask,
    Integration,
    Condition,
    Loop,
    Join,
    Approval,
    Artifact,
}

impl NodeKind {
    pub const ALL: [Self; 9] = [
        Self::Trigger,
        Self::LlmTask,
        Self::CodeTask,
        Self::Integration,
        Self::Condition,
        Self::Loop,
        Self::Join,
        Self::Approval,
        Self::Artifact,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Trigger => "trigger",
            Self::LlmTask => "llm_task",
            Self::CodeTask => "code_task",
            Self::Integration => "integration",
            Self::Condition => "condition",
            Self::Loop => "loop",
            Self::Join => "join",
            Self::Approval => "approval",
            Self::Artifact => "artifact",
        }
    }
}

impl fmt::Display for NodeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for NodeKind {
    type Err = NodeKindParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "trigger" => Ok(Self::Trigger),
            "llm_task" => Ok(Self::LlmTask),
            "code_task" => Ok(Self::CodeTask),
            "integration" => Ok(Self::Integration),
            "condition" => Ok(Self::Condition),
            "loop" => Ok(Self::Loop),
            "join" => Ok(Self::Join),
            "approval" => Ok(Self::Approval),
            "artifact" => Ok(Self::Artifact),
            other => Err(NodeKindParseError::Unknown(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKindParseError {
    Unknown(String),
}

impl fmt::Display for NodeKindParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(value) => write!(f, "unknown node kind: {value}"),
        }
    }
}

impl std::error::Error for NodeKindParseError {}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NodeContract {
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub sandbox: Option<String>,
    pub runtime: Option<String>,
    pub artifacts: Vec<String>,
    pub acceptance: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub id: NodeId,
    pub kind: NodeKind,
    pub description: String,
    pub contract: NodeContract,
}

impl Node {
    pub fn new(
        id: impl Into<String>,
        kind: NodeKind,
        description: impl Into<String>,
    ) -> Result<Self, NodeIdError> {
        Ok(Self {
            id: NodeId::new(id)?,
            kind,
            description: description.into(),
            contract: NodeContract::default(),
        })
    }

    pub fn with_input(mut self, input: impl Into<String>) -> Self {
        self.contract.inputs.push(input.into());
        self
    }

    pub fn with_output(mut self, output: impl Into<String>) -> Self {
        self.contract.outputs.push(output.into());
        self
    }

    pub fn with_sandbox(mut self, sandbox: impl Into<String>) -> Self {
        self.contract.sandbox = Some(sandbox.into());
        self
    }

    pub fn with_runtime(mut self, runtime: impl Into<String>) -> Self {
        self.contract.runtime = Some(runtime.into());
        self
    }

    pub fn with_artifact(mut self, artifact: impl Into<String>) -> Self {
        self.contract.artifacts.push(artifact.into());
        self
    }

    pub fn with_acceptance(mut self, acceptance: impl Into<String>) -> Self {
        self.contract.acceptance = Some(acceptance.into());
        self
    }
}
