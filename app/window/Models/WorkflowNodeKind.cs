namespace ManualWindow.Models;

public enum WorkflowNodeKind
{
    Context,
    Script,
    Agent,
    Claude,
    Codex,
    Digest,
}

public static class WorkflowNodeKindExtensions
{
    public static WorkflowNodeKind FromServerKind(string value) => value switch
    {
        "claude"   => WorkflowNodeKind.Claude,
        "constant" => WorkflowNodeKind.Context,
        "codex"    => WorkflowNodeKind.Codex,
        "pi"       => WorkflowNodeKind.Agent,
        "template" => WorkflowNodeKind.Digest,
        _          => WorkflowNodeKind.Script,
    };

    public static string DisplayName(this WorkflowNodeKind kind) => kind switch
    {
        WorkflowNodeKind.Context => "Context",
        WorkflowNodeKind.Script  => "Rust Script",
        WorkflowNodeKind.Agent   => "Pi Agent",
        WorkflowNodeKind.Claude  => "Claude Review",
        WorkflowNodeKind.Codex   => "Codex Review",
        WorkflowNodeKind.Digest  => "Digest",
        _                        => kind.ToString(),
    };
}
