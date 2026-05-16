using System.Text.Json.Nodes;

namespace ManualWindow.Models;

public sealed class WorkflowDeleteResult
{
    public string WorkflowId { get; }
    public bool Deleted { get; }

    public WorkflowDeleteResult(string workflowId, bool deleted)
    {
        WorkflowId = workflowId; Deleted = deleted;
    }

    public static WorkflowDeleteResult FromJson(JsonNode result)
    {
        var obj = result.AsObject();
        return new WorkflowDeleteResult(
            obj["workflow_id"]?.GetValue<string>() ?? throw AppServerClientException.InvalidResponse(),
            obj["deleted"]?.GetValue<bool>()       ?? throw AppServerClientException.InvalidResponse());
    }
}
