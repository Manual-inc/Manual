# Manual

Manual is a Rust-based, fast, lightweight runtime for turning repeatable work into agent-executable workflows.

## Core Features

- **Workflow**: describes repeatable work as a graph of nodes, edges, branches, loops, integrations, acceptance criteria, and artifacts.
- **Agent**: treats Codex, Claude Code, scripts, Python programs, and future adapters as executable units that can inspect, reason, calculate, and produce results.
- **Sandbox**: wraps each agent or workflow node in an OS-native execution boundary so file, network, process, and workspace permissions stay explicit.

## What This Repository Contains

This repository currently starts as a compact Rust workspace:

- `core`: shared Manual domain language and workspace metadata.
- `cli`: a thin command entrypoint for inspecting and validating Manual assets.
- `skill`: a bundled agent skill template plus validation entrypoints.
- `app`: an early application shell that depends on the shared core.

The product direction is broader than the first implementation: Manual is intended to become a local-first automation control plane for workflow graphs, agent routing, sandbox policies, run history, cost tracking, artifacts, and a localhost visualization surface.

## Documentation

Detailed English documentation lives in [docs](docs/README.md). The same Markdown source can be converted into a static GitHub Pages site:

```bash
node docs/build.mjs
```

After running the build, publish the `docs/` directory with GitHub Pages.
