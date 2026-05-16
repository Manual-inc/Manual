using ManualWindow.Models;
using ManualWindow.Services;
using ManualWindow.Stores;
using System.Text.Json.Nodes;
using Xunit;

namespace ManualWindow.Tests;

// Fake client for unit tests (no UI dispatcher needed — tests run on test thread)
internal sealed class FakeAppServerClient : IAppServerClient
{
    public List<WorkflowSummary> Workflows { get; set; } = new();
    public JsonObject? WorkflowJson { get; set; }
    public string StartedRunId { get; set; } = "run-1";
    public bool StopCalled { get; private set; }
    public WorkflowStartOptions? LastStartOptions { get; private set; }

    public Task<List<WorkflowSummary>> WorkflowsAsync(CancellationToken ct = default)
        => Task.FromResult(Workflows);

    public Task<JsonObject> WorkflowAsync(string id, CancellationToken ct = default)
        => Task.FromResult(WorkflowJson ?? new JsonObject { ["id"] = id, ["nodes"] = new JsonArray(), ["dependencies"] = new JsonArray() });

    public Task<WorkflowMutationResult> CreateWorkflowAsync(JsonObject workflow, CancellationToken ct = default)
        => Task.FromResult(new WorkflowMutationResult("wf-new", 0));

    public Task<WorkflowMutationResult> UpdateWorkflowAsync(string id, JsonObject workflow, CancellationToken ct = default)
        => Task.FromResult(new WorkflowMutationResult(id, 0));

    public Task<WorkflowDeleteResult> DeleteWorkflowAsync(string id, CancellationToken ct = default)
        => Task.FromResult(new WorkflowDeleteResult(id, true));

    public Task<string> StartWorkflowAsync(string id, WorkflowStartOptions? options = null, CancellationToken ct = default)
    {
        LastStartOptions = options;
        return Task.FromResult(StartedRunId);
    }

    public Task<StopWorkflowResult> StopWorkflowAsync(string runId, CancellationToken ct = default)
    {
        StopCalled = true;
        return Task.FromResult(new StopWorkflowResult(runId, true, null));
    }

    public Task<string> ResumeWorkflowAsync(string runId, WorkflowStartOptions? options = null, CancellationToken ct = default)
        => Task.FromResult("run-resumed");

    public Task<WorkflowEventsPage> EventsAsync(string runId, int cursor, CancellationToken ct = default)
        => Task.FromResult(new WorkflowEventsPage(new List<JsonObject>(), cursor, true, new JsonObject()));

    public async IAsyncEnumerable<AppServerLiveEvent> LiveEventsAsync(
        [System.Runtime.CompilerServices.EnumeratorCancellation] CancellationToken ct = default)
    {
        await Task.CompletedTask;
        yield break;
    }
}

// Note: WorkflowRunStore requires DispatcherQueue (UI thread) so full reducer tests
// require a UI-thread runner. These tests validate model logic without the store dispatcher.
public class WorkflowRunStoreTests
{
    [Fact]
    public void WorkflowDisplayBuilder_EmptyWorkflow_ReturnsEmptyModel()
    {
        var json = new JsonObject
        {
            ["id"] = "test",
            ["nodes"] = new JsonArray(),
            ["dependencies"] = new JsonArray()
        };
        var model = WorkflowDisplayBuilder.Build(json);
        Assert.Equal("test", model.WorkflowId);
        Assert.Empty(model.Nodes);
        Assert.Empty(model.Edges);
    }

    [Fact]
    public void WorkflowDisplayBuilder_LinearChain_AssignsIncreasingX()
    {
        var json = new JsonObject
        {
            ["id"] = "chain",
            ["nodes"] = new JsonArray
            {
                new JsonObject { ["id"] = "a", ["kind"] = "constant" },
                new JsonObject { ["id"] = "b", ["kind"] = "pi" },
                new JsonObject { ["id"] = "c", ["kind"] = "template" }
            },
            ["dependencies"] = new JsonArray
            {
                new JsonObject { ["node"] = "b", ["depends_on"] = "a" },
                new JsonObject { ["node"] = "c", ["depends_on"] = "b" }
            }
        };
        var model = WorkflowDisplayBuilder.Build(json);
        Assert.Equal(3, model.Nodes.Count);
        var nodeA = model.Nodes.First(n => n.Id == "a");
        var nodeB = model.Nodes.First(n => n.Id == "b");
        var nodeC = model.Nodes.First(n => n.Id == "c");
        Assert.True(nodeA.Position.X < nodeB.Position.X);
        Assert.True(nodeB.Position.X < nodeC.Position.X);
    }

    [Fact]
    public async Task FakeClient_StartWorkflow_RecordsOptions()
    {
        var fake = new FakeAppServerClient();
        var opts = new WorkflowStartOptions { StartNodeId = "node-x", Mode = ExecutionMode.Step };
        var runId = await fake.StartWorkflowAsync("wf-1", opts);
        Assert.Equal("run-1", runId);
        Assert.Equal("node-x", fake.LastStartOptions?.StartNodeId);
        Assert.Equal(ExecutionMode.Step, fake.LastStartOptions?.Mode);
    }

    [Fact]
    public async Task FakeClient_StopWorkflow_SetsFlagAndReturnsResult()
    {
        var fake = new FakeAppServerClient();
        var result = await fake.StopWorkflowAsync("run-1");
        Assert.True(fake.StopCalled);
        Assert.True(result.Cancelled);
    }

    [Fact]
    public void WorkflowStartOptions_Defaults_AreAuto()
    {
        var opts = new WorkflowStartOptions();
        Assert.Equal(ExecutionMode.Auto, opts.Mode);
        Assert.Null(opts.StartNodeId);
        Assert.False(opts.ResumeFromFailure);
        Assert.Empty(opts.InputOverrides);
    }
}
