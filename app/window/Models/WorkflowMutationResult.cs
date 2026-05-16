using System.Text.Json.Nodes;

namespace ManualWindow.Models;

public sealed class WorkflowMutationResult
{
    public string WorkflowId { get; }
    public int NodeCount { get; }

    public WorkflowMutationResult(string workflowId, int nodeCount)
    {
        WorkflowId = workflowId; NodeCount = nodeCount;
    }

    public static WorkflowMutationResult FromJson(JsonNode result)
    {
        var obj = result.AsObject();
        return new WorkflowMutationResult(
            obj["workflow_id"]?.GetValue<string>() ?? throw AppServerClientException.InvalidResponse(),
            obj["node_count"]?.GetValue<int>()     ?? throw AppServerClientException.InvalidResponse());
    }
}
