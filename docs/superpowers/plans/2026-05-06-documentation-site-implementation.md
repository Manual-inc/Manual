# Documentation Site Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add English product documentation for Manual and generate a static GitHub Pages site from the docs directory.

**Architecture:** Keep Markdown files as the source of truth under `docs/`. Use a small dependency-free Node.js generator to convert the Markdown files into static HTML pages that GitHub Pages can serve from `/docs`.

**Tech Stack:** Markdown, HTML, CSS, Node.js built-in modules, Rust workspace verification.

---

### Task 1: Gather Product Context

**Files:**
- Read: `/Users/leejs/Documents/wiki/wiki/projects/Manual.md`
- Read: `/Users/leejs/Documents/wiki/wiki/sources/manual-core-features-note-2026-05-05.md`
- Read: `/Users/leejs/Documents/wiki/wiki/analyses/manual-prd-2026-05-01.md`
- Read: `/Users/leejs/Documents/wiki/wiki/analyses/manual-sandbox-package-design-2026-05-04.md`
- Read: `/Users/leejs/github/Manual/Cargo.toml`
- Read: `/Users/leejs/github/Manual/crates/core/src/lib.rs`
- Read: `/Users/leejs/github/Manual/crates/cli/src/lib.rs`
- Read: `/Users/leejs/github/Manual/crates/skill/src/lib.rs`

- [x] **Step 1: Identify product direction**

Manual is a Rust-based, fast, lightweight runtime for turning repeatable work into agent-executable workflows. The three core features are workflow, agent, and sandbox.

- [x] **Step 2: Identify current implementation**

The current repository is a compact Rust workspace with `core`, `cli`, `skill`, and `app`. It does not yet implement the full workflow graph, job runner, cost ledger, or sandbox runtime.

### Task 2: Add Markdown Documentation

**Files:**
- Create: `/Users/leejs/github/Manual/README.md`
- Create: `/Users/leejs/github/Manual/docs/README.md`
- Create: `/Users/leejs/github/Manual/docs/product.md`
- Create: `/Users/leejs/github/Manual/docs/architecture.md`
- Create: `/Users/leejs/github/Manual/docs/workflow.md`
- Create: `/Users/leejs/github/Manual/docs/agents.md`
- Create: `/Users/leejs/github/Manual/docs/sandbox.md`
- Create: `/Users/leejs/github/Manual/docs/cli-and-skill.md`
- Create: `/Users/leejs/github/Manual/docs/roadmap.md`
- Create: `/Users/leejs/github/Manual/docs/github-pages.md`

- [x] **Step 1: Write root README**

Include a one-line product summary that emphasizes Rust, speed, and lightness. Explicitly list Workflow, Agent, and Sandbox as core features.

- [x] **Step 2: Write detailed docs**

Split product direction, architecture, workflow, agents, sandbox, CLI and skill usage, roadmap, and GitHub Pages instructions into separate Markdown files.

### Task 3: Add Static Site Generator

**Files:**
- Create: `/Users/leejs/github/Manual/docs/build.mjs`
- Create: `/Users/leejs/github/Manual/docs/assets/site.css`
- Create: `/Users/leejs/github/Manual/docs/assets/manual-map.svg`
- Create: `/Users/leejs/github/Manual/docs/.nojekyll`

- [x] **Step 1: Generate HTML from Markdown**

Create a dependency-free Node.js script that reads the docs Markdown files and writes static HTML files.

- [x] **Step 2: Add site styling**

Create a documentation-focused UI with a persistent navigation rail, readable article layout, high-contrast text, and a visual Manual execution map asset.

### Task 4: Verify

**Files:**
- Verify: `/Users/leejs/github/Manual/docs/*.html`
- Verify: `/Users/leejs/github/Manual/README.md`
- Verify: `/Users/leejs/github/Manual/docs/*.md`

- [x] **Step 1: Run docs build**

Run: `node docs/build.mjs`
Expected: HTML pages are generated under `docs/`.

- [x] **Step 2: Run Rust tests**

Run: `cargo test`
Expected: the existing Rust workspace tests pass.

- [x] **Step 3: Inspect generated docs**

Confirm generated HTML files contain navigation links and the Markdown docs remain English.
