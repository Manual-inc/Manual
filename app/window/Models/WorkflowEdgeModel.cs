namespace ManualWindow.Models;

public sealed class WorkflowEdgeModel
{
    public string From { get; }
    public string To { get; }
    public string Id => $"{From}->{To}";

    public WorkflowEdgeModel(string from, string to)
    {
        From = from; To = to;
    }
}
