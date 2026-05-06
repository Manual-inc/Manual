# Architecture

Manual is designed as a local-first Rust workspace that can grow into a workflow control plane.

The current implementation is deliberately compact. It establishes a shared Rust foundation before adding persistence, runners, adapters, cost ledgers, or a web UI.

## Current Workspace

The repository contains four workspace members:

| Crate | Role Today | Intended Direction |
| --- | --- | --- |
| `core` | Shared workspace descriptor and domain foundation. | Own domain language, validation rules, workflow types, job types, and cost records. |
| `cli` | Thin command entrypoint with `about` and skill validation behavior. | Become the user and agent entrypoint for workflow, job, run, cost, artifact, sandbox, and serve commands. |
| `skill` | Bundled skill template and delegation into the CLI validator. | Package Manual instructions so agents can create, validate, and run workflows through the CLI. |
| `app` | Minimal app shell that proves the core can be shared. | Become the local visualization or application surface, likely served by `manual serve`. |

## Product Architecture

The broader Manual architecture has these parts:

| Layer | Responsibility |
| --- | --- |
| Manual CLI | Local command surface for users and agents. |
| Workflow Graph | Repeatable work represented as nodes, edges, policies, and artifact contracts. |
| Job Runner | Turns a workflow into a tracked run with node execution state. |
| Runtime Adapters | Connect Codex, Claude Code, scripts, Python, and future agent runtimes. |
| Sandbox Runtime | Applies OS-native execution boundaries around nodes. |
| Cost Ledger | Records token usage, model prices, baseline cost, actual cost, and savings. |
| Artifact Store | Keeps reports, patches, logs, test output, and other run products. |
| Local Visualization | Shows workflow graphs, timelines, node details, costs, and artifacts at localhost. |

## Boundaries

Manual should keep these concerns separate:

- Domain validation belongs in `core`.
- CLI parsing belongs in `cli`.
- Skill packaging belongs in `skill`.
- Visualization belongs in `app` or a future server crate.
- Runtime adapter code should not leak into workflow validation.
- Sandbox policy modeling should be shared, while platform-specific execution remains behind backend modules.

That separation keeps the codebase easy to extend without turning the first CLI into a large mixed-purpose file.

## Data Flow

```text
natural language request
  -> Manual Skill
  -> workflow graph spec
  -> manual workflow import
  -> manual job run
  -> node execution through adapter
  -> sandboxed process
  -> artifact and cost records
  -> manual serve visualization
```

The current repository implements only the earliest part of this path. The docs describe the intended path so the next implementation steps stay coherent.

## Design Principle

Manual should prefer the smallest complete path over broad scaffolding. A thin workflow command that calls core validation is better than a large control plane that cannot execute one real workflow.
