use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use script::{ScriptDependency, ScriptDependencySource, ScriptRequest};
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScriptId(String);

impl ScriptId {
    pub fn new(value: impl Into<String>) -> Result<Self, ScriptIdError> {
        let value = value.into();

        if value.trim().is_empty() {
            return Err(ScriptIdError::Empty);
        }

        if value.chars().any(char::is_whitespace) {
            return Err(ScriptIdError::ContainsWhitespace(value));
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

impl fmt::Display for ScriptId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptIdError {
    Empty,
    ContainsWhitespace(String),
}

impl fmt::Display for ScriptIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "script id cannot be empty"),
            Self::ContainsWhitespace(value) => {
                write!(f, "script id cannot contain whitespace: {value}")
            }
        }
    }
}

impl std::error::Error for ScriptIdError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptDefinition {
    id: ScriptId,
    rust_source: String,
    dependencies: Vec<ScriptDependency>,
}

impl ScriptDefinition {
    pub fn new(
        id: impl Into<String>,
        rust_source: impl Into<String>,
    ) -> Result<Self, ScriptRegistryError> {
        Ok(Self {
            id: ScriptId::new(id).map_err(ScriptRegistryError::InvalidId)?,
            rust_source: rust_source.into(),
            dependencies: Vec::new(),
        })
    }

    pub fn id(&self) -> &ScriptId {
        &self.id
    }

    pub fn rust_source(&self) -> &str {
        &self.rust_source
    }

    pub fn dependencies(&self) -> &[ScriptDependency] {
        &self.dependencies
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

    pub fn set_rust_source(&mut self, rust_source: impl Into<String>) {
        self.rust_source = rust_source.into();
    }

    pub fn set_dependencies<I>(&mut self, dependencies: I)
    where
        I: IntoIterator<Item = ScriptDependency>,
    {
        self.dependencies = dependencies.into_iter().collect();
    }

    pub fn request(&self, input_json: impl Into<String>) -> ScriptRequest {
        ScriptRequest::new(&self.rust_source, input_json)
            .with_dependencies(self.dependencies.iter().cloned())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScriptRegistry {
    scripts: BTreeMap<ScriptId, ScriptDefinition>,
}

impl ScriptRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, script: ScriptDefinition) -> Result<(), ScriptRegistryError> {
        if self.scripts.contains_key(script.id()) {
            return Err(ScriptRegistryError::DuplicateId(script.id().clone()));
        }

        self.scripts.insert(script.id().clone(), script);
        Ok(())
    }

    pub fn resolve(&self, id: impl Into<String>) -> Result<&ScriptDefinition, ScriptRegistryError> {
        let id = ScriptId::new(id).map_err(ScriptRegistryError::InvalidId)?;

        self.scripts
            .get(&id)
            .ok_or(ScriptRegistryError::UnknownId(id))
    }

    pub fn get(&self, id: &ScriptId) -> Option<&ScriptDefinition> {
        self.scripts.get(id)
    }

    pub fn get_mut(&mut self, id: &ScriptId) -> Option<&mut ScriptDefinition> {
        self.scripts.get_mut(id)
    }

    pub fn remove(&mut self, id: &ScriptId) -> Option<ScriptDefinition> {
        self.scripts.remove(id)
    }

    pub fn len(&self) -> usize {
        self.scripts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.scripts.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &ScriptDefinition> {
        self.scripts.values()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptRegistryError {
    InvalidId(ScriptIdError),
    DuplicateId(ScriptId),
    UnknownId(ScriptId),
}

impl fmt::Display for ScriptRegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId(error) => write!(f, "{error}"),
            Self::DuplicateId(id) => write!(f, "duplicate script id: {id}"),
            Self::UnknownId(id) => write!(f, "unknown script id: {id}"),
        }
    }
}

impl std::error::Error for ScriptRegistryError {}

pub trait ScriptStore {
    fn load(&self) -> Result<ScriptRegistry, ScriptStoreError>;

    fn save(&self, registry: &ScriptRegistry) -> Result<(), ScriptStoreError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileScriptStore {
    path: PathBuf,
}

impl FileScriptStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn for_repository(repository_root: impl AsRef<Path>) -> Self {
        Self::new(
            repository_root
                .as_ref()
                .join(".manual")
                .join("scripts.toml"),
        )
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl ScriptStore for FileScriptStore {
    fn load(&self) -> Result<ScriptRegistry, ScriptStoreError> {
        if !self.path.exists() {
            return Ok(ScriptRegistry::new());
        }

        let contents = fs::read_to_string(&self.path)
            .map_err(|error| ScriptStoreError::io(&self.path, error))?;
        let document: StoredScriptDocument =
            toml::from_str(&contents).map_err(|error| ScriptStoreError::Decode {
                path: self.path.clone(),
                message: error.to_string(),
            })?;

        document.into_registry()
    }

    fn save(&self, registry: &ScriptRegistry) -> Result<(), ScriptStoreError> {
        let document = StoredScriptDocument::from_registry(registry);
        let contents =
            toml::to_string_pretty(&document).map_err(|error| ScriptStoreError::Encode {
                message: error.to_string(),
            })?;

        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|error| ScriptStoreError::io(parent, error))?;
            }
        }

        fs::write(&self.path, contents).map_err(|error| ScriptStoreError::io(&self.path, error))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptStoreError {
    Io { path: PathBuf, message: String },
    Decode { path: PathBuf, message: String },
    Encode { message: String },
    Registry(ScriptRegistryError),
}

impl ScriptStoreError {
    fn io(path: &Path, error: std::io::Error) -> Self {
        Self::Io {
            path: path.to_path_buf(),
            message: error.to_string(),
        }
    }
}

impl fmt::Display for ScriptStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, message } => {
                write!(
                    f,
                    "script store I/O failed at {}: {message}",
                    path.display()
                )
            }
            Self::Decode { path, message } => write!(
                f,
                "script store failed to decode {}: {message}",
                path.display()
            ),
            Self::Encode { message } => write!(f, "script store failed to encode: {message}"),
            Self::Registry(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ScriptStoreError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredScriptDocument {
    scripts: Vec<StoredScriptDefinition>,
}

impl StoredScriptDocument {
    fn from_registry(registry: &ScriptRegistry) -> Self {
        Self {
            scripts: registry
                .iter()
                .map(StoredScriptDefinition::from_definition)
                .collect(),
        }
    }

    fn into_registry(self) -> Result<ScriptRegistry, ScriptStoreError> {
        let mut registry = ScriptRegistry::new();

        for script in self.scripts {
            registry
                .insert(script.into_definition()?)
                .map_err(ScriptStoreError::Registry)?;
        }

        Ok(registry)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredScriptDefinition {
    id: String,
    rust_source: String,
    dependencies: Vec<StoredScriptDependency>,
}

impl StoredScriptDefinition {
    fn from_definition(script: &ScriptDefinition) -> Self {
        Self {
            id: script.id().as_str().to_string(),
            rust_source: script.rust_source().to_string(),
            dependencies: script
                .dependencies()
                .iter()
                .map(StoredScriptDependency::from_dependency)
                .collect(),
        }
    }

    fn into_definition(self) -> Result<ScriptDefinition, ScriptStoreError> {
        let dependencies: Vec<_> = self
            .dependencies
            .into_iter()
            .map(StoredScriptDependency::into_dependency)
            .collect();

        ScriptDefinition::new(self.id, self.rust_source)
            .map(|script| script.with_dependencies(dependencies))
            .map_err(ScriptStoreError::Registry)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredScriptDependency {
    name: String,
    #[serde(flatten)]
    source: StoredScriptDependencySource,
}

impl StoredScriptDependency {
    fn from_dependency(dependency: &ScriptDependency) -> Self {
        Self {
            name: dependency.name.clone(),
            source: StoredScriptDependencySource::from_source(&dependency.source),
        }
    }

    fn into_dependency(self) -> ScriptDependency {
        ScriptDependency {
            name: self.name,
            source: self.source.into_source(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "kebab-case")]
enum StoredScriptDependencySource {
    Version { version: String },
    Path { path: PathBuf },
    ManifestValue { value: String },
}

impl StoredScriptDependencySource {
    fn from_source(source: &ScriptDependencySource) -> Self {
        match source {
            ScriptDependencySource::Version(version) => Self::Version {
                version: version.clone(),
            },
            ScriptDependencySource::Path(path) => Self::Path { path: path.clone() },
            ScriptDependencySource::ManifestValue(value) => Self::ManifestValue {
                value: value.clone(),
            },
        }
    }

    fn into_source(self) -> ScriptDependencySource {
        match self {
            Self::Version { version } => ScriptDependencySource::Version(version),
            Self::Path { path } => ScriptDependencySource::Path(path),
            Self::ManifestValue { value } => ScriptDependencySource::ManifestValue(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_definition_builds_request_with_dependencies() {
        let script =
            ScriptDefinition::new("echo", "fn main(input: &str) -> String { input.into() }")
                .unwrap()
                .with_dependency(ScriptDependency::version("serde_json", "1"));

        let request = script.request("{}");

        assert_eq!(request.rust_source, script.rust_source());
        assert_eq!(request.input_json, "{}");
        assert_eq!(request.dependencies, script.dependencies());
    }
}
