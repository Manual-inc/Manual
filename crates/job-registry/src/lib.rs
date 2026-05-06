use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use job::{Job, JobError, JobId, JobIdError, JobNode, JobStatus, NodeRunStatus};
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JobRegistry {
    jobs: BTreeMap<JobId, Job>,
}

impl JobRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, job: Job) -> Result<(), JobRegistryError> {
        if self.jobs.contains_key(job.id()) {
            return Err(JobRegistryError::DuplicateId(job.id().clone()));
        }

        self.jobs.insert(job.id().clone(), job);
        Ok(())
    }

    pub fn resolve(&self, id: impl Into<String>) -> Result<&Job, JobRegistryError> {
        let id = JobId::new(id).map_err(JobRegistryError::InvalidId)?;

        self.jobs.get(&id).ok_or(JobRegistryError::UnknownId(id))
    }

    pub fn get(&self, id: &JobId) -> Option<&Job> {
        self.jobs.get(id)
    }

    pub fn get_mut(&mut self, id: &JobId) -> Option<&mut Job> {
        self.jobs.get_mut(id)
    }

    pub fn remove(&mut self, id: &JobId) -> Option<Job> {
        self.jobs.remove(id)
    }

    pub fn len(&self) -> usize {
        self.jobs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Job> {
        self.jobs.values()
    }

    pub fn jobs_for_workflow<'a>(
        &'a self,
        workflow_id: &'a str,
    ) -> impl Iterator<Item = &'a Job> + 'a {
        self.jobs
            .values()
            .filter(move |job| job.workflow_id() == workflow_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobRegistryError {
    InvalidId(JobIdError),
    DuplicateId(JobId),
    UnknownId(JobId),
}

impl fmt::Display for JobRegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId(error) => write!(f, "{error}"),
            Self::DuplicateId(id) => write!(f, "duplicate job id: {id}"),
            Self::UnknownId(id) => write!(f, "unknown job id: {id}"),
        }
    }
}

impl std::error::Error for JobRegistryError {}

pub trait JobStore {
    fn load(&self) -> Result<JobRegistry, JobStoreError>;

    fn save(&self, registry: &JobRegistry) -> Result<(), JobStoreError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileJobStore {
    path: PathBuf,
}

impl FileJobStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn for_repository(repository_root: impl AsRef<Path>) -> Self {
        Self::new(repository_root.as_ref().join(".manual").join("jobs.toml"))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl JobStore for FileJobStore {
    fn load(&self) -> Result<JobRegistry, JobStoreError> {
        if !self.path.exists() {
            return Ok(JobRegistry::new());
        }

        let contents =
            fs::read_to_string(&self.path).map_err(|error| JobStoreError::io(&self.path, error))?;
        let document: StoredJobDocument =
            toml::from_str(&contents).map_err(|error| JobStoreError::Decode {
                path: self.path.clone(),
                message: error.to_string(),
            })?;

        document.into_registry()
    }

    fn save(&self, registry: &JobRegistry) -> Result<(), JobStoreError> {
        let document = StoredJobDocument::from_registry(registry);
        let contents =
            toml::to_string_pretty(&document).map_err(|error| JobStoreError::Encode {
                message: error.to_string(),
            })?;

        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|error| JobStoreError::io(parent, error))?;
            }
        }

        fs::write(&self.path, contents).map_err(|error| JobStoreError::io(&self.path, error))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobStoreError {
    Io { path: PathBuf, message: String },
    Decode { path: PathBuf, message: String },
    Encode { message: String },
    Registry(JobRegistryError),
    Job(JobError),
}

impl JobStoreError {
    fn io(path: &Path, error: std::io::Error) -> Self {
        Self::Io {
            path: path.to_path_buf(),
            message: error.to_string(),
        }
    }
}

impl fmt::Display for JobStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, message } => {
                write!(f, "job store I/O failed at {}: {message}", path.display())
            }
            Self::Decode { path, message } => {
                write!(
                    f,
                    "job store failed to decode {}: {message}",
                    path.display()
                )
            }
            Self::Encode { message } => write!(f, "job store failed to encode: {message}"),
            Self::Registry(error) => write!(f, "{error}"),
            Self::Job(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for JobStoreError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredJobDocument {
    jobs: Vec<StoredJob>,
}

impl StoredJobDocument {
    fn from_registry(registry: &JobRegistry) -> Self {
        Self {
            jobs: registry.iter().map(StoredJob::from_job).collect(),
        }
    }

    fn into_registry(self) -> Result<JobRegistry, JobStoreError> {
        let mut registry = JobRegistry::new();

        for job in self.jobs {
            registry
                .insert(job.into_job()?)
                .map_err(JobStoreError::Registry)?;
        }

        Ok(registry)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredJob {
    id: String,
    workflow_id: String,
    input_json: String,
    status: StoredJobStatus,
    nodes: Vec<StoredJobNode>,
}

impl StoredJob {
    fn from_job(job: &Job) -> Self {
        Self {
            id: job.id().as_str().to_string(),
            workflow_id: job.workflow_id().to_string(),
            input_json: job.input_json().to_string(),
            status: job.status().into(),
            nodes: job.nodes().iter().map(StoredJobNode::from_node).collect(),
        }
    }

    fn into_job(self) -> Result<Job, JobStoreError> {
        let nodes = self
            .nodes
            .into_iter()
            .map(StoredJobNode::into_node)
            .collect::<Result<Vec<_>, _>>()?;

        Job::restore(
            self.id,
            self.workflow_id,
            self.input_json,
            self.status.into(),
            nodes,
        )
        .map_err(JobStoreError::Job)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredJobNode {
    node_id: String,
    status: StoredNodeRunStatus,
    attempts: u32,
}

impl StoredJobNode {
    fn from_node(node: &JobNode) -> Self {
        Self {
            node_id: node.node_id().as_str().to_string(),
            status: node.status().into(),
            attempts: node.attempts(),
        }
    }

    fn into_node(self) -> Result<JobNode, JobStoreError> {
        JobNode::restore(self.node_id, self.status.into(), self.attempts)
            .map_err(JobError::InvalidNodeId)
            .map_err(JobStoreError::Job)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StoredJobStatus {
    Created,
    Running,
    Succeeded,
    Failed,
    Canceled,
}

impl From<JobStatus> for StoredJobStatus {
    fn from(value: JobStatus) -> Self {
        match value {
            JobStatus::Created => Self::Created,
            JobStatus::Running => Self::Running,
            JobStatus::Succeeded => Self::Succeeded,
            JobStatus::Failed => Self::Failed,
            JobStatus::Canceled => Self::Canceled,
        }
    }
}

impl From<StoredJobStatus> for JobStatus {
    fn from(value: StoredJobStatus) -> Self {
        match value {
            StoredJobStatus::Created => Self::Created,
            StoredJobStatus::Running => Self::Running,
            StoredJobStatus::Succeeded => Self::Succeeded,
            StoredJobStatus::Failed => Self::Failed,
            StoredJobStatus::Canceled => Self::Canceled,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StoredNodeRunStatus {
    Pending,
    Ready,
    Running,
    Succeeded,
    Failed,
    Skipped,
    WaitingForApproval,
}

impl From<NodeRunStatus> for StoredNodeRunStatus {
    fn from(value: NodeRunStatus) -> Self {
        match value {
            NodeRunStatus::Pending => Self::Pending,
            NodeRunStatus::Ready => Self::Ready,
            NodeRunStatus::Running => Self::Running,
            NodeRunStatus::Succeeded => Self::Succeeded,
            NodeRunStatus::Failed => Self::Failed,
            NodeRunStatus::Skipped => Self::Skipped,
            NodeRunStatus::WaitingForApproval => Self::WaitingForApproval,
        }
    }
}

impl From<StoredNodeRunStatus> for NodeRunStatus {
    fn from(value: StoredNodeRunStatus) -> Self {
        match value {
            StoredNodeRunStatus::Pending => Self::Pending,
            StoredNodeRunStatus::Ready => Self::Ready,
            StoredNodeRunStatus::Running => Self::Running,
            StoredNodeRunStatus::Succeeded => Self::Succeeded,
            StoredNodeRunStatus::Failed => Self::Failed,
            StoredNodeRunStatus::Skipped => Self::Skipped,
            StoredNodeRunStatus::WaitingForApproval => Self::WaitingForApproval,
        }
    }
}
