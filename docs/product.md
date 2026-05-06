# Product Direction

Manual is an agent workflow runtime for repeatable work.

The product idea came from a shift away from naming the system around harnesses or stables and toward a user-facing metaphor: a manual is a repeatable procedure. Instead of asking people to read that procedure and perform it by hand, Manual lets agents install, interpret, and execute it.

## Product Positioning

Manual is best understood as an AI work automation MSP.

A cloud MSP helps teams operate infrastructure more efficiently, then proves the savings. Manual aims to do the same for agent workflows. It should show the difference between running every step on an expensive premium model and routing each step to the cheapest agent, model, script, or rule that can still produce the required quality.

## Primary Promise

Manual helps a user say:

```text
This is a repeatable job. Make it work.
```

The system should then turn that job into a reusable workflow, run it, capture outputs, measure token and model cost, and improve the route over time.

## Problems Manual Solves

- Agent runtimes are powerful, but real production use still requires users to understand tools, permissions, runtime placement, model choice, and local access.
- Teams repeatedly explain the same workflow to coding agents, support agents, or internal automation scripts.
- Simple classification, summarization, checks, and formatting often consume the same premium model budget as hard coding or reasoning work.
- Local files, secure systems, cron-like schedules, and internal tools do not fit cleanly into one automation product.
- It is difficult to prove whether an agent workflow is saving time, saving tokens, preserving quality, or simply hiding work inside expensive model calls.

## Product Shape

Manual combines three core features:

- **Workflow**: a graph of repeatable work.
- **Agent**: an executable reasoning or automation unit.
- **Sandbox**: a safe execution boundary around every node.

The workflow defines the structure. The agent performs the work. The sandbox keeps each action inside a declared permission boundary.

## Target Users

Manual starts with developers and development teams because debugging, testing, review, and documentation workflows are concrete and measurable. The same structure can later support operations, customer support, product management, marketing, sales, and internal support teams.

| User | Repeatable Work | Manual Value |
| --- | --- | --- |
| Development team | Debugging, tests, code review, release checks | Standardized execution, artifacts, cost tracking |
| Operations or support | Ticket triage, incident reports, response drafts | Faster responses and consistent quality |
| Product managers | Meeting notes, requirement cleanup, issue creation | Structured handoff and less manual formatting |
| Marketing or sales | Research, lead cleanup, proposal drafts | Repeatable research and document generation |
| Internal support | Policy answers, request review, notices | Auditable knowledge work automation |

## MVP Focus

The MVP should connect a Manual Skill, a Rust `manual` CLI, a workflow graph spec, a Codex-oriented runtime adapter, cost measurement, and local visualization.

The first useful demo is debugging automation:

1. Receive a QA, VOC, or operations issue.
2. Inspect symptoms, code, logs, and tests.
3. identify likely root cause.
4. draft and apply a fix.
5. add or update tests.
6. produce a root-cause report, patch, test result, and cost report.

Pull request creation, deployment, organization approval policies, and deep multi-runtime federation can wait until the core workflow loop is real.
