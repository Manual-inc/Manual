# Manual Project Wiki

## What this is

A persistent, LLM-maintained wiki for the Manual project: an executable-manual product for composing, running, monitoring, and refining local multi-agent workflows.

The human curates sources and product direction. The LLM maintains the wiki pages, cross-references, meeting notes, decisions, and synthesis pages.

## Directory structure

- `raw/` - Source documents. Never modify existing files here after they are created.
  - `raw/meetings/` - Meeting transcripts, rough notes, and source captures.
- `wiki/` - LLM-maintained pages.
  - `sources/` - One summary page per ingested source.
  - `meetings/` - Clean meeting minutes and follow-up records.
  - `decisions/` - Product and architecture decisions.
  - `features/` - Feature concepts, requirements, and open questions.
  - `architecture/` - System design, runtime, client, agent, and sandbox notes.
  - `analyses/` - Preserved comparisons, syntheses, and query results.
  - `index.md` - Catalog of all wiki pages. Read this first when answering wiki queries.
  - `log.md` - Chronological record of operations.
  - `overview.md` - High-level synthesis of the wiki.

## Page conventions

- Every wiki page has YAML frontmatter with `title`, `type`, `tags`, `sources`, `date_created`, and `date_updated`.
- Use Korean for content unless the source or user request is primarily English.
- Use `[[wikilinks]]` for cross-references between wiki pages.
- Cite source material by linking to the relevant page in `wiki/sources/`.
- Preserve uncertainty from rough meeting transcripts instead of overfitting unclear words.
- Flag contradictions explicitly. Do not silently overwrite older claims.

## Ingest workflow

1. Read the source completely.
2. For meeting transcripts, ingest directly and normalize obvious speech-to-text noise unless the user asks for a discussion first.
3. Create a source summary page in `wiki/sources/`.
4. Create or update meeting, decision, feature, and architecture pages.
5. Update `wiki/overview.md` if the source changes the big picture.
6. Update `wiki/index.md`.
7. Append to `wiki/log.md`.

## Query workflow

1. Read `wiki/index.md`.
2. Read relevant wiki pages and synthesize an answer with links to wiki pages and source summaries.
3. Offer to file valuable answers into `wiki/analyses/` when the answer should compound into the project memory.

## Lint checklist

- Contradictions between pages.
- Claims superseded by newer sources.
- Orphan pages with no inbound links.
- Frequently mentioned features, decisions, or architecture concepts without their own page.
- Missing cross-references.
- Knowledge gaps worth investigating.

## Domain-specific notes

- Manual is currently discussed as a local-first MVP.
- Current implementation areas include Rust app-server/agent/workflow/sandbox crates, a Rust CLI, and native clients such as the macOS Swift app.
- Meeting transcripts may contain noisy speech recognition. The wiki should preserve the operational meaning and note ambiguous terms when needed.
