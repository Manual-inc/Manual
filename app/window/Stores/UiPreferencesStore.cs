using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;

namespace ManualWindow.Stores;

public sealed class UiPreferencesStore
{
    private static readonly string FilePath = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
        "Manual", "prefs.json");

    private bool _sidebarVisible     = true;
    private bool _inspectorVisible   = true;
    private bool _bottomPanelVisible = true;

    public UiPreferencesStore()
    {
        try
        {
            if (File.Exists(FilePath))
            {
                var dict = JsonSerializer.Deserialize<Dictionary<string, bool>>(
                    File.ReadAllText(FilePath));
                if (dict is not null)
                {
                    _sidebarVisible     = dict.GetValueOrDefault("sidebarVisible",     true);
                    _inspectorVisible   = dict.GetValueOrDefault("inspectorVisible",   true);
                    _bottomPanelVisible = dict.GetValueOrDefault("bottomPanelVisible", true);
                }
            }
        }
        catch { }
    }

    public bool SidebarVisible
    {
        get => _sidebarVisible;
        set { _sidebarVisible = value; Save(); }
    }

    public bool InspectorVisible
    {
        get => _inspectorVisible;
        set { _inspectorVisible = value; Save(); }
    }

    public bool BottomPanelVisible
    {
        get => _bottomPanelVisible;
        set { _bottomPanelVisible = value; Save(); }
    }

    private void Save()
    {
        try
        {
            Directory.CreateDirectory(Path.GetDirectoryName(FilePath)!);
            File.WriteAllText(FilePath, JsonSerializer.Serialize(new Dictionary<string, bool>
            {
                ["sidebarVisible"]     = _sidebarVisible,
                ["inspectorVisible"]   = _inspectorVisible,
                ["bottomPanelVisible"] = _bottomPanelVisible,
            }));
        }
        catch { }
    }
}
