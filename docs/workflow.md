# Workflow

A Manual workflow is a graph that describes repeatable work.

It is not a flat checklist. It should support branches, loops, joins, integrations, artifacts, cost records, and acceptance criteria. The graph exists so agents and scripts can execute work in a structured way and so humans can inspect what happened afterward.

## Minimal Concept

```text
workflow
  goal
  inputs
  nodes
  edges
  tools
  artifacts
  acceptance criteria
  model policy
  agent policy
  sandbox policy
  constraints
```

## Node Types

Manual can start with a small set of node types and expand carefully:

| Node Type | Use |
| --- | --- |
| `trigger` | Starts a run from a manual request, schedule, event, or external system. |
| `llm_task` | Delegates reasoning, analysis, writing, or classification to a model or agent. |
| `code_task` | Runs deterministic code such as tests, scripts, static analysis, or transforms. |
| `integration` | Reads or writes external systems such as GitHub, Slack, Jira, logs, or VOC tools. |
| `condition` | Chooses the next edge based on input or node output. |
| `loop` | Repeats bounded work such as retry, list processing, or iterative search. |
| `join` | Merges outputs from multiple branches. |
| `approval` | Pauses until a human approves or rejects a step. |
| `artifact` | Captures a report, patch, test output, or other durable result. |

## Edge Types

| Edge Type | Use |
| --- | --- |
| `sequence` | Move from one node to the next. |
| `branch` | Follow a path when a condition matches. |
| `loop_back` | Return to an earlier node until a bounded condition stops. |
| `parallel` | Start multiple paths from one point. |
| `join` | Merge multiple paths. |
| `error` | Route failure to retry, fallback, approval, or report generation. |

## Validation Direction

The wiki notes define a useful set of graph validation rules:

- A workflow needs at least one node.
- The entry node must point to a real node.
- Every edge source and target must point to a real node.
- Node IDs must be unique.
- Direct self-loops should be rejected.
- General cycles may be allowed when they represent intentional bounded loops.

The `workflow` crate starts with these graph validation rules and can expand toward import/export, branch conditions, loop bounds, and orchestration planning.

## Example Workflow Spec

```yaml
name: payment-voc-debugging
goal: Find why paid orders remain PENDING and produce a tested fix.
inputs:
  - voc_ticket
  - repository_path
acceptance_criteria:
  - Root cause is explained in plain English.
  - A patch is produced when the cause is in code.
  - Relevant tests pass or failures are reported.
nodes:
  - id: trigger
    type: trigger
    description: Receive the VOC ticket and repository path.
  - id: inspect
    type: llm_task
    agent: codex
    sandbox: read-only
    description: Inspect symptoms, logs, and likely code paths.
  - id: reproduce
    type: code_task
    sandbox: workspace-write
    description: Add or run a focused reproduction test.
  - id: fix
    type: llm_task
    agent: codex
    sandbox: workspace-write
    description: Apply the smallest safe code change.
  - id: verify
    type: code_task
    sandbox: workspace-write
    description: Run the relevant tests.
  - id: report
    type: artifact
    description: Write root cause, patch summary, tests, and cost notes.
edges:
  - from: trigger
    to: inspect
    type: sequence
  - from: inspect
    to: reproduce
    type: sequence
  - from: reproduce
    to: fix
    type: branch
    condition: failure_reproduced
  - from: fix
    to: verify
    type: sequence
  - from: verify
    to: report
    type: sequence
artifacts:
  - root_cause_report.md
  - patch.diff
  - test-results.txt
model_policy:
  default: low-cost
  hard_reasoning: premium
constraints:
  network: restricted
  max_loop_count: 3
```

## Workflow Optimization

A workflow becomes more valuable after repeated runs. Manual should learn which nodes are expensive, which branches fail often, which loops need bounded retries, and which LLM tasks can be replaced by code, rules, retrieval, or smaller models.

The important metric is not only "cost went down." It is "cost went down while the artifact still met the acceptance criteria."
