using ManualWindow.Models;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace ManualWindow.Views.Graph;

public sealed partial class NodeActionOverlay : UserControl
{
    public static readonly DependencyProperty NodeStatusProperty =
        DependencyProperty.Register(nameof(NodeStatus), typeof(WorkflowNodeStatus),
            typeof(NodeActionOverlay), new PropertyMetadata(WorkflowNodeStatus.Idle, OnStateChanged));

    public static readonly DependencyProperty IsWorkflowRunningProperty =
        DependencyProperty.Register(nameof(IsWorkflowRunning), typeof(bool),
            typeof(NodeActionOverlay), new PropertyMetadata(false, OnStateChanged));

    public WorkflowNodeStatus NodeStatus
    {
        get => (WorkflowNodeStatus)GetValue(NodeStatusProperty);
        set => SetValue(NodeStatusProperty, value);
    }

    public bool IsWorkflowRunning
    {
        get => (bool)GetValue(IsWorkflowRunningProperty);
        set => SetValue(IsWorkflowRunningProperty, value);
    }

    public event RoutedEventHandler? RunClicked;
    public event RoutedEventHandler? RestartClicked;
    public event RoutedEventHandler? StopClicked;
    public event RoutedEventHandler? OverrideClicked;

    public NodeActionOverlay() { InitializeComponent(); UpdateVisibility(); }

    private static void OnStateChanged(DependencyObject d, DependencyPropertyChangedEventArgs e)
        => ((NodeActionOverlay)d).UpdateVisibility();

    private void UpdateVisibility()
    {
        var s = NodeStatus;
        RunButton.Visibility      = (s == WorkflowNodeStatus.Idle || s == WorkflowNodeStatus.Succeeded ||
                                     s == WorkflowNodeStatus.Skipped || s == WorkflowNodeStatus.Cancelled)
                                    ? Visibility.Visible : Visibility.Collapsed;
        RestartButton.Visibility  = s == WorkflowNodeStatus.Failed ? Visibility.Visible : Visibility.Collapsed;
        StopButton.Visibility     = (s == WorkflowNodeStatus.Running && IsWorkflowRunning)
                                    ? Visibility.Visible : Visibility.Collapsed;
        OverrideButton.Visibility = s != WorkflowNodeStatus.Running ? Visibility.Visible : Visibility.Collapsed;
    }

    private void RunButton_Click(object sender, RoutedEventArgs e)      => RunClicked?.Invoke(this, e);
    private void RestartButton_Click(object sender, RoutedEventArgs e)  => RestartClicked?.Invoke(this, e);
    private void StopButton_Click(object sender, RoutedEventArgs e)     => StopClicked?.Invoke(this, e);
    private void OverrideButton_Click(object sender, RoutedEventArgs e) => OverrideClicked?.Invoke(this, e);
}
