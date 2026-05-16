using System.Text.Json.Nodes;

namespace ManualWindow.Models;

public sealed class WorkflowEventsPage
{
    public IReadOnlyList<JsonObject> Events { get; }
    public int NextCursor { get; }
    public bool Completed { get; }
    public JsonObject Run { get; }
    public string? FirstFailedNode { get; }
    public bool Resumable { get; }
    public bool Paused { get; }

    public WorkflowEventsPage(
        IReadOnlyList<JsonObject> events,
        int nextCursor,
        bool completed,
        JsonObject run)
    {
        Events = events;
        NextCursor = nextCursor;
        Completed = completed;
        Run = run;
        FirstFailedNode = run["first_failed_node"]?.GetValue<string>();
        Resumable = run["resumable"]?.GetValue<bool>() ?? false;
        Paused = run["paused"]?.GetValue<bool>() ?? false;
    }
}
