use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use manual_worflow::WorkflowDefinition;

#[derive(Clone)]
pub(crate) struct WorkflowStore {
    storage_dir: PathBuf,
}

impl WorkflowStore {
    pub(crate) fn new(storage_dir: impl AsRef<Path>) -> Self {
        Self {
            storage_dir: storage_dir.as_ref().to_path_buf(),
        }
    }

    pub(crate) fn load_workflows(&self) -> BTreeMap<String, WorkflowDefinition> {
        let mut workflows = BTreeMap::new();
        let entries = match fs::read_dir(&self.storage_dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return workflows,
            Err(error) => {
                eprintln!(
                    "failed to read workflow storage directory {}: {error}",
                    self.storage_dir.display()
                );
                return workflows;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }

            match fs::read_to_string(&path)
                .ok()
                .and_then(|content| serde_json::from_str::<WorkflowDefinition>(&content).ok())
            {
                Some(workflow) => {
                    workflows.insert(workflow.id.clone(), workflow);
                }
                None => {
                    eprintln!("failed to load workflow file {}", path.display());
                }
            }
        }

        workflows
    }

    pub(crate) fn save(&self, workflow: &WorkflowDefinition) -> io::Result<()> {
        fs::create_dir_all(&self.storage_dir)?;
        let path = self.workflow_path(&workflow.id);
        let content = serde_json::to_string_pretty(workflow).map_err(io::Error::other)?;
        fs::write(path, content)
    }

    pub(crate) fn delete(&self, workflow_id: &str) -> io::Result<()> {
        let path = self.workflow_path(workflow_id);
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }

    fn workflow_path(&self, workflow_id: &str) -> PathBuf {
        self.storage_dir
            .join(format!("{}.json", hex_encode(workflow_id.as_bytes())))
    }
}

pub(crate) fn default_workflow_storage_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("MANUAL_RS_WORKFLOW_DIR") {
        return PathBuf::from(path);
    }

    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".manual-rs")
        .join("workflows")
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }

    encoded
}
