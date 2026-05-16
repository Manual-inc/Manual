namespace ManualWindow.Models;

public sealed class WorkflowDisplayModel
{
    public string WorkflowId { get; }
    public IReadOnlyList<WorkflowNodeModel> Nodes { get; }
    public IReadOnlyList<WorkflowEdgeModel> Edges { get; }

    public WorkflowDisplayModel(string workflowId,
        IReadOnlyList<WorkflowNodeModel> nodes,
        IReadOnlyList<WorkflowEdgeModel> edges)
    {
        WorkflowId = workflowId; Nodes = nodes; Edges = edges;
    }
}
