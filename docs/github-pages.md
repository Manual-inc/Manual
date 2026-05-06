# GitHub Pages

The `docs/` directory is ready to serve as a GitHub Pages source.

The Markdown files are the source of truth. Static HTML files are generated from those Markdown files by `docs/build.mjs`.

## Build Locally

From the repository root:

```bash
node docs/build.mjs
```

The generator writes HTML files next to the Markdown source:

- `docs/index.html`
- `docs/product.html`
- `docs/architecture.html`
- `docs/workflow.html`
- `docs/agents.html`
- `docs/sandbox.html`
- `docs/cli-and-skill.html`
- `docs/roadmap.html`
- `docs/github-pages.html`

It also uses:

- `docs/assets/site.css`
- `docs/assets/manual-map.svg`
- `docs/.nojekyll`

## Publish With GitHub Pages

In the repository settings:

1. Open **Settings**.
2. Open **Pages**.
3. Set the source to **Deploy from a branch**.
4. Choose the branch to publish.
5. Choose the `/docs` folder.
6. Save the settings.

GitHub Pages will serve `docs/index.html` as the documentation home page.

## Editing Docs

Edit the Markdown files first. Then run:

```bash
node docs/build.mjs
```

Commit both the Markdown source and generated HTML so the published site updates without a separate build system.
