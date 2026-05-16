using System.Text.Json.Nodes;

namespace ManualWindow.Models;

public static class BusinessWorkflowExample
{
    public const string WorkflowId = "business-pipeline-health";

    public static readonly IReadOnlyList<WorkflowNodeModel> Nodes = new[]
    {
        new WorkflowNodeModel("weekly_context",    "Weekly Context",    "B2B SaaS, 2026-W19",         WorkflowNodeKind.Context, new WorkflowNodePosition(0.10, 0.45)),
        new WorkflowNodeModel("sales_health",      "Sales Health",      "Rust script metrics",         WorkflowNodeKind.Script,  new WorkflowNodePosition(0.34, 0.25)),
        new WorkflowNodeModel("support_health",    "Support Health",    "Rust script queue scan",      WorkflowNodeKind.Script,  new WorkflowNodePosition(0.34, 0.66)),
        new WorkflowNodeModel("pi_recommendation", "Pi Recommendation", "Risk and next action",        WorkflowNodeKind.Agent,   new WorkflowNodePosition(0.58, 0.30)),
        new WorkflowNodeModel("chaos_script",      "Chaos Script",      "Intentional script failure",  WorkflowNodeKind.Script,  new WorkflowNodePosition(0.58, 0.66)),
        new WorkflowNodeModel("operator_digest",   "Operator Digest",   "Final run summary",           WorkflowNodeKind.Digest,  new WorkflowNodePosition(0.88, 0.45)),
    };

    public static readonly IReadOnlyList<WorkflowEdgeModel> Edges = new[]
    {
        new WorkflowEdgeModel("weekly_context",    "sales_health"),
        new WorkflowEdgeModel("weekly_context",    "support_health"),
        new WorkflowEdgeModel("sales_health",      "pi_recommendation"),
        new WorkflowEdgeModel("support_health",    "pi_recommendation"),
        new WorkflowEdgeModel("pi_recommendation", "chaos_script"),
        new WorkflowEdgeModel("weekly_context",    "operator_digest"),
        new WorkflowEdgeModel("sales_health",      "operator_digest"),
        new WorkflowEdgeModel("support_health",    "operator_digest"),
        new WorkflowEdgeModel("pi_recommendation", "operator_digest"),
        new WorkflowEdgeModel("chaos_script",      "operator_digest"),
    };

    public static JsonObject JsonDefinition => JsonNode.Parse("""
        {
          "id": "business-pipeline-health",
          "nodes": [
            { "id": "weekly_context",    "kind": "constant", "payload": { "account": "B2B SaaS", "week": "2026-W19" } },
            { "id": "sales_health",      "kind": "script",   "script": "sales_health.rhai" },
            { "id": "support_health",    "kind": "script",   "script": "support_health.rhai" },
            { "id": "pi_recommendation", "kind": "pi",       "model": "pi-3" },
            { "id": "chaos_script",      "kind": "fail",     "error": "Intentional script failure" },
            { "id": "operator_digest",   "kind": "template", "template": "Final run summary" }
          ],
          "dependencies": [
            { "node": "sales_health",      "depends_on": "weekly_context" },
            { "node": "support_health",    "depends_on": "weekly_context" },
            { "node": "pi_recommendation", "depends_on": "sales_health" },
            { "node": "pi_recommendation", "depends_on": "support_health" },
            { "node": "chaos_script",      "depends_on": "pi_recommendation" },
            { "node": "operator_digest",   "depends_on": "weekly_context" },
            { "node": "operator_digest",   "depends_on": "sales_health" },
            { "node": "operator_digest",   "depends_on": "support_health" },
            { "node": "operator_digest",   "depends_on": "pi_recommendation" },
            { "node": "operator_digest",   "depends_on": "chaos_script" }
          ]
        }
        """)!.AsObject();
}
