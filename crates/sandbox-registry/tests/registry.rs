use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use sandbox::SandboxPolicy;
use sandbox_registry::FileSandboxStore;
use sandbox_registry::SandboxDefinition;
use sandbox_registry::SandboxId;
use sandbox_registry::SandboxRegistry;
use sandbox_registry::SandboxRegistryError;
use sandbox_registry::SandboxStore;

#[test]
fn registry_resolves_named_sandbox_definition() {
    let policy = SandboxPolicy::read_only("/workspace");
    let definition = SandboxDefinition::new("readonly", policy.clone()).unwrap();
    let mut registry = SandboxRegistry::new();

    registry.insert(definition).unwrap();

    let resolved = registry.resolve("readonly").unwrap();
    assert_eq!(resolved.id().as_str(), "readonly");
    assert_eq!(resolved.policy(), &policy);
}

#[test]
fn registry_rejects_duplicate_ids() {
    let definition =
        SandboxDefinition::new("workspace-write", SandboxPolicy::workspace_write(".")).unwrap();
    let duplicate =
        SandboxDefinition::new("workspace-write", SandboxPolicy::read_only(".")).unwrap();
    let mut registry = SandboxRegistry::new();

    registry.insert(definition).unwrap();

    let error = registry.insert(duplicate).unwrap_err();
    assert!(matches!(
        error,
        SandboxRegistryError::DuplicateId(id) if id.as_str() == "workspace-write"
    ));
}

#[test]
fn registry_reports_unknown_sandbox_ids() {
    let registry = SandboxRegistry::new();

    let error = registry.resolve("missing").unwrap_err();

    assert!(matches!(
        error,
        SandboxRegistryError::UnknownId(id) if id.as_str() == "missing"
    ));
}

#[test]
fn registry_updates_existing_sandbox_definition() {
    let mut registry = SandboxRegistry::new();
    registry
        .insert(SandboxDefinition::new("readonly", SandboxPolicy::read_only("/workspace")).unwrap())
        .unwrap();

    registry
        .update(
            "readonly",
            SandboxDefinition::new(
                "workspace-write",
                SandboxPolicy::workspace_write("/workspace"),
            )
            .unwrap(),
        )
        .unwrap();

    assert!(matches!(
        registry.resolve("readonly").unwrap_err(),
        SandboxRegistryError::UnknownId(id) if id.as_str() == "readonly"
    ));
    assert_eq!(
        registry.resolve("workspace-write").unwrap().policy(),
        &SandboxPolicy::workspace_write("/workspace")
    );
}

#[test]
fn registry_update_rejects_renames_to_existing_ids() {
    let mut registry = SandboxRegistry::new();
    registry
        .insert(SandboxDefinition::new("readonly", SandboxPolicy::read_only("/workspace")).unwrap())
        .unwrap();
    registry
        .insert(
            SandboxDefinition::new(
                "workspace-write",
                SandboxPolicy::workspace_write("/workspace"),
            )
            .unwrap(),
        )
        .unwrap();

    let error = registry
        .update(
            "readonly",
            SandboxDefinition::new("workspace-write", SandboxPolicy::danger_full_access()).unwrap(),
        )
        .unwrap_err();

    assert!(matches!(
        error,
        SandboxRegistryError::DuplicateId(id) if id.as_str() == "workspace-write"
    ));
    assert_eq!(
        registry.resolve("readonly").unwrap().policy(),
        &SandboxPolicy::read_only("/workspace")
    );
}

#[test]
fn registry_removes_existing_sandbox_definition() {
    let mut registry = SandboxRegistry::new();
    registry
        .insert(SandboxDefinition::new("readonly", SandboxPolicy::read_only("/workspace")).unwrap())
        .unwrap();

    let removed = registry.remove("readonly").unwrap();

    assert_eq!(removed.id().as_str(), "readonly");
    assert!(registry.is_empty());
}

#[test]
fn sandbox_id_rejects_empty_or_whitespace_values() {
    assert!(SandboxId::new("").is_err());
    assert!(SandboxId::new("   ").is_err());
    assert!(SandboxId::new("read only").is_err());
}

#[test]
fn file_store_reloads_saved_registry_from_disk() {
    let temp_dir = unique_temp_dir("reloads-saved-registry");
    let store_path = temp_dir.join(".manual").join("sandboxes.toml");
    let store = FileSandboxStore::new(&store_path);
    let mut registry = SandboxRegistry::new();
    let read_only = SandboxPolicy::read_only("/workspace");
    let mut networked_write = SandboxPolicy::workspace_write("/workspace");
    networked_write.network.mode = sandbox::NetworkMode::Enabled;

    registry
        .insert(SandboxDefinition::new("read-only", read_only.clone()).unwrap())
        .unwrap();
    registry
        .insert(SandboxDefinition::new("networked-write", networked_write.clone()).unwrap())
        .unwrap();

    store.save(&registry).unwrap();

    let reloaded_store = FileSandboxStore::new(&store_path);
    let reloaded = reloaded_store.load().unwrap();

    assert_eq!(reloaded.resolve("read-only").unwrap().policy(), &read_only);
    assert_eq!(
        reloaded.resolve("networked-write").unwrap().policy(),
        &networked_write
    );

    fs::remove_dir_all(temp_dir).unwrap();
}

#[test]
fn file_store_loads_missing_file_as_empty_registry() {
    let temp_dir = unique_temp_dir("loads-missing-file");
    let store = FileSandboxStore::new(temp_dir.join(".manual").join("sandboxes.toml"));

    let registry = store.load().unwrap();

    assert!(registry.is_empty());

    fs::remove_dir_all(temp_dir).unwrap();
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before UNIX_EPOCH")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "manual-sandbox-registry-{name}-{timestamp}-{}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}
