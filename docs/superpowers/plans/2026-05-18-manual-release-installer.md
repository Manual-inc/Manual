# Manual Release Installer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship Manual through GitHub Releases so Unix users can install it with `curl | bash`, with release artifacts that contain both the CLI and its paired app-server binary.

**Architecture:** Rename the shipped binaries to product-specific names, make the CLI locate a colocated server binary after installation, package both binaries into release archives, and add a root `install.sh` that resolves the right archive from GitHub Releases. Keep the existing CI jobs, then add a release workflow triggered by version tags and a branch-safe validation path for the installer.

**Tech Stack:** Rust, GitHub Actions, Bash, tar.gz packaging, llm-wiki project documentation

---

### Task 1: Lock the shipped binary contract with tests

**Files:**
- Modify: `app/cli/src/main.rs`
- Modify: `app/cli/tests/cli.rs`

- [ ] **Step 1: Write the failing sibling-server lookup test**

```rust
#[test]
fn resolve_server_bin_prefers_sibling_manual_app_server() {
    let temp = TestDir::new("manual-cli-sibling-server");
    let cli = temp.path().join("manual");
    let server = temp.path().join("manual-app-server");
    fs::write(&cli, "").unwrap();
    fs::write(&server, "").unwrap();

    let resolved = resolve_server_bin_from(
        None,
        None,
        Some(&cli),
        Some(Path::new("/workspace")),
    )
    .unwrap();

    assert_eq!(resolved, server);
}
```

- [ ] **Step 2: Run the focused CLI test to verify it fails**

Run: `cargo test resolve_server_bin_prefers_sibling_manual_app_server --manifest-path app/cli/Cargo.toml`
Expected: FAIL because the helper does not exist and sibling lookup is unsupported.

- [ ] **Step 3: Update the integration tests to expect the shipped binary name**

```rust
fn manual_cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_manual"))
}
```

- [ ] **Step 4: Run the CLI test suite to verify the binary rename breaks before the manifest change**

Run: `cargo test --manifest-path app/cli/Cargo.toml`
Expected: FAIL because Cargo still builds `manual-cli`, not `manual`.

### Task 2: Implement the installed binary layout

**Files:**
- Modify: `app/cli/Cargo.toml`
- Modify: `app/cli/src/main.rs`
- Modify: `app/cli/tests/cli.rs`
- Modify: `manual-rs/crates/app-server/Cargo.toml`

- [ ] **Step 1: Declare product binary names in Cargo manifests**

```toml
[[bin]]
name = "manual"
path = "src/main.rs"
```

```toml
[[bin]]
name = "manual-app-server"
path = "src/main.rs"
```

- [ ] **Step 2: Add minimal sibling-binary resolution in the CLI**

```rust
if let Some(current_exe) = current_exe {
    if let Some(bin_dir) = current_exe.parent() {
        let sibling = bin_dir.join(server_binary_name());
        if sibling.is_file() {
            return Ok(sibling);
        }
    }
}
```

- [ ] **Step 3: Run the CLI suite to verify the renamed binary and lookup logic pass**

Run: `cargo test --manifest-path app/cli/Cargo.toml`
Expected: PASS

### Task 3: Add release packaging and installer entrypoint

**Files:**
- Create: `install.sh`
- Create: `.github/workflows/release.yml`
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Write the installer script with version/platform resolution**

```bash
release_url="${MANUAL_INSTALL_BASE_URL:-https://github.com/Manual-inc/Manual/releases/download/${version}}"
archive="manual-${platform}.tar.gz"
```

- [ ] **Step 2: Add a release workflow that builds archives containing `manual` and `manual-app-server`**

```yaml
on:
  push:
    tags:
      - "v*"
```

- [ ] **Step 3: Extend CI to validate the installer script on non-tag pushes**

Run: `bash install.sh --help`
Expected: exit 0 with usage output

### Task 4: Update wiki docs and verify end to end

**Files:**
- Create: `docs/wiki/architecture/cli-release-distribution.md`
- Modify: `docs/wiki/목차.md`
- Modify: `docs/wiki/작업-로그.md`

- [ ] **Step 1: Document the release archive and installer contract**

```md
## 배포 계약

- GitHub Release 아카이브는 `manual`과 `manual-app-server`를 함께 포함한다.
- `install.sh`는 사용자 플랫폼에 맞는 아카이브를 내려받아 `~/.local/bin`에 설치한다.
```

- [ ] **Step 2: Run verification commands**

Run: `cargo test --manifest-path app/cli/Cargo.toml`
Expected: PASS

Run: `cargo test --manifest-path manual-rs/Cargo.toml -p app-server`
Expected: PASS

Run: `cargo run --manifest-path docs/test/Cargo.toml`
Expected: `ok: no orphan documents found`

- [ ] **Step 3: Review the final release-related diff**

Run: `git diff -- app/cli/Cargo.toml app/cli/src/main.rs app/cli/tests/cli.rs manual-rs/crates/app-server/Cargo.toml .github/workflows/ci.yml .github/workflows/release.yml install.sh docs/wiki/architecture/cli-release-distribution.md docs/wiki/목차.md docs/wiki/작업-로그.md`
Expected: only the release-installer changes and linked wiki updates appear.
