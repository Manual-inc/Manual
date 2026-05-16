using ManualWindow.Models;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using System.Collections.ObjectModel;
using System.Collections.Specialized;

namespace ManualWindow.Views;

// Thin view-model wrapper so DataTemplate can bind TimeLabel without a converter.
internal sealed class EventTimelineItem
{
    public string TimeLabel { get; }
    public string? NodeId { get; }
    public string Title { get; }
    public string Detail { get; }

    public EventTimelineItem(WorkflowEventModel m)
    {
        TimeLabel = m.Time.LocalDateTime.ToString("HH:mm:ss");
        NodeId    = m.NodeId;
        Title     = m.Title;
        Detail    = m.Detail;
    }
}

public sealed partial class EventTimelineView : UserControl
{
    public static readonly DependencyProperty EventsProperty =
        DependencyProperty.Register(nameof(Events), typeof(ObservableCollection<WorkflowEventModel>),
            typeof(EventTimelineView), new PropertyMetadata(null, OnEventsChanged));

    public ObservableCollection<WorkflowEventModel>? Events
    {
        get => (ObservableCollection<WorkflowEventModel>?)GetValue(EventsProperty);
        set => SetValue(EventsProperty, value);
    }

    private readonly ObservableCollection<EventTimelineItem> _items = [];

    public EventTimelineView()
    {
        InitializeComponent();
        EventList.ItemsSource = _items;
    }

    private static void OnEventsChanged(DependencyObject d, DependencyPropertyChangedEventArgs e)
    {
        var view = (EventTimelineView)d;

        if (e.OldValue is ObservableCollection<WorkflowEventModel> old)
            old.CollectionChanged -= view.OnCollectionChanged;

        view._items.Clear();

        if (e.NewValue is ObservableCollection<WorkflowEventModel> newColl)
        {
            foreach (var m in newColl)
                view._items.Add(new EventTimelineItem(m));

            newColl.CollectionChanged += view.OnCollectionChanged;
        }

        view.UpdateVisibility();
    }

    private void OnCollectionChanged(object? sender, NotifyCollectionChangedEventArgs e)
    {
        if (e.Action == NotifyCollectionChangedAction.Add && e.NewItems is not null)
        {
            foreach (WorkflowEventModel m in e.NewItems)
                _items.Add(new EventTimelineItem(m));
        }
        else if (e.Action == NotifyCollectionChangedAction.Reset)
        {
            _items.Clear();
        }

        UpdateVisibility();

        if (_items.Count > 0)
            EventList.ScrollIntoView(_items[^1]);
    }

    private void UpdateVisibility()
    {
        bool hasEvents = _items.Count > 0;
        EmptyText.Visibility = hasEvents ? Visibility.Collapsed : Visibility.Visible;
        EventList.Visibility = hasEvents ? Visibility.Visible   : Visibility.Collapsed;
    }
}
