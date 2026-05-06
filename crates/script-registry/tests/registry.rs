use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use script::{ScriptDependency, ScriptDependencySource, ScriptRunner};
use script_registry::{
    FileScriptStore, ScriptDefinition, ScriptId, ScriptRegistry, ScriptRegistryError, ScriptStore,
};

#[test]
fn registry_resolves_script_by_id_and_builds_requests() {
    let script = ScriptDefinition::new(
        "echo-json",
        r#"
fn main(input_json: &str) -> String {
    input_json.to_string()
}
"#,
    )
    .expect("script definition should be valid")
    .with_dependency(ScriptDependency::version("serde_json", "1"));
    let mut registry = ScriptRegistry::new();

    registry
        .insert(script.clone())
        .expect("script should be inserted");

    let resolved = registry
        .resolve("echo-json")
        .expect("script should resolve");
    let request = resolved.request(r#"{"ticket":"VOC-1"}"#);

    assert_eq!(resolved, &script);
    assert_eq!(request.rust_source, script.rust_source());
    assert_eq!(request.input_json, r#"{"ticket":"VOC-1"}"#);
    assert_eq!(request.dependencies.as_slice(), script.dependencies());
}

#[test]
fn registry_rejects_duplicate_script_ids() {
    let script = ScriptDefinition::new("summarize", "fn main(_: &str) -> String { String::new() }")
        .expect("script definition should be valid");
    let duplicate =
        ScriptDefinition::new("summarize", "fn main(_: &str) -> String { \"v2\".into() }")
            .expect("duplicate definition should be valid before insert");
    let mut registry = ScriptRegistry::new();

    registry.insert(script).expect("first script should insert");

    let error = registry
        .insert(duplicate)
        .expect_err("registry should reject duplicate script ids");

    assert!(matches!(
        error,
        ScriptRegistryError::DuplicateId(ref id) if id.as_str() == "summarize"
    ));
    assert_eq!(error.to_string(), "duplicate script id: summarize");
}

#[test]
fn registry_reports_unknown_script_ids() {
    let registry = ScriptRegistry::new();

    let error = registry
        .resolve("missing")
        .expect_err("registry should report missing scripts");

    assert!(matches!(
        error,
        ScriptRegistryError::UnknownId(ref id) if id.as_str() == "missing"
    ));
    assert_eq!(error.to_string(), "unknown script id: missing");
}

#[test]
fn script_id_rejects_empty_or_whitespace_values() {
    assert!(ScriptId::new("").is_err());
    assert!(ScriptId::new("   ").is_err());
    assert!(ScriptId::new("has whitespace").is_err());
}

#[test]
fn registry_iterates_scripts_by_id_order() {
    let mut registry = ScriptRegistry::new();

    registry
        .insert(
            ScriptDefinition::new(
                "release-notes",
                "fn main(_: &str) -> String { \"\".into() }",
            )
            .unwrap(),
        )
        .unwrap();
    registry
        .insert(
            ScriptDefinition::new("debug-voc", "fn main(_: &str) -> String { \"\".into() }")
                .unwrap(),
        )
        .unwrap();

    let ids = registry
        .iter()
        .map(|script| script.id().as_str())
        .collect::<Vec<_>>();

    assert_eq!(ids, ["debug-voc", "release-notes"]);
}

#[test]
fn registry_updates_and_removes_scripts() {
    let mut registry = ScriptRegistry::new();
    let id = ScriptId::new("formatter").expect("script id should be valid");

    registry
        .insert(ScriptDefinition::new("formatter", "old").unwrap())
        .expect("script should insert");

    let script = registry
        .get_mut(&id)
        .expect("script should be available for mutation");
    script.set_rust_source("new");
    script.set_dependencies([ScriptDependency::manifest_value(
        "regex",
        r#"{ version = "1", default-features = false }"#,
    )]);

    let updated = registry.resolve("formatter").unwrap();
    assert_eq!(updated.rust_source(), "new");
    assert_eq!(updated.dependencies().len(), 1);

    let removed = registry.remove(&id).expect("script should be removed");

    assert_eq!(removed.id().as_str(), "formatter");
    assert!(registry.is_empty());
}

#[test]
fn file_store_reloads_saved_registry_from_disk_and_runs_loaded_script() {
    let temp_dir = unique_temp_dir("reloads-saved-registry");
    let store_path = temp_dir.join(".manual").join("scripts.toml");
    let store = FileScriptStore::new(&store_path);
    let mut registry = ScriptRegistry::new();

    registry
        .insert(
            ScriptDefinition::new(
                "echo-json",
                r#"
fn main(input_json: &str) -> String {
    format!("registered:{input_json}")
}
"#,
            )
            .unwrap(),
        )
        .unwrap();

    store.save(&registry).expect("registry should save");

    let reloaded = FileScriptStore::new(&store_path)
        .load()
        .expect("registry should reload");
    let script = reloaded
        .resolve("echo-json")
        .expect("saved script should resolve");
    let output = ScriptRunner::default()
        .run(&script.request(r#"{"ok":true}"#))
        .expect("loaded script should run");

    assert!(output.success());
    assert_eq!(output.stdout, r#"registered:{"ok":true}"#);

    fs::remove_dir_all(temp_dir).unwrap();
}

#[test]
fn file_store_roundtrips_dependency_sources() {
    let temp_dir = unique_temp_dir("roundtrips-dependency-sources");
    let store_path = temp_dir.join(".manual").join("scripts.toml");
    let store = FileScriptStore::new(&store_path);
    let mut registry = ScriptRegistry::new();

    registry
        .insert(
            ScriptDefinition::new(
                "dependencies",
                "fn main(_: &str) -> String { String::new() }",
            )
            .unwrap()
            .with_dependencies([
                ScriptDependency::version("serde_json", "1"),
                ScriptDependency::path("helper-package", "/workspace/helper"),
                ScriptDependency::manifest_value(
                    "regex",
                    r#"{ version = "1", default-features = false }"#,
                ),
            ]),
        )
        .unwrap();

    store.save(&registry).unwrap();
    let reloaded = FileScriptStore::new(&store_path).load().unwrap();
    let dependencies = reloaded.resolve("dependencies").unwrap().dependencies();

    assert_eq!(dependencies.len(), 3);
    assert!(matches!(
        &dependencies[0].source,
        ScriptDependencySource::Version(version) if version == "1"
    ));
    assert!(matches!(
        &dependencies[1].source,
        ScriptDependencySource::Path(path) if path == &PathBuf::from("/workspace/helper")
    ));
    assert!(matches!(
        &dependencies[2].source,
        ScriptDependencySource::ManifestValue(value) if value.contains("default-features = false")
    ));

    fs::remove_dir_all(temp_dir).unwrap();
}

#[test]
fn file_store_loads_missing_file_as_empty_registry() {
    let temp_dir = unique_temp_dir("loads-missing-file");
    let store = FileScriptStore::new(temp_dir.join(".manual").join("scripts.toml"));

    let registry = store.load().expect("missing file should load as empty");

    assert!(registry.is_empty());

    fs::remove_dir_all(temp_dir).unwrap();
}

#[test]
fn file_store_for_repository_uses_manual_scripts_file() {
    let store = FileScriptStore::for_repository("/workspace/manual");

    assert_eq!(
        store.path(),
        std::path::Path::new("/workspace/manual")
            .join(".manual")
            .join("scripts.toml")
    );
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after UNIX_EPOCH")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "manual-script-registry-{name}-{timestamp}-{}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}
