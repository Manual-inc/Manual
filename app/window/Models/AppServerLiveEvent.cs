using System.Text.Json.Nodes;

namespace ManualWindow.Models;

public sealed class AppServerLiveEvent
{
    public string Name { get; }
    public JsonObject Payload { get; }

    public AppServerLiveEvent(string name, JsonObject payload)
    {
        Name = name; Payload = payload;
    }
}
