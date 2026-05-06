use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use sandbox::FilesystemAccess;
use sandbox::FilesystemEntry;
use sandbox::FilesystemMode;
use sandbox::FilesystemPolicy;
use sandbox::NetworkMode;
use sandbox::NetworkPolicy;
use sandbox::SandboxPolicy;
use sandbox::SandboxPreset;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SandboxId(String);

impl SandboxId {
    pub fn new(value: impl Into<String>) -> Result<Self, SandboxIdError> {
        let value = value.into();

        if value.trim().is_empty() {
            return Err(SandboxIdError::Empty);
        }

        if value.chars().any(char::is_whitespace) {
            return Err(SandboxIdError::ContainsWhitespace(value));
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

impl fmt::Display for SandboxId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxIdError {
    Empty,
    ContainsWhitespace(String),
}

impl fmt::Display for SandboxIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "sandbox id cannot be empty"),
            Self::ContainsWhitespace(value) => {
                write!(f, "sandbox id cannot contain whitespace: {value}")
            }
        }
    }
}

impl std::error::Error for SandboxIdError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxDefinition {
    id: SandboxId,
    policy: SandboxPolicy,
}

impl SandboxDefinition {
    pub fn new(id: impl Into<String>, policy: SandboxPolicy) -> Result<Self, SandboxRegistryError> {
        Ok(Self {
            id: SandboxId::new(id).map_err(SandboxRegistryError::InvalidId)?,
            policy,
        })
    }

    pub fn id(&self) -> &SandboxId {
        &self.id
    }

    pub fn policy(&self) -> &SandboxPolicy {
        &self.policy
    }

    pub fn into_policy(self) -> SandboxPolicy {
        self.policy
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SandboxRegistry {
    definitions: BTreeMap<SandboxId, SandboxDefinition>,
}

impl SandboxRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, definition: SandboxDefinition) -> Result<(), SandboxRegistryError> {
        if self.definitions.contains_key(definition.id()) {
            return Err(SandboxRegistryError::DuplicateId(definition.id().clone()));
        }

        self.definitions.insert(definition.id().clone(), definition);
        Ok(())
    }

    pub fn resolve(
        &self,
        id: impl Into<String>,
    ) -> Result<&SandboxDefinition, SandboxRegistryError> {
        let id = SandboxId::new(id).map_err(SandboxRegistryError::InvalidId)?;

        self.definitions
            .get(&id)
            .ok_or(SandboxRegistryError::UnknownId(id))
    }

    pub fn get(&self, id: &SandboxId) -> Option<&SandboxDefinition> {
        self.definitions.get(id)
    }

    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &SandboxDefinition> {
        self.definitions.values()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxRegistryError {
    InvalidId(SandboxIdError),
    DuplicateId(SandboxId),
    UnknownId(SandboxId),
}

impl fmt::Display for SandboxRegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId(error) => write!(f, "{error}"),
            Self::DuplicateId(id) => write!(f, "duplicate sandbox id: {id}"),
            Self::UnknownId(id) => write!(f, "unknown sandbox id: {id}"),
        }
    }
}

impl std::error::Error for SandboxRegistryError {}

pub trait SandboxStore {
    fn load(&self) -> Result<SandboxRegistry, SandboxStoreError>;

    fn save(&self, registry: &SandboxRegistry) -> Result<(), SandboxStoreError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSandboxStore {
    path: PathBuf,
}

impl FileSandboxStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn for_repository(repository_root: impl AsRef<Path>) -> Self {
        Self::new(
            repository_root
                .as_ref()
                .join(".manual")
                .join("sandboxes.toml"),
        )
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl SandboxStore for FileSandboxStore {
    fn load(&self) -> Result<SandboxRegistry, SandboxStoreError> {
        if !self.path.exists() {
            return Ok(SandboxRegistry::new());
        }

        let contents = fs::read_to_string(&self.path)
            .map_err(|error| SandboxStoreError::io(&self.path, error))?;
        let document: StoredSandboxDocument =
            toml::from_str(&contents).map_err(|error| SandboxStoreError::Decode {
                path: self.path.clone(),
                message: error.to_string(),
            })?;

        document.into_registry()
    }

    fn save(&self, registry: &SandboxRegistry) -> Result<(), SandboxStoreError> {
        let document = StoredSandboxDocument::from_registry(registry);
        let contents =
            toml::to_string_pretty(&document).map_err(|error| SandboxStoreError::Encode {
                message: error.to_string(),
            })?;

        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|error| SandboxStoreError::io(parent, error))?;
            }
        }

        fs::write(&self.path, contents).map_err(|error| SandboxStoreError::io(&self.path, error))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxStoreError {
    Io { path: PathBuf, message: String },
    Decode { path: PathBuf, message: String },
    Encode { message: String },
    Registry(SandboxRegistryError),
}

impl SandboxStoreError {
    fn io(path: &Path, error: std::io::Error) -> Self {
        Self::Io {
            path: path.to_path_buf(),
            message: error.to_string(),
        }
    }
}

impl fmt::Display for SandboxStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, message } => {
                write!(
                    f,
                    "sandbox store I/O failed at {}: {message}",
                    path.display()
                )
            }
            Self::Decode { path, message } => write!(
                f,
                "sandbox store failed to decode {}: {message}",
                path.display()
            ),
            Self::Encode { message } => write!(f, "sandbox store failed to encode: {message}"),
            Self::Registry(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for SandboxStoreError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredSandboxDocument {
    sandboxes: Vec<StoredSandboxDefinition>,
}

impl StoredSandboxDocument {
    fn from_registry(registry: &SandboxRegistry) -> Self {
        Self {
            sandboxes: registry
                .iter()
                .map(StoredSandboxDefinition::from_definition)
                .collect(),
        }
    }

    fn into_registry(self) -> Result<SandboxRegistry, SandboxStoreError> {
        let mut registry = SandboxRegistry::new();

        for definition in self.sandboxes {
            registry
                .insert(definition.into_definition()?)
                .map_err(SandboxStoreError::Registry)?;
        }

        Ok(registry)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredSandboxDefinition {
    id: String,
    policy: StoredSandboxPolicy,
}

impl StoredSandboxDefinition {
    fn from_definition(definition: &SandboxDefinition) -> Self {
        Self {
            id: definition.id().as_str().to_string(),
            policy: StoredSandboxPolicy::from_policy(definition.policy()),
        }
    }

    fn into_definition(self) -> Result<SandboxDefinition, SandboxStoreError> {
        SandboxDefinition::new(self.id, self.policy.into_policy())
            .map_err(SandboxStoreError::Registry)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredSandboxPolicy {
    preset: StoredSandboxPreset,
    filesystem: StoredFilesystemPolicy,
    network: StoredNetworkPolicy,
}

impl StoredSandboxPolicy {
    fn from_policy(policy: &SandboxPolicy) -> Self {
        Self {
            preset: policy.preset.into(),
            filesystem: StoredFilesystemPolicy::from_policy(&policy.filesystem),
            network: StoredNetworkPolicy::from_policy(&policy.network),
        }
    }

    fn into_policy(self) -> SandboxPolicy {
        SandboxPolicy {
            preset: self.preset.into(),
            filesystem: self.filesystem.into_policy(),
            network: self.network.into_policy(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum StoredSandboxPreset {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

impl From<SandboxPreset> for StoredSandboxPreset {
    fn from(value: SandboxPreset) -> Self {
        match value {
            SandboxPreset::ReadOnly => Self::ReadOnly,
            SandboxPreset::WorkspaceWrite => Self::WorkspaceWrite,
            SandboxPreset::DangerFullAccess => Self::DangerFullAccess,
        }
    }
}

impl From<StoredSandboxPreset> for SandboxPreset {
    fn from(value: StoredSandboxPreset) -> Self {
        match value {
            StoredSandboxPreset::ReadOnly => Self::ReadOnly,
            StoredSandboxPreset::WorkspaceWrite => Self::WorkspaceWrite,
            StoredSandboxPreset::DangerFullAccess => Self::DangerFullAccess,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredFilesystemPolicy {
    mode: StoredFilesystemMode,
    entries: Vec<StoredFilesystemEntry>,
}

impl StoredFilesystemPolicy {
    fn from_policy(policy: &FilesystemPolicy) -> Self {
        Self {
            mode: policy.mode.into(),
            entries: policy
                .entries
                .iter()
                .map(StoredFilesystemEntry::from_entry)
                .collect(),
        }
    }

    fn into_policy(self) -> FilesystemPolicy {
        FilesystemPolicy {
            mode: self.mode.into(),
            entries: self
                .entries
                .into_iter()
                .map(StoredFilesystemEntry::into_entry)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum StoredFilesystemMode {
    Restricted,
    Unrestricted,
    External,
}

impl From<FilesystemMode> for StoredFilesystemMode {
    fn from(value: FilesystemMode) -> Self {
        match value {
            FilesystemMode::Restricted => Self::Restricted,
            FilesystemMode::Unrestricted => Self::Unrestricted,
            FilesystemMode::External => Self::External,
        }
    }
}

impl From<StoredFilesystemMode> for FilesystemMode {
    fn from(value: StoredFilesystemMode) -> Self {
        match value {
            StoredFilesystemMode::Restricted => Self::Restricted,
            StoredFilesystemMode::Unrestricted => Self::Unrestricted,
            StoredFilesystemMode::External => Self::External,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredFilesystemEntry {
    path: PathBuf,
    access: StoredFilesystemAccess,
}

impl StoredFilesystemEntry {
    fn from_entry(entry: &FilesystemEntry) -> Self {
        Self {
            path: entry.path.clone(),
            access: entry.access.into(),
        }
    }

    fn into_entry(self) -> FilesystemEntry {
        FilesystemEntry {
            path: self.path,
            access: self.access.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum StoredFilesystemAccess {
    Read,
    Write,
    None,
}

impl From<FilesystemAccess> for StoredFilesystemAccess {
    fn from(value: FilesystemAccess) -> Self {
        match value {
            FilesystemAccess::Read => Self::Read,
            FilesystemAccess::Write => Self::Write,
            FilesystemAccess::None => Self::None,
        }
    }
}

impl From<StoredFilesystemAccess> for FilesystemAccess {
    fn from(value: StoredFilesystemAccess) -> Self {
        match value {
            StoredFilesystemAccess::Read => Self::Read,
            StoredFilesystemAccess::Write => Self::Write,
            StoredFilesystemAccess::None => Self::None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredNetworkPolicy {
    mode: StoredNetworkMode,
}

impl StoredNetworkPolicy {
    fn from_policy(policy: &NetworkPolicy) -> Self {
        Self {
            mode: policy.mode.into(),
        }
    }

    fn into_policy(self) -> NetworkPolicy {
        NetworkPolicy {
            mode: self.mode.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum StoredNetworkMode {
    Restricted,
    Enabled,
}

impl From<NetworkMode> for StoredNetworkMode {
    fn from(value: NetworkMode) -> Self {
        match value {
            NetworkMode::Restricted => Self::Restricted,
            NetworkMode::Enabled => Self::Enabled,
        }
    }
}

impl From<StoredNetworkMode> for NetworkMode {
    fn from(value: StoredNetworkMode) -> Self {
        match value {
            StoredNetworkMode::Restricted => Self::Restricted,
            StoredNetworkMode::Enabled => Self::Enabled,
        }
    }
}
