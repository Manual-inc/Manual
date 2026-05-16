using System.Text.Json.Nodes;

namespace ManualWindow.Models;

public record struct WorkflowNodePosition(double X, double Y);

public sealed class WorkflowNodeModel
{
    public string Id { get; }
    public string Title { get; }
    public string Subtitle { get; }
    public WorkflowNodeKind Kind { get; }
    public WorkflowNodePosition Position { get; }
    public WorkflowNodeStatus Status { get; set; } = WorkflowNodeStatus.Idle;
    public string? Result { get; set; }
    public string? PreviousResult { get; set; }
    public JsonObject? InputOverride { get; set; }

    public WorkflowNodeModel(
        string id, string title, string subtitle,
        WorkflowNodeKind kind, WorkflowNodePosition position)
    {
        Id = id; Title = title; Subtitle = subtitle;
        Kind = kind; Position = position;
    }
}
