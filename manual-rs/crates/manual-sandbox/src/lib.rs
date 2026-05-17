use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

pub fn crate_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

pub fn create_sandbox(sandbox_id: String, params: &Value, now: &str) -> Value {
    // Why this exists: docs/wiki/systems/샌드박스-기능.md defines reusable
    // execution boundaries that can be applied without per-command approval.
    json!({
        "id": sandbox_id,
        "name": params.get("name").cloned().unwrap_or_else(|| json!("Docs Writer")),
        "allow_read": params.get("allow_read").cloned().unwrap_or_else(|| json!(["docs/**"])),
        "allow_write": params.get("allow_write").cloned().unwrap_or_else(|| json!(["docs/wiki/**", ".manual/tmp/**", ".manual/cache/**"])),
        "allow_commands": params.get("allow_commands").cloned().unwrap_or_else(|| json!(["scripts/**"])),
        "deny_commands": params.get("deny_commands").cloned().unwrap_or_else(|| json!(["scripts/deploy.sh"])),
        "allow_network": params.get("allow_network").cloned().unwrap_or_else(|| json!(["api.example.com"])),
        "deny_network": params.get("deny_network").cloned().unwrap_or_else(|| json!([])),
        "allow_env": params.get("allow_env").cloned().unwrap_or_else(|| json!(["MANUAL_*"])),
        "tmp_write": params.get("tmp_write").cloned().unwrap_or_else(|| json!([".manual/tmp/**"])),
        "cache_write": params.get("cache_write").cloned().unwrap_or_else(|| json!([".manual/cache/**"])),
        "scope_root": params.get("scope_root").cloned().unwrap_or_else(|| json!(null)),
        "created_at": now,
        "updated_at": now,
        "history": [{ "at": now, "change": "sandbox_created" }],
    })
}

pub fn update_sandbox(mut sandbox: Value, changes: &Value, now: &str) -> Value {
    let before = sandbox.clone();
    merge_object(&mut sandbox, changes);
    sandbox["updated_at"] = json!(now);
    let after = sandbox.clone();
    push_json_array(
        &mut sandbox["history"],
        json!({ "at": now, "change": "sandbox_updated", "before": before, "after": after }),
    );
    sandbox
}

pub fn evaluate(sandbox: &Value, operation: &str, target: &str) -> Value {
    let (allow_key, deny_key) = match operation {
        "read_file" => ("allow_read", ""),
        "write_file" => ("allow_write", ""),
        "network" => ("allow_network", "deny_network"),
        "execute" => ("allow_commands", "deny_commands"),
        "read_env" => ("allow_env", ""),
        _ => ("", ""),
    };
    let denied = !deny_key.is_empty() && matches_any(&sandbox[deny_key], target);
    let allowed = !denied && matches_any(&sandbox[allow_key], target);
    let reason = if denied {
        "explicit deny policy matched"
    } else if allowed {
        "allowed by sandbox policy"
    } else {
        "not allowed by sandbox policy"
    };

    json!({
        "allowed": allowed,
        "approval_required": false,
        "operation": operation,
        "target": target,
        "reason": reason,
        "violation": !allowed,
        "halt_node": !allowed,
        "retry_allowed_after_policy_or_input_change": !allowed,
        "allowed_tmp": sandbox["tmp_write"],
        "allowed_cache": sandbox["cache_write"],
    })
}

pub fn platform_backends() -> Value {
    json!({
        "macos": ["seatbelt", "sandbox-exec"],
        "linux": ["namespace", "seccomp", "bubblewrap", "firejail"],
        "windows": ["job-object", "appcontainer", "windows-sandbox"],
        "current": current_backend_name(),
    })
}

pub fn current_backend_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "sandbox-exec"
    } else {
        "unsupported"
    }
}

pub fn sandboxed_command(
    sandbox: &Value,
    program: impl AsRef<OsStr>,
    args: &[String],
) -> io::Result<Command> {
    let program = PathBuf::from(program.as_ref());
    if cfg!(target_os = "macos") {
        macos_sandboxed_command(sandbox, &program, args)
    } else {
        unsupported_sandbox_command(&program, args)
    }
}

pub fn run_sandboxed(
    sandbox: &Value,
    program: impl AsRef<OsStr>,
    args: &[String],
) -> io::Result<Output> {
    sandboxed_command(sandbox, program, args)?.output()
}

#[cfg(target_os = "macos")]
fn macos_sandboxed_command(
    sandbox: &Value,
    program: &Path,
    args: &[String],
) -> io::Result<Command> {
    let profile = write_macos_profile(sandbox)?;
    let mut command = Command::new("sandbox-exec");
    command.arg("-f").arg(profile);
    command.arg(program);
    command.args(args);
    Ok(command)
}

#[cfg(not(target_os = "macos"))]
fn macos_sandboxed_command(
    _sandbox: &Value,
    _program: &Path,
    _args: &[String],
) -> io::Result<Command> {
    unreachable!("macOS sandbox command should only be used on macOS")
}

fn unsupported_sandbox_command(program: &Path, args: &[String]) -> io::Result<Command> {
    let _ = (program, args);
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "no OS sandbox backend is implemented for this platform",
    ))
}

#[cfg(target_os = "macos")]
fn write_macos_profile(sandbox: &Value) -> io::Result<PathBuf> {
    let profile = macos_seatbelt_profile(sandbox);
    let mut path = std::env::temp_dir();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    path.push(format!("manual-sandbox-{unique}.sb"));
    fs::write(&path, profile)?;
    Ok(path)
}

pub fn macos_seatbelt_profile(sandbox: &Value) -> String {
    let mut profile = String::from("(version 1)\n(allow default)\n");
    let roots = sandbox_roots(sandbox);

    for root in &roots {
        profile.push_str(&format!(
            "(deny file-read* (subpath \"{}\"))\n",
            escape_sb(root)
        ));
        profile.push_str(&format!(
            "(deny file-write* (subpath \"{}\"))\n",
            escape_sb(root)
        ));
        profile.push_str(&format!(
            "(deny process-exec (subpath \"{}\"))\n",
            escape_sb(root)
        ));
    }

    for path in sandbox_paths(sandbox, "allow_read")
        .into_iter()
        .chain(sandbox_paths(sandbox, "allow_commands"))
    {
        profile.push_str(&format!(
            "(allow file-read* (subpath \"{}\"))\n",
            escape_sb(&path)
        ));
    }

    for path in sandbox_paths(sandbox, "allow_write")
        .into_iter()
        .chain(sandbox_paths(sandbox, "tmp_write"))
        .chain(sandbox_paths(sandbox, "cache_write"))
    {
        profile.push_str(&format!(
            "(allow file-write* (subpath \"{}\"))\n",
            escape_sb(&path)
        ));
    }

    for path in sandbox_paths(sandbox, "allow_commands") {
        profile.push_str(&format!(
            "(allow process-exec (subpath \"{}\"))\n",
            escape_sb(&path)
        ));
    }

    if network_denied(sandbox) {
        profile.push_str("(deny network*)\n");
    }

    profile
}

fn merge_object(target: &mut Value, changes: &Value) {
    let (Some(target), Some(changes)) = (target.as_object_mut(), changes.as_object()) else {
        return;
    };
    for (key, value) in changes {
        target.insert(key.clone(), value.clone());
    }
}

fn push_json_array(target: &mut Value, value: Value) {
    if !target.is_array() {
        *target = json!([]);
    }
    target
        .as_array_mut()
        .expect("target should be JSON array")
        .push(value);
}

fn matches_any(patterns: &Value, target: &str) -> bool {
    patterns.as_array().is_some_and(|patterns| {
        patterns
            .iter()
            .any(|pattern| pattern_matches(pattern.as_str().unwrap_or_default(), target))
    })
}

fn pattern_matches(pattern: &str, target: &str) -> bool {
    if pattern == target {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix("/**") {
        return target == prefix || target.starts_with(&format!("{prefix}/"));
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return target.starts_with(prefix);
    }
    false
}

fn sandbox_roots(sandbox: &Value) -> Vec<String> {
    let explicit = sandbox["scope_root"]
        .as_str()
        .filter(|value| !value.is_empty())
        .map(|value| vec![canonical_policy_path(value)]);
    explicit.unwrap_or_else(|| {
        let mut roots = Vec::new();
        for key in [
            "allow_read",
            "allow_write",
            "allow_commands",
            "tmp_write",
            "cache_write",
        ] {
            for path in sandbox_paths(sandbox, key) {
                if let Some(parent) = Path::new(&path).parent() {
                    roots.push(parent.display().to_string());
                }
            }
        }
        roots.sort();
        roots.dedup();
        roots
    })
}

fn sandbox_paths(sandbox: &Value, key: &str) -> Vec<String> {
    sandbox[key]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|path| !path.contains('*'))
        .map(canonical_policy_path)
        .collect()
}

fn canonical_policy_path(path: &str) -> String {
    let path = Path::new(path);
    fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string()
}

fn network_denied(sandbox: &Value) -> bool {
    sandbox["allow_network"]
        .as_array()
        .is_some_and(|hosts| hosts.is_empty())
        || sandbox["deny_network"]
            .as_array()
            .is_some_and(|hosts| hosts.iter().any(|host| host.as_str() == Some("*")))
}

fn escape_sb(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::net::TcpListener;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use serde_json::json;

    #[test]
    fn creates_reusable_sandbox_policy_with_defaults() {
        let sandbox = super::create_sandbox(
            "sandbox-1".to_owned(),
            &json!({ "name": "Docs Writer" }),
            "2026-05-17T00:00:00Z",
        );

        assert_eq!(sandbox["id"], "sandbox-1");
        assert!(
            sandbox["allow_read"]
                .as_array()
                .unwrap()
                .contains(&json!("docs/**"))
        );
        assert_eq!(sandbox["history"][0]["change"], "sandbox_created");
    }

    #[test]
    fn updates_policy_and_records_before_after_history() {
        let sandbox = super::create_sandbox(
            "sandbox-1".to_owned(),
            &json!({ "name": "Docs Writer" }),
            "2026-05-17T00:00:00Z",
        );

        let updated = super::update_sandbox(
            sandbox,
            &json!({ "allow_write": ["docs/wiki/**"] }),
            "2026-05-17T00:01:00Z",
        );

        assert_eq!(updated["allow_write"], json!(["docs/wiki/**"]));
        assert_eq!(updated["history"][1]["change"], "sandbox_updated");
        assert!(updated["history"][1]["before"].is_object());
        assert!(updated["history"][1]["after"].is_object());
    }

    #[test]
    fn deny_policy_wins_over_allow_policy() {
        let sandbox = super::create_sandbox(
            "sandbox-1".to_owned(),
            &json!({
                "allow_commands": ["scripts/**"],
                "deny_commands": ["scripts/deploy.sh"]
            }),
            "2026-05-17T00:00:00Z",
        );

        let decision = super::evaluate(&sandbox, "execute", "scripts/deploy.sh");

        assert_eq!(decision["allowed"], false);
        assert_eq!(decision["reason"], "explicit deny policy matched");
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn macos_sandbox_allows_and_blocks_real_file_operations() {
        let root = temp_dir("file-ops");
        let allowed = root.join("allowed");
        let denied = root.join("denied");
        fs::create_dir_all(&allowed).unwrap();
        fs::create_dir_all(&denied).unwrap();
        fs::write(allowed.join("read.txt"), "allowed").unwrap();
        fs::write(denied.join("read.txt"), "denied").unwrap();
        fs::write(denied.join("delete.txt"), "delete me").unwrap();
        let script = root.join("file_ops.sh");
        fs::write(
            &script,
            format!(
                "#!/bin/sh\ncat '{}'\ncat '{}'\necho changed > '{}'\nrm '{}'\n",
                allowed.join("read.txt").display(),
                denied.join("read.txt").display(),
                allowed.join("modified.txt").display(),
                denied.join("delete.txt").display()
            ),
        )
        .unwrap();
        make_executable(&script);

        let sandbox = json!({
            "scope_root": root,
            "allow_read": [allowed.join("read.txt"), script],
            "allow_write": [allowed],
            "allow_commands": [script],
            "allow_network": [],
            "deny_network": ["*"],
            "tmp_write": [],
            "cache_write": []
        });

        let output = super::run_sandboxed(&sandbox, &script, &[]).unwrap();

        assert_ne!(output.status.code(), Some(0));
        assert!(String::from_utf8_lossy(&output.stdout).contains("allowed"));
        assert!(allowed.join("modified.txt").exists());
        assert!(denied.join("delete.txt").exists());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn macos_sandbox_blocks_unlisted_script_execution() {
        let root = temp_dir("exec-ops");
        let allowed = root.join("allowed.sh");
        let denied = root.join("denied.sh");
        fs::write(&allowed, "#!/bin/sh\necho allowed\n").unwrap();
        fs::write(&denied, "#!/bin/sh\necho denied\n").unwrap();
        make_executable(&allowed);
        make_executable(&denied);
        let sandbox = json!({
            "scope_root": root,
            "allow_read": [allowed],
            "allow_write": [],
            "allow_commands": [allowed],
            "allow_network": [],
            "deny_network": ["*"],
            "tmp_write": [],
            "cache_write": []
        });

        let allowed_output = super::run_sandboxed(&sandbox, &allowed, &[]).unwrap();
        let denied_output = super::run_sandboxed(&sandbox, &denied, &[]).unwrap();

        assert_eq!(allowed_output.status.code(), Some(0));
        assert_ne!(denied_output.status.code(), Some(0));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn macos_sandbox_blocks_real_network_connection() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port().to_string();
        let nc = Path::new("/usr/bin/nc");
        assert!(nc.exists(), "network sandbox test requires /usr/bin/nc");
        let sandbox = json!({
            "allow_read": [],
            "allow_write": [],
            "allow_commands": [nc],
            "allow_network": [],
            "deny_network": ["*"],
            "tmp_write": [],
            "cache_write": []
        });
        let args = vec!["-z".to_owned(), "127.0.0.1".to_owned(), port];

        let output = super::run_sandboxed(&sandbox, nc, &args).unwrap();

        assert_ne!(output.status.code(), Some(0));
    }

    fn temp_dir(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        path.push(format!("manual-sandbox-test-{name}-{unique}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[cfg(unix)]
    fn make_executable(path: &Path) {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }
}
