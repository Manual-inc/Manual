# Agents

An agent is the executable unit inside a Manual workflow node.

Manual should use a broad definition. An agent can be a coding agent, a language model call, a Python script, a shell command, or a future runtime adapter. The shared requirement is that it receives input, performs useful work, and returns output that the workflow can route or store.

## Agent Roles

The wiki notes split agent behavior into three useful roles:

| Role | Meaning |
| --- | --- |
| Recognizer | Reads inputs, files, logs, tickets, tool output, or environment state. |
| Reasoner or Executor | Makes decisions, calculates, edits, runs tools, or calls models. |
| Emitter | Produces structured output, artifacts, status events, or human-readable reports. |

A single node can hide a simple internal pipeline, but the public workflow graph should stay readable.

## Supported Agent Shapes

Manual should eventually support:

- Codex as an initial coding-agent adapter.
- Claude Code as a second coding-agent adapter.
- Rust scripts for deterministic JSON-in, text-out transforms, including dependency-backed scripts compiled through Cargo.
- Python scripts for deterministic or semi-deterministic work.
- Shell commands for tests, checks, transforms, and local automation.
- Future OpenCode or Agent SDK adapters.
- Lightweight model calls for classification, summarization, formatting, and routing.

## Adapter Boundary

The job runner should not care whether a node used Codex, Claude Code, Python, or a local binary. It should receive common events:

- node started
- node wrote log output
- node produced an artifact
- node reported token usage
- node succeeded, failed, skipped, or waited for approval

Runtime-specific details belong behind adapters.

## Agent Policy

Manual should route work based on the job, not based on one global model choice.

Examples:

- Use a lightweight model for ticket classification.
- Use a script for formatting, parsing, or static checks.
- Use a stronger coding agent for root-cause analysis and patch creation.
- Use a deterministic command for verification.
- Use human approval when the patch touches sensitive code or production configuration.

This is the basis of Manual's cost optimization story.

## Node Contract

Every agent node should make these details explicit:

| Field | Purpose |
| --- | --- |
| Input contract | What the node needs to read. |
| Output contract | What downstream nodes can depend on. |
| Sandbox policy | What the node may read, write, or access over the network. |
| Model or runtime policy | Which agent or model class should execute it. |
| Artifact contract | What durable output should be preserved. |
| Acceptance condition | How the workflow knows the node result is usable. |

Readable contracts matter because Manual is meant to be used by people through agents, not only by developers writing low-level config.
