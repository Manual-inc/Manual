using Windows.Storage;

namespace ManualWindow.Stores;

public sealed class UiPreferencesStore
{
    private static ApplicationDataContainer Settings =>
        ApplicationData.Current.LocalSettings;

    public bool SidebarVisible
    {
        get => Settings.Values["ManualWindow.sidebarVisible"] as bool? ?? true;
        set => Settings.Values["ManualWindow.sidebarVisible"] = value;
    }

    public bool InspectorVisible
    {
        get => Settings.Values["ManualWindow.inspectorVisible"] as bool? ?? true;
        set => Settings.Values["ManualWindow.inspectorVisible"] = value;
    }

    public bool BottomPanelVisible
    {
        get => Settings.Values["ManualWindow.bottomPanelVisible"] as bool? ?? true;
        set => Settings.Values["ManualWindow.bottomPanelVisible"] = value;
    }
}
