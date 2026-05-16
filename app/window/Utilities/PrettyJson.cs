using System.Text.Json;
using System.Text.Json.Nodes;

namespace ManualWindow.Utilities;

public static class PrettyJson
{
    private static readonly JsonSerializerOptions PrettyOptions = new()
    {
        WriteIndented = true,
    };

    public static string Stringify(JsonNode? node)
    {
        if (node is null) return "{}";
        return node.ToJsonString(PrettyOptions);
    }

    public static string Stringify(object? value)
    {
        if (value is null) return "{}";
        try { return JsonSerializer.Serialize(value, PrettyOptions); }
        catch { return value.ToString() ?? "{}"; }
    }
}
