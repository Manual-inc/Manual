using ManualWindow.Models;
using ManualWindow.Theme;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Windows.UI;

namespace ManualWindow.Views;

public sealed partial class NodeInspectorView : UserControl
{
    // DependencyProperty: Node (WorkflowNodeModel?)
    public static readonly DependencyProperty NodeProperty =
        DependencyProperty.Register(nameof(Node), typeof(WorkflowNodeModel),
            typeof(NodeInspectorView), new PropertyMetadata(null, OnNodeChanged));

    // DependencyProperty: IsWorkflowRunning (bool)
    public static readonly DependencyProperty IsWorkflowRunningProperty =
        DependencyProperty.Register(nameof(IsWorkflowRunning), typeof(bool),
            typeof(NodeInspectorView), new PropertyMetadata(false, OnNodeChanged));

    public WorkflowNodeModel? Node
    {
        get => (WorkflowNodeModel?)GetValue(NodeProperty);
        set => SetValue(NodeProperty, value);
    }

    public bool IsWorkflowRunning
    {
        get => (bool)GetValue(IsWorkflowRunningProperty);
        set => SetValue(IsWorkflowRunningProperty, value);
    }

    // Events that bubble up to the shell
    public event Action<string>? RunRequested;       // nodeId
    public event Action? RestartRequested;
    public event Action? StopRequested;
    public event Action<string>? OverrideRequested;  // nodeId

    public NodeInspectorView() { InitializeComponent(); }

    private static void OnNodeChanged(DependencyObject d, DependencyPropertyChangedEventArgs e)
        => ((NodeInspectorView)d).Refresh();

    private void Refresh()
    {
        var node = Node;
        if (node is null)
        {
            PlaceholderText.Visibility = Visibility.Visible;
            ContentPanel.Visibility    = Visibility.Collapsed;
            return;
        }

        PlaceholderText.Visibility = Visibility.Collapsed;
        ContentPanel.Visibility    = Visibility.Visible;

        NodeTitleText.Text = node.Title;
        NodeKindText.Text  = node.Kind.DisplayName().ToUpperInvariant();
        NodeIdText.Text    = node.Id;

        // Status icon + label
        StatusIcon.FontFamily = new FontFamily(SymbolGlyphs.FontFamily);
        (StatusIcon.Glyph, StatusIcon.Foreground, StatusText.Text) = node.Status switch
        {
            WorkflowNodeStatus.Idle      => (SymbolGlyphs.SlashCircle,  Brush(AppTheme.TextFaint),                          "Idle"),
            WorkflowNodeStatus.Running   => (SymbolGlyphs.Hourglass,    Brush(AppTheme.StatusColor(node.Status)),            "Running"),
            WorkflowNodeStatus.Succeeded => (SymbolGlyphs.CheckCircle,  Brush(AppTheme.StatusColor(node.Status)),            "Succeeded"),
            WorkflowNodeStatus.Failed    => (SymbolGlyphs.ErrorCircle,  Brush(AppTheme.StatusColor(node.Status)),            "Failed"),
            WorkflowNodeStatus.Skipped   => (SymbolGlyphs.SkipCircle,   Brush(AppTheme.TextMuted),                          "Skipped"),
            WorkflowNodeStatus.Paused    => (SymbolGlyphs.PauseCircle,  Brush(AppTheme.StatusColor(node.Status)),            "Paused"),
            WorkflowNodeStatus.Cancelled => (SymbolGlyphs.SlashCircle,  Brush(AppTheme.TextMuted),                          "Cancelled"),
            _                            => (SymbolGlyphs.SlashCircle,  Brush(AppTheme.TextMuted),                          node.Status.ToString()),
        };
        StatusText.Foreground = StatusIcon.Foreground;

        // Actions visibility
        bool canRun    = node.Status is WorkflowNodeStatus.Idle
                                     or WorkflowNodeStatus.Succeeded
                                     or WorkflowNodeStatus.Skipped
                                     or WorkflowNodeStatus.Cancelled;
        bool isFailed  = node.Status == WorkflowNodeStatus.Failed;
        bool isRunning = node.Status == WorkflowNodeStatus.Running;

        RunPanel.Visibility      = Show(canRun);
        RestartPanel.Visibility  = Show(isFailed);
        StopPanel.Visibility     = Show(isRunning && IsWorkflowRunning);
        OverridePanel.Visibility = Show(node.Status != WorkflowNodeStatus.Running);

        // Output
        bool hasResult = node.Result is not null;
        OutputPanel.Visibility = Show(hasResult);
        if (hasResult) OutputText.Text = node.Result;

        bool hasPrev = node.Result is not null && node.PreviousResult is not null;
        PreviousOutputPanel.Visibility = Show(hasPrev);
        if (hasPrev)
        {
            PreviousOutputText.Text = node.PreviousResult;
            CurrentOutputText.Text  = node.Result;
        }
    }

    private static Visibility Show(bool v) => v ? Visibility.Visible : Visibility.Collapsed;

    private static SolidColorBrush Brush(Color c) => new(c);

    private void RunButton_Click(object s, RoutedEventArgs e)
    {
        if (Node is not null) RunRequested?.Invoke(Node.Id);
    }

    private void RestartButton_Click(object s, RoutedEventArgs e) => RestartRequested?.Invoke();

    private void StopButton_Click(object s, RoutedEventArgs e) => StopRequested?.Invoke();

    private void OverrideButton_Click(object s, RoutedEventArgs e)
    {
        if (Node is not null) OverrideRequested?.Invoke(Node.Id);
    }
}
