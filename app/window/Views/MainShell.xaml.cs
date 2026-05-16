using ManualWindow.Models;
using ManualWindow.Stores;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading.Tasks;

namespace ManualWindow.Views;

public sealed partial class MainShell : UserControl
{
    private WorkflowRunStore? _store;
    private UiPreferencesStore? _prefs;
    private XamlRoot? _xamlRoot;

    public MainShell() { InitializeComponent(); Loaded += OnLoaded; }

    public void Initialize(WorkflowRunStore store, UiPreferencesStore prefs)
    {
        _store = store;
        _prefs = prefs;
        _xamlRoot = XamlRoot;

        // Bind store → views
        BindStore();

        // Subscribe to store property changes
        store.PropertyChanged += (_, e) => UpdateFromStore(e.PropertyName);

        // Wire up view events
        WireEvents();

        // Restore panel visibility from prefs
        ApplyPrefs();
    }

    private void OnLoaded(object sender, RoutedEventArgs e)
    {
        _xamlRoot = XamlRoot;
    }

    // ── Store → View binding ────────────────────────────────────────────────

    private void BindStore()
    {
        if (_store is null) return;
        Sidebar.Workflows = _store.Workflows;
        Sidebar.SelectedWorkflowId = _store.SelectedWorkflowId;
        GraphView.Nodes = _store.Nodes;
        GraphView.Edges = _store.Edges;
        GraphView.IsRunning = _store.IsRunning;
        BottomPanelView.Events = _store.Events;
        TopBar.WorkflowId = _store.SelectedWorkflowId;
        TopBar.IsRunning = _store.IsRunning;
        TopBar.IsPaused = _store.IsPaused;
        TopBar.StatusMessage = _store.StatusMessage;
        UpdateInspector();
    }

    private void UpdateFromStore(string? propName)
    {
        if (_store is null) return;
        switch (propName)
        {
            case nameof(WorkflowRunStore.Nodes):
                GraphView.Nodes = _store.Nodes;
                UpdateInspector();
                break;
            case nameof(WorkflowRunStore.Edges):
                GraphView.Edges = _store.Edges;
                break;
            case nameof(WorkflowRunStore.IsRunning):
                GraphView.IsRunning = _store.IsRunning;
                TopBar.IsRunning = _store.IsRunning;
                UpdateInspector();
                break;
            case nameof(WorkflowRunStore.IsPaused):
                TopBar.IsPaused = _store.IsPaused;
                break;
            case nameof(WorkflowRunStore.StatusMessage):
                TopBar.StatusMessage = _store.StatusMessage;
                break;
            case nameof(WorkflowRunStore.SelectedWorkflowId):
                Sidebar.SelectedWorkflowId = _store.SelectedWorkflowId;
                TopBar.WorkflowId = _store.SelectedWorkflowId;
                break;
            case nameof(WorkflowRunStore.SelectedNodeId):
                GraphView.SelectedNodeId = _store.SelectedNodeId;
                UpdateInspector();
                break;
            case nameof(WorkflowRunStore.Workflows):
                Sidebar.Workflows = _store.Workflows;
                break;
        }
    }

    private void UpdateInspector()
    {
        if (_store is null) return;
        var node = _store.SelectedNodeId is not null
            ? _store.Nodes.FirstOrDefault(n => n.Id == _store.SelectedNodeId)
            : null;
        Inspector.Node = node;
        Inspector.IsWorkflowRunning = _store.IsRunning;
    }

    // ── Event wiring ────────────────────────────────────────────────────────

    private void WireEvents()
    {
        // GraphView events
        GraphView.NodeSelected += id =>
        {
            if (_store is null) return;
            _store.SelectedNodeId = _store.SelectedNodeId == id ? null : id;
        };
        GraphView.NodeRun      += id => RunAsync(() => _store?.RunNode(id));
        GraphView.NodeRestart  += id => RunAsync(() => _store?.RestartFromFailure());
        GraphView.NodeStop     += () => RunAsync(() => _store?.Stop());
        GraphView.NodeOverride += id => _ = ShowOverrideDialogAsync(id);

        // Inspector events
        Inspector.RunRequested      += id => RunAsync(() => _store?.RunNode(id));
        Inspector.RestartRequested  += () => RunAsync(() => _store?.RestartFromFailure());
        Inspector.StopRequested     += () => RunAsync(() => _store?.Stop());
        Inspector.OverrideRequested += id => _ = ShowOverrideDialogAsync(id);

        // TopBar events
        TopBar.ExecuteRequested  += () => RunAsync(() => _store?.Start());
        TopBar.StepModeRequested += () => RunAsync(() => _store?.StartStepMode());
        TopBar.StopRequested     += () => RunAsync(() => _store?.Stop());
        TopBar.NextStepRequested += () => RunAsync(() => _store?.ResumeStep());

        // Sidebar events
        Sidebar.WorkflowSelected += id => RunAsync(() => _store?.SelectWorkflow(id));
        Sidebar.RefreshRequested += () => RunAsync(() => _store?.Refresh());

        // BottomPanel
        BottomPanelView.ClearRequested += () => _store?.Events.Clear();
    }

    // ── Override dialog ─────────────────────────────────────────────────────

    private async Task ShowOverrideDialogAsync(string nodeId)
    {
        if (_store is null || _xamlRoot is null) return;
        var node = _store.Nodes.FirstOrDefault(n => n.Id == nodeId);
        var dialog = new NodeInputOverrideDialog { XamlRoot = _xamlRoot };
        dialog.SetInitialJson(node?.InputOverride);

        var result = await dialog.ShowAsync();
        if (result == ContentDialogResult.Primary && dialog.Result is not null)
        {
            if (node is not null) node.InputOverride = dialog.Result;
            await (_store.RunNode(nodeId, dialog.Result) ?? Task.CompletedTask);
        }
    }

    // ── Panel toggles ───────────────────────────────────────────────────────

    private void ApplyPrefs()
    {
        if (_prefs is null) return;
        SetSidebarVisible(_prefs.SidebarVisible);
        SetInspectorVisible(_prefs.InspectorVisible);
        SetBottomVisible(_prefs.BottomPanelVisible);
    }

    private void ToggleSidebar_Click(object s, RoutedEventArgs e)
    {
        bool newVal = SidebarColumn.Width.Value == 0;
        SetSidebarVisible(newVal);
        if (_prefs is not null) _prefs.SidebarVisible = newVal;
    }

    private void ToggleInspector_Click(object s, RoutedEventArgs e)
    {
        bool newVal = InspectorColumn.Width.Value == 0;
        SetInspectorVisible(newVal);
        if (_prefs is not null) _prefs.InspectorVisible = newVal;
    }

    private void ToggleBottom_Click(object s, RoutedEventArgs e)
    {
        bool newVal = BottomRow.Height.Value == 0;
        SetBottomVisible(newVal);
        if (_prefs is not null) _prefs.BottomPanelVisible = newVal;
    }

    private void SetSidebarVisible(bool visible)
        => SidebarColumn.Width = new GridLength(visible ? 240 : 0);

    private void SetInspectorVisible(bool visible)
        => InspectorColumn.Width = new GridLength(visible ? 320 : 0);

    private void SetBottomVisible(bool visible)
        => BottomRow.Height = new GridLength(visible ? 200 : 0);

    // ── Helpers ─────────────────────────────────────────────────────────────

    private static void RunAsync(Func<Task?> action)
        => _ = Task.Run(async () =>
        {
            var t = action();
            if (t is not null) await t;
        });
}
