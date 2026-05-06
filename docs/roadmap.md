# Roadmap

Manual should grow by completing one real path at a time.

The current repository is a small Rust foundation. The next work should avoid broad scaffolding until a workflow can be created, validated, run, inspected, and explained.

## Current Status

Implemented today:

- Rust workspace with `core`, `cli`, `skill`, and `app`.
- Shared workspace descriptor in `core`.
- CLI `about` command.
- CLI skill validation by checking for `SKILL.md`.
- `manual-skill` wrapper behavior for bundled skill template validation.
- A default skill template with a reference note.
- English Markdown documentation and a static GitHub Pages docs site generator.

Not implemented yet:

- Workflow graph domain model.
- Workflow import/export.
- Job runner.
- Runtime adapters.
- Cost ledger.
- Artifact store.
- Sandbox backend.
- `manual serve` local visualization server.

## Recommended Build Order

1. Add a workflow domain module to `core`.
2. Start with the smallest graph model: workflow ID, name, goal, entry node, nodes, and edges.
3. Add validation for empty graphs, missing entry nodes, duplicate node IDs, missing edge endpoints, and self-loops.
4. Expose `manual workflow validate <file>` through the CLI.
5. Add a sample workflow spec and use it as a fixture.
6. Add `manual workflow import` and `manual workflow show` with local file storage.
7. Add a minimal job runner that records node status without executing external agents.
8. Add a script or command adapter before adding a full coding-agent adapter.
9. Add sandbox policy metadata to nodes before enforcing platform sandboxes.
10. Add cost records after runs produce stable node execution events.
11. Add `manual serve` when there is real workflow and run data to visualize.

## MVP Demo Path

The first end-to-end demo should be debugging automation.

Required output:

- root-cause report
- patch or explanation for why no patch was applied
- test output
- artifact list
- node-level execution timeline
- premium baseline cost
- actual run cost
- saved percent

## Risk Controls

- Keep Manual as the user-facing name. Treat Stables as historical context only.
- Keep workflow syntax readable by humans and agents.
- Avoid adding every runtime at once. Codex and scripts are enough for the first adapter path.
- Do not claim cost savings unless token usage and pricing snapshots are recorded.
- Bind local visualization to `127.0.0.1` by default.
- Make sandbox fallback behavior explicit. Do not silently downgrade a requested policy.

## Success Criteria

Manual reaches its first meaningful milestone when the same workflow can be run more than once with different inputs and still produce structured artifacts, node status, and cost records.
