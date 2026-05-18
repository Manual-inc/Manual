use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use manual_worflow::{WorkflowDefinition, WorkflowRun};

use manual_node::{NodeRun, NodeTemplate};

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
        load_json_map(
            &self.workflows_dir(),
            "workflow",
            |workflow: &WorkflowDefinition| workflow.id.clone(),
        )
    }

    pub(crate) fn save_workflow(&self, workflow: &WorkflowDefinition) -> io::Result<()> {
        self.save_json(&self.workflow_path(&workflow.id), workflow)
    }

    pub(crate) fn delete_workflow(&self, workflow_id: &str) -> io::Result<()> {
        delete_file(self.workflow_path(workflow_id))
    }

    pub(crate) fn load_runs(&self) -> BTreeMap<String, WorkflowRun> {
        let mut runs = BTreeMap::new();
        let entries = match fs::read_dir(self.runs_dir()) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return runs,
            Err(error) => {
                eprintln!(
                    "failed to read workflow run storage directory {}: {error}",
                    self.runs_dir().display()
                );
                return runs;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }

            let Some(run_id) = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .and_then(hex_decode)
            else {
                continue;
            };

            match fs::read_to_string(&path)
                .ok()
                .and_then(|content| serde_json::from_str::<WorkflowRun>(&content).ok())
            {
                Some(run) => {
                    runs.insert(run_id, run);
                }
                None => {
                    eprintln!("failed to load workflow run file {}", path.display());
                }
            }
        }

        runs
    }

    pub(crate) fn load_run(&self, run_id: &str) -> Option<WorkflowRun> {
        fs::read_to_string(self.run_path(run_id))
            .ok()
            .and_then(|content| serde_json::from_str::<WorkflowRun>(&content).ok())
    }

    pub(crate) fn save_run(&self, run_id: &str, run: &WorkflowRun) -> io::Result<()> {
        self.save_json(&self.run_path(run_id), run)
    }

    pub(crate) fn load_node_templates(&self) -> BTreeMap<String, NodeTemplate> {
        load_json_map(
            &self.node_templates_dir(),
            "node template",
            |template: &NodeTemplate| template.id.clone(),
        )
    }

    pub(crate) fn save_node_template(&self, template: &NodeTemplate) -> io::Result<()> {
        self.save_json(&self.node_template_path(&template.id), template)
    }

    pub(crate) fn delete_node_template(&self, template_id: &str) -> io::Result<()> {
        delete_file(self.node_template_path(template_id))
    }

    pub(crate) fn load_node_runs(&self) -> BTreeMap<String, NodeRun> {
        let mut runs = BTreeMap::new();
        let entries = match fs::read_dir(self.node_runs_dir()) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return runs,
            Err(error) => {
                eprintln!(
                    "failed to read node run storage directory {}: {error}",
                    self.node_runs_dir().display()
                );
                return runs;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let Some(run_id) = path
                .file_stem()
                .and_then(|s| s.to_str())
                .and_then(hex_decode)
            else {
                continue;
            };
            match fs::read_to_string(&path)
                .ok()
                .and_then(|c| serde_json::from_str::<NodeRun>(&c).ok())
            {
                Some(run) => {
                    runs.insert(run_id, run);
                }
                None => {
                    eprintln!("failed to load node run file {}", path.display());
                }
            }
        }

        runs
    }

    pub(crate) fn load_node_run(&self, run_id: &str) -> Option<NodeRun> {
        fs::read_to_string(self.node_run_path(run_id))
            .ok()
            .and_then(|c| serde_json::from_str::<NodeRun>(&c).ok())
    }

    pub(crate) fn save_node_run(&self, run_id: &str, run: &NodeRun) -> io::Result<()> {
        self.save_json(&self.node_run_path(run_id), run)
    }

    pub(crate) fn load_values(&self, namespace: &str) -> BTreeMap<String, serde_json::Value> {
        // Why this exists: docs/wiki/architecture/manual-app-architecture.md keeps
        // app-server state file-backed so local clients share the same records.
        load_json_map(
            &self.storage_dir.join(namespace),
            namespace,
            |value: &serde_json::Value| {
                value["id"]
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| "unknown".to_owned())
            },
        )
    }

    pub(crate) fn save_value(
        &self,
        namespace: &str,
        id: &str,
        value: &serde_json::Value,
    ) -> io::Result<()> {
        self.save_json(
            &self.storage_dir.join(namespace).join(encoded_json_file(id)),
            value,
        )
    }

    fn save_json<T: serde::Serialize>(&self, path: &Path, value: &T) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(value).map_err(io::Error::other)?;
        let temporary_path = path.with_extension("json.tmp");
        fs::write(&temporary_path, content)?;
        fs::rename(temporary_path, path)
    }

    fn workflow_path(&self, workflow_id: &str) -> PathBuf {
        self.workflows_dir().join(encoded_json_file(workflow_id))
    }

    fn run_path(&self, run_id: &str) -> PathBuf {
        self.runs_dir().join(encoded_json_file(run_id))
    }

    fn workflows_dir(&self) -> PathBuf {
        self.storage_dir.join("workflows")
    }

    fn runs_dir(&self) -> PathBuf {
        self.storage_dir.join("runs")
    }

    fn node_templates_dir(&self) -> PathBuf {
        self.storage_dir.join("nodes")
    }

    fn node_runs_dir(&self) -> PathBuf {
        self.storage_dir.join("node_runs")
    }

    fn node_template_path(&self, template_id: &str) -> PathBuf {
        self.node_templates_dir()
            .join(encoded_json_file(template_id))
    }

    fn node_run_path(&self, run_id: &str) -> PathBuf {
        self.node_runs_dir().join(encoded_json_file(run_id))
    }
}

fn encoded_json_file(id: &str) -> String {
    format!("{}.json", hex_encode(id.as_bytes()))
}

pub(crate) fn default_workflow_storage_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("MANUAL_RS_WORKFLOW_DIR") {
        return default_workflow_storage_dir_from(
            Some(Path::new(&path)),
            &std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            std::env::var_os("HOME").as_deref().map(Path::new),
        );
    }

    default_workflow_storage_dir_from(
        None,
        &std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        std::env::var_os("HOME").as_deref().map(Path::new),
    )
}

fn default_workflow_storage_dir_from(
    override_dir: Option<&Path>,
    current_dir: &Path,
    home_dir: Option<&Path>,
) -> PathBuf {
    if let Some(path) = override_dir {
        return path.to_path_buf();
    }

    // Why this exists: docs/wiki/architecture/manual-app-architecture.md documents
    // that app-server state lives under the shared hidden `~/.manual` root by default.
    if let Some(path) = home_dir {
        return path.join(".manual");
    }

    current_dir.join(".manual")
}

fn load_json_map<T>(
    storage_dir: &Path,
    label: &str,
    key_for_value: impl Fn(&T) -> String,
) -> BTreeMap<String, T>
where
    T: serde::de::DeserializeOwned,
{
    let mut values = BTreeMap::new();
    let entries = match fs::read_dir(storage_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return values,
        Err(error) => {
            eprintln!(
                "failed to read {label} storage directory {}: {error}",
                storage_dir.display()
            );
            return values;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }

        match fs::read_to_string(&path)
            .ok()
            .and_then(|content| serde_json::from_str::<T>(&content).ok())
        {
            Some(value) => {
                let key = key_for_value(&value);
                if !key.is_empty() {
                    values.insert(key, value);
                }
            }
            None => {
                eprintln!("failed to load {label} file {}", path.display());
            }
        }
    }

    values
}

fn delete_file(path: PathBuf) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
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

fn hex_decode(encoded: &str) -> Option<String> {
    if !encoded.len().is_multiple_of(2) {
        return None;
    }

    let mut bytes = Vec::with_capacity(encoded.len() / 2);
    for chunk in encoded.as_bytes().chunks_exact(2) {
        let high = hex_value(chunk[0])?;
        let low = hex_value(chunk[1])?;
        bytes.push((high << 4) | low);
    }

    String::from_utf8(bytes).ok()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_storage_dir_uses_hidden_manual_directory_when_home_exists() {
        let storage_dir = default_workflow_storage_dir_from(
            None,
            Path::new("/"),
            Some(Path::new("/Users/example")),
        );

        assert_eq!(storage_dir, PathBuf::from("/Users/example/.manual"));
    }

    #[test]
    fn default_storage_dir_honors_env_override_when_present() {
        let storage_dir = default_workflow_storage_dir_from(
            Some(Path::new("/tmp/manual-state")),
            Path::new("/"),
            Some(Path::new("/Users/example")),
        );

        assert_eq!(storage_dir, PathBuf::from("/tmp/manual-state"));
    }
}
