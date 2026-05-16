using CommunityToolkit.Mvvm.ComponentModel;
using ManualWindow.Models;
using ManualWindow.Services;
using Microsoft.UI.Dispatching;
using System.Collections.ObjectModel;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace ManualWindow.Stores;

public sealed partial class WorkflowRunStore : ObservableObject, IDisposable
{
    private readonly IAppServerClient _client;
    private readonly DispatcherQueue _dispatcher;
    private CancellationTokenSource? _liveCts;

    [ObservableProperty]
    private ObservableCollection<WorkflowSummary> _workflows = new();

    [ObservableProperty]
    private ObservableCollection<WorkflowNodeModel> _nodes = new();

    [ObservableProperty]
    private ObservableCollection<WorkflowEdgeModel> _edges = new();

    [ObservableProperty]
    private ObservableCollection<WorkflowEventModel> _events = new();

    [ObservableProperty]
    private bool _isRunning;

    [ObservableProperty]
    private bool _isLoading;

    [ObservableProperty]
    private bool _isPaused;

    [ObservableProperty]
    private bool _isResumable;

    [ObservableProperty]
    private string? _selectedNodeId;

    [ObservableProperty]
    private string? _selectedWorkflowId;

    [ObservableProperty]
    private string? _runId;

    [ObservableProperty]
    private string? _firstFailedNodeId;

    [ObservableProperty]
    private string _statusMessage = "Ready";

    [ObservableProperty]
    private string? _rawWorkflowJson;

    public WorkflowRunStore(IAppServerClient client)
    {
        _client = client;
        _dispatcher = DispatcherQueue.GetForCurrentThread();
    }

    public async Task Bootstrap()
    {
        await Refresh();
        if (Workflows.Count > 0)
        {
            await SelectWorkflow(Workflows[0].WorkflowId);
        }
        else
        {
            await _client.CreateWorkflowAsync(BusinessWorkflowExample.JsonDefinition);
            await Refresh();
            if (Workflows.Count > 0)
                await SelectWorkflow(Workflows[0].WorkflowId);
        }

        _liveCts = new CancellationTokenSource();
        _ = Task.Run(() => RunLiveEventsAsync(_liveCts.Token));
    }

    public async Task Refresh()
    {
        IsLoading = true;
        var list = await _client.WorkflowsAsync();
        Run(() => Workflows = new ObservableCollection<WorkflowSummary>(list));
        IsLoading = false;
    }

    public async Task SelectWorkflow(string workflowId)
    {
        SelectedWorkflowId = workflowId;
        IsLoading = true;
        var json = await _client.WorkflowAsync(workflowId);
        RawWorkflowJson = json.ToJsonString(new JsonSerializerOptions { WriteIndented = true });
        var display = WorkflowDisplayBuilder.Build(json);
        Run(() =>
        {
            Nodes = new ObservableCollection<WorkflowNodeModel>(display.Nodes);
            Edges = new ObservableCollection<WorkflowEdgeModel>(display.Edges);
            Events = new ObservableCollection<WorkflowEventModel>();
        });
        foreach (var node in Nodes)
            node.Status = WorkflowNodeStatus.Idle;
        IsLoading = false;
        StatusMessage = "Ready";
    }

    public async Task SaveSelectedWorkflow(JsonObject workflow)
    {
        if (SelectedWorkflowId is null) return;
        await _client.UpdateWorkflowAsync(SelectedWorkflowId, workflow);
        await SelectWorkflow(SelectedWorkflowId);
    }

    public async Task DeleteSelectedWorkflow()
    {
        if (SelectedWorkflowId is null) return;
        await _client.DeleteWorkflowAsync(SelectedWorkflowId);
        SelectedWorkflowId = null;
        Run(() => { Nodes.Clear(); Edges.Clear(); Events.Clear(); });
        await Refresh();
        if (Workflows.Count > 0)
            await SelectWorkflow(Workflows[0].WorkflowId);
    }

    public async Task Start(WorkflowStartOptions? options = null)
    {
        if (SelectedWorkflowId is null) return;
        var runId = await _client.StartWorkflowAsync(SelectedWorkflowId, options ?? new WorkflowStartOptions());
        Run(() => { RunId = runId; IsRunning = true; IsPaused = false; FirstFailedNodeId = null; });
        await PollEventsAsync(runId);
    }

    public async Task RunNode(string nodeId, JsonObject? inputOverrides = null)
    {
        if (SelectedWorkflowId is null) return;
        var opts = new WorkflowStartOptions { StartNodeId = nodeId };
        if (inputOverrides != null) opts.InputOverrides = inputOverrides;
        await Start(opts);
    }

    public async Task RestartFromFailure()
    {
        if (SelectedWorkflowId is null) return;
        var opts = new WorkflowStartOptions { ResumeFromFailure = true };
        if (RunId != null) opts.ResumeRunId = RunId;
        var runId = await _client.StartWorkflowAsync(SelectedWorkflowId, opts);
        Run(() => { RunId = runId; IsRunning = true; IsPaused = false; FirstFailedNodeId = null; });
        await PollEventsAsync(runId);
    }

    public async Task Stop()
    {
        if (RunId is null) return;
        await _client.StopWorkflowAsync(RunId);
    }

    public async Task ResumeStep()
    {
        if (RunId is null || !IsPaused) return;
        var newRunId = await _client.ResumeWorkflowAsync(RunId, new WorkflowStartOptions());
        Run(() => { RunId = newRunId; IsPaused = false; });
        await PollEventsAsync(newRunId);
    }

    public async Task StartStepMode()
    {
        if (SelectedWorkflowId is null) return;
        var opts = new WorkflowStartOptions { Mode = ExecutionMode.Step };
        await Start(opts);
    }

    private async Task PollEventsAsync(string runId)
    {
        int cursor = 0;
        while (true)
        {
            WorkflowEventsPage page;
            try { page = await _client.EventsAsync(runId, cursor); }
            catch { break; }

            ApplyEvents(page);
            cursor = page.NextCursor;

            if (page.Completed) break;
            await Task.Delay(300);
        }
        Run(() =>
        {
            IsRunning = false;
            if (IsPaused) StatusMessage = "Paused — step mode";
        });
    }

    private void ApplyEvents(WorkflowEventsPage page)
    {
        Run(() =>
        {
            IsResumable = page.Resumable;
            IsPaused = page.Paused;
            FirstFailedNodeId = page.FirstFailedNode;

            foreach (var evt in page.Events)
            {
                string evtType = evt["type"]?.GetValue<string>() ?? "";
                string? nodeId = evt["node_id"]?.GetValue<string>();
                string title = evtType.Replace("_", " ");
                string detail = evt["message"]?.GetValue<string>() ?? "";

                Events.Add(new WorkflowEventModel(DateTimeOffset.UtcNow, nodeId, title, detail));

                if (nodeId is not null)
                {
                    var node = Nodes.FirstOrDefault(n => n.Id == nodeId);
                    if (node is not null)
                    {
                        var status = evtType switch
                        {
                            "node_started"   => WorkflowNodeStatus.Running,
                            "node_succeeded" => WorkflowNodeStatus.Succeeded,
                            "node_failed"    => WorkflowNodeStatus.Failed,
                            "node_skipped"   => WorkflowNodeStatus.Skipped,
                            "node_paused"    => WorkflowNodeStatus.Paused,
                            "node_cancelled" => WorkflowNodeStatus.Cancelled,
                            _                => node.Status
                        };
                        node.Status = status;
                        if (evtType == "node_succeeded")
                        {
                            node.PreviousResult = node.Result;
                            node.Result = evt["output"]?.ToJsonString();
                        }
                        else if (evtType == "node_failed")
                        {
                            node.Result = evt["error"]?.GetValue<string>();
                        }
                    }
                }

                if (evtType == "workflow_completed")      StatusMessage = "Completed";
                else if (evtType == "workflow_failed")    StatusMessage = "Failed";
                else if (evtType == "workflow_cancelled") StatusMessage = "Cancelled";
                else if (evtType == "workflow_paused")    { StatusMessage = "Paused — step mode"; IsPaused = true; }
            }
        });
    }

    private async Task RunLiveEventsAsync(CancellationToken ct)
    {
        while (!ct.IsCancellationRequested)
        {
            try
            {
                await foreach (var liveEvent in _client.LiveEventsAsync(ct))
                {
                    if (liveEvent.Name == "workflow_changed")
                        _ = _dispatcher.TryEnqueue(async () => await Refresh());
                }
            }
            catch (OperationCanceledException) { break; }
            catch { await Task.Delay(2000, ct); }
        }
    }

    private void Run(Action action) => _dispatcher.TryEnqueue(() => action());

    public void Dispose()
    {
        _liveCts?.Cancel();
        _liveCts?.Dispose();
        _liveCts = null;
        (_client as IDisposable)?.Dispose();
    }
}
