# Sandbox

Manual treats sandboxing as a core feature, not as an optional hardening detail.

The product model is simple: an agent becomes safe enough to run as a workflow node only when it is wrapped in an explicit execution boundary. That boundary should define filesystem access, network access, process behavior, environment variables, working directory, and timeout.

## Why Sandbox Is A Core Feature

Agents can inspect files, run tools, modify code, call networks, and spawn processes. That power is useful only if the workflow declares where the agent is allowed to move.

Manual's sandbox feature should answer these questions:

- Which paths can this node read?
- Which paths can this node write?
- Is network access blocked, enabled, or restricted?
- Which metadata directories should stay protected?
- What command actually ran?
- What stdout, stderr, exit code, and timeout result came back?

## OS-Native Direction

The wiki research points toward an independent Rust sandbox package that can be used by Manual and by other projects. The first workspace version lives in `crates/sandbox`.

| OS | Likely Backend | Notes |
| --- | --- | --- |
| macOS | Seatbelt through `sandbox-exec` | Compile a policy into SBPL and wrap the command. |
| Linux | bubblewrap, seccomp, `no_new_privs`, and possibly Landlock | Build a filesystem namespace, then reduce network and process syscall surface. |
| Windows | restricted token, capability SID, ACLs, setup helper, and `CreateProcessAsUserW` | Requires setup and refresh behavior, not only one spawn call. |

The common API hides platform details while refusing to silently weaken a requested policy. In the current crate, `danger-full-access` can run directly, while enforced policies compile to OS-specific execution plans and report a clear backend-unavailable error when the required backend is missing.

## Policy Presets

Manual should start with a small preset surface:

| Preset | Meaning |
| --- | --- |
| `read-only` | Read the workspace or declared inputs, write nothing durable, restrict network by default. |
| `workspace-write` | Allow writes inside declared workspace roots while protecting sensitive metadata. |
| `danger-full-access` | Run without sandbox restrictions. The name should remain intentionally loud. |

These presets compile into richer filesystem and network policies internally.

## Policy Model

A useful v0 policy can look like this:

```text
SandboxPolicy
  filesystem:
    mode: restricted | unrestricted | external
    entries:
      - path
      - access: read | write | none
  network:
    mode: restricted | enabled
  process:
    cwd
    env
    timeout_ms
```

Manual should protect sensitive directories by default, especially:

- `.git`
- `.manual`
- `.codex`
- `.agents`

## Command Surface

The independent package could expose a standalone CLI:

```bash
manual-sandbox run --policy read-only --cwd . -- cargo test
manual-sandbox run --policy policy.json --cwd /path/to/workspace -- npm test
manual-sandbox explain --policy policy.json --cwd .
manual-sandbox doctor
```

Manual itself could later wrap the same functionality:

```bash
manual sandbox run --policy node-policy.json -- cargo test
```

## Design Lesson

The strongest design direction from the Codex sandbox research is to avoid a single oversimplified backend trait. Manual should separate:

- a policy model
- a policy compiler
- platform-specific backend setup
- a sandboxed process interface for stdout, stderr, exit code, timeout, and cancellation

That separation matches how real OS sandboxes differ.

The initial `sandbox` crate follows that split with:

- `SandboxPolicy`, `FilesystemPolicy`, and `NetworkPolicy` as OS-neutral policy types.
- `SandboxRunner::detect()` for local OS detection.
- `CompiledSandboxPlan` for macOS Seatbelt, Linux bubblewrap, Windows restricted-token, or explicit direct execution.
- `CommandSpec` and `SandboxResult` for stdin, captured stdout, stderr, and exit code.

The `runtime` crate builds on this surface: it receives JSON input, derives a `SandboxPolicy`, and runs either a compiled Rust script or an agent command through `SandboxRunner`.
