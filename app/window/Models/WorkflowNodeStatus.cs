namespace ManualWindow.Models;

public enum WorkflowNodeStatus
{
    Idle,
    Running,
    Succeeded,
    Failed,
    Skipped,
    Paused,
    Cancelled,
}
