namespace ManualWindow.Models;

public sealed class WorkflowEventModel
{
    public Guid Id { get; } = Guid.NewGuid();
    public DateTimeOffset Time { get; }
    public string? NodeId { get; }
    public string Title { get; }
    public string Detail { get; }

    public WorkflowEventModel(DateTimeOffset time, string? nodeId, string title, string detail)
    {
        Time = time; NodeId = nodeId; Title = title; Detail = detail;
    }
}
