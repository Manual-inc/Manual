using ManualWindow.Models;
using Xunit;

namespace ManualWindow.Tests;

public class WorkflowModelsTests
{
    [Fact]
    public void ServerKindMapping_AllCasesCorrect()
    {
        Assert.Equal(WorkflowNodeKind.Claude,   WorkflowNodeKindExtensions.FromServerKind("claude"));
        Assert.Equal(WorkflowNodeKind.Context,  WorkflowNodeKindExtensions.FromServerKind("constant"));
        Assert.Equal(WorkflowNodeKind.Agent,    WorkflowNodeKindExtensions.FromServerKind("pi"));
        Assert.Equal(WorkflowNodeKind.Digest,   WorkflowNodeKindExtensions.FromServerKind("template"));
        Assert.Equal(WorkflowNodeKind.Codex,    WorkflowNodeKindExtensions.FromServerKind("codex"));
        Assert.Equal(WorkflowNodeKind.Script,   WorkflowNodeKindExtensions.FromServerKind("unknown"));
    }
}
