using System.Text.Json.Nodes;

namespace ManualWindow.Models;

public sealed class WorkflowSummary
{
    public string WorkflowId { get; }
    public int NodeCount { get; }

    public WorkflowSummary(string workflowId, int nodeCount)
    {
        WorkflowId = workflowId; NodeCount = nodeCount;
    }

    public static WorkflowSummary FromJson(JsonNode result)
    {
        var obj = result.AsObject();
        var workflowId = obj["workflow_id"]?.GetValue<string>()
            ?? throw AppServerClientException.InvalidResponse();
        var nodeCount = obj["node_count"]?.GetValue<int>()
            ?? throw AppServerClientException.InvalidResponse();
        return new WorkflowSummary(workflowId, nodeCount);
    }
}
