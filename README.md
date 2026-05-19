# Manual

Manual is a local-first AI workflow console focused on one thing: helping you run work, see what happened, and immediately understand how to optimize the next run.

Fastest demo:

```bash
manual demo optimization
```

Before the first run, you can check local connectivity with:

```bash
manual doctor
```

If `manual doctor` reports action needed, follow its printed next steps. When it is healthy, jump straight to `manual demo optimization`.

Repository demo wrapper:

```bash
bash scripts/demo-optimization.sh
```

First real workflow after the demo:

```bash
manual workflow starter --repo . --run
manual workflow starter code-review --run
manual workflow starter test-plan --run
```

Browse starter presets:

```bash
manual workflow starter
manual workflow starter --repo .
manual workflow starter --run
```

The starter catalog now tells you when each preset fits best, what result you should expect, and which changed files drove the recommendation before you launch it.
After a starter run completes, Manual also shows a reusable outcome summary so you can quickly rerun or share the result.
If you only need the stored result again later, `manual workflow starter-outcome <workflow_id>` prints that saved summary without rerunning.
`manual workflow starter-outcome --latest` prints the newest stored starter summary immediately.
Add `--copy` to either form to push that summary straight to the clipboard.

Start here for setup and product entry points:

- [Quick Start](docs/wiki/analyses/2026-05-19-quick-start.md)
- [Wiki Index](docs/wiki/목차.md)
- [Demo Flow Notes](docs/wiki/analyses/2026-05-19-demo-flow.md)
- [CLI Command Surface](docs/wiki/architecture/manual-cli-command-surface.md)
- [App Architecture](docs/wiki/architecture/manual-app-architecture.md)
