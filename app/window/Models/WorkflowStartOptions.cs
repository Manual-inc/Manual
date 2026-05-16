using System.Text.Json.Nodes;

namespace ManualWindow.Models;

public enum ExecutionMode { Auto, Step }

public sealed class WorkflowStartOptions
{
    public string? StartNodeId { get; set; }
    public bool ResumeFromFailure { get; set; }
    public JsonObject InputOverrides { get; set; } = new();
    public ExecutionMode Mode { get; set; } = ExecutionMode.Auto;
    public string? ResumeRunId { get; set; }
}
