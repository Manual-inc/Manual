using System.Text.Json.Nodes;

namespace ManualWindow.Models;

public sealed class StopWorkflowResult
{
    public string RunId { get; }
    public bool Cancelled { get; }
    public string? Message { get; }

    public StopWorkflowResult(string runId, bool cancelled, string? message)
    {
        RunId = runId; Cancelled = cancelled; Message = message;
    }

    public static StopWorkflowResult FromJson(JsonNode result)
    {
        var obj = result.AsObject();
        return new StopWorkflowResult(
            obj["run_id"]?.GetValue<string>()   ?? throw AppServerClientException.InvalidResponse(),
            obj["cancelled"]?.GetValue<bool>()  ?? throw AppServerClientException.InvalidResponse(),
            obj["message"]?.GetValue<string>());
    }
}
