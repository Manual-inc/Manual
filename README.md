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
manual workflow starter code-review --run
```

Browse starter presets:

```bash
manual workflow starter
```

Start here for setup and product entry points:

- [Quick Start](docs/wiki/analyses/2026-05-19-quick-start.md)
- [Wiki Index](docs/wiki/목차.md)
- [Demo Flow Notes](docs/wiki/analyses/2026-05-19-demo-flow.md)
- [CLI Command Surface](docs/wiki/architecture/manual-cli-command-surface.md)
- [App Architecture](docs/wiki/architecture/manual-app-architecture.md)
