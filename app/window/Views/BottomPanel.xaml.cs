using ManualWindow.Models;
using ManualWindow.Theme;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using System.Collections.ObjectModel;

namespace ManualWindow.Views;

public sealed partial class BottomPanel : UserControl
{
    public static readonly DependencyProperty EventsProperty =
        DependencyProperty.Register(nameof(Events), typeof(ObservableCollection<WorkflowEventModel>),
            typeof(BottomPanel), new PropertyMetadata(null,
                (d, e) => ((BottomPanel)d).Timeline.Events =
                    e.NewValue as ObservableCollection<WorkflowEventModel>));

    public ObservableCollection<WorkflowEventModel>? Events
    {
        get => (ObservableCollection<WorkflowEventModel>?)GetValue(EventsProperty);
        set => SetValue(EventsProperty, value);
    }

    public event Action? ClearRequested;

    public BottomPanel()
    {
        InitializeComponent();
        ClearIcon.Glyph = SymbolGlyphs.Delete;
    }

    private void Clear_Click(object s, RoutedEventArgs e) => ClearRequested?.Invoke();
}
