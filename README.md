# Manual

Install the current verified release with:

```bash
curl -fsSL https://github.com/Manual-inc/Manual/releases/download/v0.1.0-codex.4/install.sh | bash
```

The installer places `manual` and `manual-app-server` in `~/.local/bin` by default.

The CLI now ships dedicated app-server command groups for `workflow`, `node`, `agent`, `manual`, `optimization`, `sandbox`, and `skill`, with raw `rpc` kept as a fallback. See [Manual CLI app-server command surface](docs/wiki/architecture/manual-cli-command-surface.md) for the full mapping.

Examples:

```bash
manual workflow list
manual node list
manual sandbox list
manual skill agent-capabilities
```

Once stable releases are published, you can install the latest stable release with:

```bash
curl -fsSL https://github.com/Manual-inc/Manual/releases/latest/download/install.sh | bash
```
