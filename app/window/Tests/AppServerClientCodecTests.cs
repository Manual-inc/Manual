using System.Text.Json.Nodes;
using ManualWindow.Models;
using Xunit;

namespace ManualWindow.Tests;

public class AppServerClientCodecTests
{
    [Fact]
    public void WorkflowSummary_DecodesCorrectly()
    {
        var json = JsonNode.Parse("""{"workflow_id":"test-wf","node_count":5}""")!;
        var summary = WorkflowSummary.FromJson(json);
        Assert.Equal("test-wf", summary.WorkflowId);
        Assert.Equal(5, summary.NodeCount);
    }

    [Fact]
    public void WorkflowMutationResult_DecodesCorrectly()
    {
        var json = JsonNode.Parse("""{"workflow_id":"wf-1","node_count":3}""")!;
        var result = WorkflowMutationResult.FromJson(json);
        Assert.Equal("wf-1", result.WorkflowId);
        Assert.Equal(3, result.NodeCount);
    }

    [Fact]
    public void WorkflowDeleteResult_DecodesCorrectly()
    {
        var json = JsonNode.Parse("""{"workflow_id":"wf-del","deleted":true}""")!;
        var result = WorkflowDeleteResult.FromJson(json);
        Assert.Equal("wf-del", result.WorkflowId);
        Assert.True(result.Deleted);
    }

    [Fact]
    public void StopWorkflowResult_DecodesCorrectly()
    {
        var json = JsonNode.Parse("""{"run_id":"run-1","cancelled":true,"message":"done"}""")!;
        var result = StopWorkflowResult.FromJson(json);
        Assert.Equal("run-1", result.RunId);
        Assert.True(result.Cancelled);
        Assert.Equal("done", result.Message);
    }

    [Fact]
    public void StopWorkflowResult_NullMessage_IsHandled()
    {
        var json = JsonNode.Parse("""{"run_id":"run-2","cancelled":false}""")!;
        var result = StopWorkflowResult.FromJson(json);
        Assert.Null(result.Message);
    }

    [Fact]
    public void WorkflowEventsPage_DecodesRunMetadata()
    {
        var run = JsonNode.Parse("""
            {"run_id":"r1","status":"failed","first_failed_node":"node_x","resumable":true,"paused":false}
            """)!.AsObject();
        var page = new WorkflowEventsPage(
            new List<JsonObject>(), nextCursor: 5, completed: false, run: run);
        Assert.Equal("node_x", page.FirstFailedNode);
        Assert.True(page.Resumable);
        Assert.False(page.Paused);
        Assert.Equal(5, page.NextCursor);
    }
}
