use std::collections::BTreeMap;
use std::fmt;

use job::{Job, JobId, JobIdError};

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
