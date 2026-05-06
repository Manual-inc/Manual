# Manual Documentation

Manual turns repeatable operational knowledge into workflows that agents can install, execute, measure, and improve.

## The Short Version

Manual is not a prompt library. It is a Rust-based execution layer for turning "the way this work should be done" into a reusable workflow graph. A workflow connects agent nodes, sandbox boundaries, integrations, cost records, and artifacts into something that can be run again with different inputs.

The current repository is intentionally small. It provides a Rust workspace with shared core metadata, node contracts, workflow graph validation, sandboxed runtime execution, a CLI shell, an app shell, and a skill packaging path. The docs describe both the implemented foundation and the product direction gathered from the local Manual wiki.

## Core Concepts

| Concept | Meaning |
| --- | --- |
| Workflow | The graph that defines repeatable work, including nodes, edges, branches, loops, inputs, outputs, and artifacts. |
| Agent | The executable unit inside a node. An agent can be Codex, Claude Code, a script, a Python program, or a future runtime adapter. |
| Sandbox | The execution boundary around an agent or node. It decides what can be read, written, reached over the network, or spawned. |
| Runtime | The layer that receives node input, applies a sandbox policy, and executes a script or agent target. |

The key product equation is:

```text
agent + sandbox = executable node
nodes + edges = workflow
workflow + run history = optimization loop
```

## Documentation Map

- [Product Direction](product.md): what Manual is, who it serves, and why cost-aware workflow automation matters.
- [Architecture](architecture.md): how the Rust workspace maps to the larger Manual architecture.
- [Workflow](workflow.md): workflow graph concepts, node types, edge types, validation, and an example spec.
- [Agents](agents.md): what counts as an agent, how adapters fit, and how agent nodes should behave.
- [Sandbox](sandbox.md): the OS-native sandbox direction and the policy model Manual should expose.
- [CLI and Skill](cli-and-skill.md): current commands, planned command surface, and the bundled skill path.
- [Roadmap](roadmap.md): current implementation status and recommended next steps.
- [GitHub Pages](github-pages.md): how the Markdown docs become a static docs site.

## Static Site

Run the generator from the repository root:

```bash
node docs/build.mjs
```

It reads the Markdown files in `docs/` and writes static HTML pages into the same directory. GitHub Pages can then serve the `docs/` directory directly.
