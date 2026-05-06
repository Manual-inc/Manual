use script::ScriptDependency;
use script::ScriptError;
use script::ScriptRequest;
use script::ScriptRunner;

use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

fn simple_request() -> ScriptRequest {
    ScriptRequest::new(
        r#"
fn main(input_json: &str) -> String {
    input_json.to_string()
}
"#,
        "{}",
    )
}

fn temp_root(name: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after UNIX_EPOCH")
        .as_nanos();

    std::env::temp_dir().join(format!(
        "manual-script-test-{name}-{}-{timestamp}",
        std::process::id()
    ))
}

fn write_helper_package(root: &std::path::Path) -> PathBuf {
    let package = root.join("helper-package");
    fs::create_dir_all(package.join("src")).expect("helper package src should be created");
    fs::write(
        package.join("Cargo.toml"),
        r#"
[package]
name = "helper-package"
version = "0.1.0"
edition = "2024"

[lib]
name = "helper_package"
path = "src/lib.rs"
"#,
    )
    .expect("helper package manifest should be written");
    fs::write(
        package.join("src").join("lib.rs"),
        r#"
pub fn summarize(input: &str) -> String {
    format!("helper-len={}", input.len())
}
"#,
    )
    .expect("helper package lib should be written");

    package
}

#[test]
fn runs_rust_source_by_passing_json_input_to_script_main() {
    let source = r#"
fn main(input_json: &str) -> String {
    format!("{{\"received\":{input_json}}}")
}
"#;
    let request = ScriptRequest::new(source, r#"{"name":"manual","count":2}"#);

    let output = ScriptRunner::default()
        .run(&request)
        .expect("script should compile and run");

    assert!(output.success());
    assert_eq!(output.stdout, r#"{"received":{"name":"manual","count":2}}"#);
    assert_eq!(output.stderr, "");
}

#[test]
fn runs_rust_source_with_path_library_dependency() {
    let root = temp_root("path-dependency");
    fs::create_dir_all(&root).expect("custom temp root should be created");
    let helper_package = write_helper_package(&root);

    let request = ScriptRequest::new(
        r#"
fn main(input_json: &str) -> String {
    helper_package::summarize(input_json)
}
"#,
        r#"{"kind":"dependency"}"#,
    )
    .with_dependency(ScriptDependency::path("helper-package", &helper_package));

    let output = ScriptRunner::default()
        .with_temp_root(&root)
        .run(&request)
        .expect("script should compile and run with a path dependency");

    assert!(output.success());
    assert_eq!(output.stdout, "helper-len=21");
    assert_eq!(output.stderr, "");

    fs::remove_dir_all(&root).expect("custom temp root should be removed");
}

#[test]
fn custom_cargo_path_is_used_for_dependency_scripts() {
    let root = temp_root("missing-cargo");
    fs::create_dir_all(&root).expect("custom temp root should be created");
    let helper_package = write_helper_package(&root);
    let request =
        simple_request().with_dependency(ScriptDependency::path("helper-package", &helper_package));

    let error = ScriptRunner::default()
        .with_temp_root(&root)
        .with_cargo("/definitely/missing/manual-script-cargo")
        .run(&request)
        .expect_err("custom cargo path should be used for dependency scripts");

    assert!(matches!(error, ScriptError::Io(_)));
    fs::remove_dir_all(&root).expect("custom temp root should be removed");
}

#[test]
fn custom_rustc_path_is_used() {
    let error = ScriptRunner::default()
        .with_rustc("/definitely/missing/manual-script-rustc")
        .run(&simple_request())
        .expect_err("custom rustc path should be used");

    assert!(matches!(error, ScriptError::Io(_)));
}

#[test]
fn custom_temp_root_is_used() {
    let missing_root = temp_root("missing-root");
    let error = ScriptRunner::default()
        .with_temp_root(&missing_root)
        .run(&simple_request())
        .expect_err("missing custom temp root should be used");

    assert!(matches!(error, ScriptError::Io(_)));
}

#[test]
fn temporary_script_workspace_is_removed_after_run() {
    let root = temp_root("cleanup");
    fs::create_dir_all(&root).expect("custom temp root should be created");

    let output = ScriptRunner::default()
        .with_temp_root(&root)
        .run(&simple_request())
        .expect("script should run from custom temp root");

    assert!(output.success());
    assert_eq!(
        fs::read_dir(&root)
            .expect("temp root should be readable")
            .count(),
        0
    );

    fs::remove_dir_all(&root).expect("custom temp root should be removed");
}

#[test]
fn compiled_script_exposes_binary_workspace_and_input_until_dropped() {
    let root = temp_root("compile");
    fs::create_dir_all(&root).expect("custom temp root should be created");

    let compiled = ScriptRunner::default()
        .with_temp_root(&root)
        .compile(&simple_request())
        .expect("script should compile");
    let binary_path = compiled.binary_path().to_path_buf();
    let workspace_path = compiled.workspace_path().to_path_buf();

    assert!(binary_path.is_file());
    assert!(workspace_path.is_dir());
    assert_eq!(compiled.input_json(), "{}");

    drop(compiled);

    assert!(
        !workspace_path.exists(),
        "compiled script workspace should be removed when dropped"
    );
    fs::remove_dir_all(&root).expect("custom temp root should be removed");
}

#[test]
fn script_errors_have_display_messages() {
    assert_eq!(
        ScriptError::Io("spawn failed".to_string()).to_string(),
        "script process failed: spawn failed"
    );

    assert_eq!(
        ScriptError::CompileFailed {
            stdout: String::new(),
            stderr: "bad source".to_string(),
            exit_code: Some(1),
        }
        .to_string(),
        "script compilation failed with exit code Some(1): bad source"
    );
}

#[test]
fn compile_errors_are_reported_with_stderr() {
    let request = ScriptRequest::new(
        r#"
fn main(input_json: &str) -> String {
    missing_symbol(input_json)
}
"#,
        "{}",
    );

    let error = ScriptRunner::default()
        .run(&request)
        .expect_err("script should fail to compile");

    match error {
        ScriptError::CompileFailed { stderr, .. } => {
            assert!(stderr.contains("missing_symbol"));
        }
        other => panic!("expected compile failure, got {other:?}"),
    }
}

#[test]
fn runtime_failures_capture_exit_code_and_stderr() {
    let request = ScriptRequest::new(
        r#"
fn main(_input_json: &str) -> String {
    eprint!("script failed");
    std::process::exit(9);
}
"#,
        "{}",
    );

    let output = ScriptRunner::default()
        .run(&request)
        .expect("script should compile even when it exits with failure");

    assert_eq!(output.exit_code, Some(9));
    assert!(!output.success());
    assert_eq!(output.stdout, "");
    assert_eq!(output.stderr, "script failed");
}
