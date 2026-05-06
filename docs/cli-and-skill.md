# CLI and Skill

Manual is meant to be used by both people and agents.

The CLI gives agents a stable local tool surface. The skill gives agents the instructions and templates needed to turn natural language requests into Manual workflow actions.

## Current CLI

The current `cli` crate supports a small command surface:

```bash
cargo run -p cli -- about
cargo run -p cli -- validate-skill crates/skill/templates/default-skill
```

The `about` command reports the workspace name and package list. The `validate-skill` command checks that a skill directory contains a `SKILL.md` file.

## Current Skill Binary

The `skill` crate provides the `manual-skill` binary:

```bash
cargo run -p skill -- template-path
cargo run -p skill -- validate-bundled
```

It can report the bundled template path and validate that the built-in skill template has the required entrypoint.

## Bundled Skill Template

The default template lives at:

```text
crates/skill/templates/default-skill/SKILL.md
```

It defines the minimum structure for a Codex or Claude-style agent skill package:

- `SKILL.md` as the required instruction entrypoint.
- `references/` for supplemental human and agent guidance.

## Planned Manual CLI

The product direction calls for a Rust `manual` CLI that can manage workflows, jobs, runs, costs, artifacts, sandbox policies, and local visualization.

Likely commands:

```bash
manual init
manual workflow list
manual workflow show <workflow-id>
manual workflow validate <file>
manual workflow import <file>
manual workflow export <workflow-id>
manual job run <workflow-id>
manual job list
manual job show <job-id>
manual cost report <job-id>
manual artifact list <job-id>
manual sandbox run --policy <policy> -- <command>
manual serve
```

The current crate names are still simple (`cli`, `skill`, `app`, `core`). A later productization step can rename binaries and packages when the command surface is ready.

## Skill Responsibility

The Manual Skill should help an agent:

1. Understand the repeatable work the user wants to automate.
2. Ask only the minimum clarifying questions required to produce a reusable workflow.
3. Write a human-readable workflow spec.
4. Validate or import the workflow with the Manual CLI.
5. Run jobs when requested.
6. Summarize artifacts, costs, failures, and next improvements.

The user should not need to learn low-level workflow syntax before receiving value.
