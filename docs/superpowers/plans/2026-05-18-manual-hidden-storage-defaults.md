# Manual Hidden Storage Defaults Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Manual store its default local discovery file and app-server state under `~/.manual` instead of platform-specific application-data directories.

**Architecture:** Keep the existing environment-variable overrides, but centralize the default path calculation around a hidden home-directory root so the CLI and app-server share the same storage convention. Cover the new defaults with focused unit tests and update the linked wiki page that explains the local app architecture.

**Tech Stack:** Rust, Cargo tests, llm-wiki project documentation

---

### Task 1: Lock the new default paths with tests

**Files:**
- Modify: `app/cli/src/main.rs`
- Modify: `manual-rs/crates/app-server/src/workflow_store.rs`

- [ ] **Step 1: Write the failing CLI discovery-path test**

```rust
#[test]
fn default_discovery_file_uses_hidden_manual_directory() {
    let path = default_discovery_file_from(None, Some(Path::new("/Users/example")));

    assert_eq!(
        path,
        PathBuf::from("/Users/example/.manual/app-server.json")
    );
}
```

- [ ] **Step 2: Run the CLI test to verify it fails**

Run: `cargo test default_discovery_file_uses_hidden_manual_directory --manifest-path app/cli/Cargo.toml`
Expected: FAIL because `default_discovery_file_from` does not exist yet and the current default points outside `~/.manual`.

- [ ] **Step 3: Write the failing app-server storage-root test**

```rust
#[test]
fn default_storage_dir_uses_hidden_manual_directory_when_home_exists() {
    let storage_dir = default_workflow_storage_dir_from(
        None,
        Path::new("/"),
        Some(Path::new("/Users/example")),
    );

    assert_eq!(storage_dir, PathBuf::from("/Users/example/.manual"));
}
```

- [ ] **Step 4: Run the app-server test to verify it fails**

Run: `cargo test default_storage_dir_uses_hidden_manual_directory_when_home_exists --manifest-path manual-rs/Cargo.toml -p app-server`
Expected: FAIL because the storage helper still points to `Application Support/Manual/workflows`.

### Task 2: Implement the new shared default root

**Files:**
- Modify: `app/cli/src/main.rs`
- Modify: `manual-rs/crates/app-server/src/workflow_store.rs`
- Update docs link comments in touched source blocks to `docs/wiki/architecture/manual-app-architecture.md`

- [ ] **Step 1: Add minimal helper logic in the CLI**

```rust
fn default_discovery_file_from(
    override_path: Option<PathBuf>,
    home_dir: Option<&Path>,
) -> PathBuf {
    if let Some(path) = override_path {
        return path;
    }

    if let Some(home) = home_dir {
        return home.join(".manual").join("app-server.json");
    }

    env::temp_dir().join("manual-app-server.json")
}
```

- [ ] **Step 2: Add minimal helper logic in the app-server storage helper**

```rust
fn default_workflow_storage_dir_from(
    override_dir: Option<&Path>,
    current_dir: &Path,
    home_dir: Option<&Path>,
) -> PathBuf {
    if let Some(path) = override_dir {
        return path.to_path_buf();
    }

    if let Some(path) = home_dir {
        return path.join(".manual");
    }

    current_dir.join(".manual")
}
```

- [ ] **Step 3: Run the focused tests to verify they pass**

Run: `cargo test default_discovery_file_uses_hidden_manual_directory --manifest-path app/cli/Cargo.toml`
Expected: PASS

Run: `cargo test default_storage_dir_uses_hidden_manual_directory_when_home_exists --manifest-path manual-rs/Cargo.toml -p app-server`
Expected: PASS

### Task 3: Update docs and run regression verification

**Files:**
- Modify: `docs/wiki/architecture/manual-app-architecture.md`
- Modify: `docs/wiki/목차.md`
- Modify: `docs/wiki/작업-로그.md`

- [ ] **Step 1: Document the hidden local storage convention**

```md
## 로컬 상태 저장

- CLI discovery 파일과 app-server 상태 저장 기본 경로는 `~/.manual/` 아래에 둔다.
- 환경 변수로 별도 경로를 지정하면 기본 경로보다 우선한다.
```

- [ ] **Step 2: Run the focused regression checks**

Run: `cargo test --manifest-path app/cli/Cargo.toml`
Expected: PASS

Run: `cargo test --manifest-path manual-rs/Cargo.toml -p app-server`
Expected: PASS

- [ ] **Step 3: Review the diff for scope**

Run: `git diff -- app/cli/src/main.rs manual-rs/crates/app-server/src/workflow_store.rs docs/wiki/architecture/manual-app-architecture.md docs/wiki/목차.md docs/wiki/작업-로그.md docs/superpowers/plans/2026-05-18-manual-hidden-storage-defaults.md`
Expected: Only the default storage-root change, related tests, and linked wiki updates appear.
