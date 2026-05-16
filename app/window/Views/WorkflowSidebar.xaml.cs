using ManualWindow.Models;
using ManualWindow.Theme;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using System.Collections.ObjectModel;

namespace ManualWindow.Views;

public sealed partial class WorkflowSidebar : UserControl
{
    public static readonly DependencyProperty WorkflowsProperty =
        DependencyProperty.Register(nameof(Workflows), typeof(ObservableCollection<WorkflowSummary>),
            typeof(WorkflowSidebar), new PropertyMetadata(null, OnWorkflowsChanged));

    public static readonly DependencyProperty SelectedWorkflowIdProperty =
        DependencyProperty.Register(nameof(SelectedWorkflowId), typeof(string),
            typeof(WorkflowSidebar), new PropertyMetadata(null, OnSelectedChanged));

    public ObservableCollection<WorkflowSummary>? Workflows
    {
        get => (ObservableCollection<WorkflowSummary>?)GetValue(WorkflowsProperty);
        set => SetValue(WorkflowsProperty, value);
    }

    public string? SelectedWorkflowId
    {
        get => (string?)GetValue(SelectedWorkflowIdProperty);
        set => SetValue(SelectedWorkflowIdProperty, value);
    }

    public event Action<string>? WorkflowSelected;
    public event Action? RefreshRequested;

    public WorkflowSidebar()
    {
        InitializeComponent();
        RefreshIcon.Glyph = SymbolGlyphs.Refresh;
    }

    private static void OnWorkflowsChanged(DependencyObject d, DependencyPropertyChangedEventArgs e)
    {
        var view = (WorkflowSidebar)d;
        view.WorkflowList.ItemsSource = e.NewValue as ObservableCollection<WorkflowSummary>;
    }

    private static void OnSelectedChanged(DependencyObject d, DependencyPropertyChangedEventArgs e)
    {
        var view = (WorkflowSidebar)d;
        var id = e.NewValue as string;

        if (id is null)
        {
            view.WorkflowList.SelectedItem = null;
            return;
        }

        foreach (var item in view.WorkflowList.Items)
        {
            if (item is WorkflowSummary s && s.WorkflowId == id)
            {
                view.WorkflowList.SelectedItem = item;
                break;
            }
        }
    }

    private bool _suppressSelectionEvent;

    private void WorkflowList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_suppressSelectionEvent) return;
        if (WorkflowList.SelectedItem is WorkflowSummary selected)
            WorkflowSelected?.Invoke(selected.WorkflowId);
    }

    private void RefreshButton_Click(object sender, RoutedEventArgs e) => RefreshRequested?.Invoke();
}
