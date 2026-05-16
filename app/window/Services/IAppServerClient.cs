using System.Text.Json.Nodes;
using ManualWindow.Models;

namespace ManualWindow.Services;

public interface IAppServerClient
{
    Task<List<WorkflowSummary>> WorkflowsAsync(CancellationToken ct = default);
    Task<JsonObject> WorkflowAsync(string id, CancellationToken ct = default);
    Task<WorkflowMutationResult> CreateWorkflowAsync(JsonObject workflow, CancellationToken ct = default);
    Task<WorkflowMutationResult> UpdateWorkflowAsync(string id, JsonObject workflow, CancellationToken ct = default);
    Task<WorkflowDeleteResult> DeleteWorkflowAsync(string id, CancellationToken ct = default);
    Task<string> StartWorkflowAsync(string workflowId, WorkflowStartOptions? options = null, CancellationToken ct = default);
    Task<StopWorkflowResult> StopWorkflowAsync(string runId, CancellationToken ct = default);
    Task<string> ResumeWorkflowAsync(string runId, WorkflowStartOptions? options = null, CancellationToken ct = default);
    Task<WorkflowEventsPage> EventsAsync(string runId, int cursor, CancellationToken ct = default);
    IAsyncEnumerable<AppServerLiveEvent> LiveEventsAsync([System.Runtime.CompilerServices.EnumeratorCancellation] CancellationToken ct = default);
}
