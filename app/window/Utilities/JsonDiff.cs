using System.Text.Json;
using System.Text.Json.Nodes;

namespace ManualWindow.Utilities;

public enum DiffOp { Same, Added, Removed }

public record DiffLine(DiffOp Op, string Text);

public static class JsonDiff
{
    public static IReadOnlyList<DiffLine> Diff(string? oldText, string? newText)
    {
        var oldLines = SplitLines(oldText);
        var newLines = SplitLines(newText);
        return Lcs(oldLines, newLines);
    }

    public static string? PrettyJson(JsonNode? value)
    {
        if (value is null) return null;
        return value.ToJsonString(new JsonSerializerOptions { WriteIndented = true });
    }

    private static string[] SplitLines(string? text)
    {
        if (string.IsNullOrEmpty(text)) return Array.Empty<string>();
        return text.Split('\n');
    }

    private static IReadOnlyList<DiffLine> Lcs(string[] oldLines, string[] newLines)
    {
        int m = oldLines.Length, n = newLines.Length;
        var dp = new int[m + 1, n + 1];

        for (int i = 1; i <= m; i++)
            for (int j = 1; j <= n; j++)
                dp[i, j] = oldLines[i - 1] == newLines[j - 1]
                    ? dp[i - 1, j - 1] + 1
                    : Math.Max(dp[i - 1, j], dp[i, j - 1]);

        var result = new List<DiffLine>();
        int r = m, c = n;
        while (r > 0 || c > 0)
        {
            if (r > 0 && c > 0 && oldLines[r - 1] == newLines[c - 1])
            {
                result.Add(new DiffLine(DiffOp.Same, oldLines[r - 1]));
                r--; c--;
            }
            else if (c > 0 && (r == 0 || dp[r, c - 1] >= dp[r - 1, c]))
            {
                result.Add(new DiffLine(DiffOp.Added, newLines[c - 1]));
                c--;
            }
            else
            {
                result.Add(new DiffLine(DiffOp.Removed, oldLines[r - 1]));
                r--;
            }
        }
        result.Reverse();
        return result;
    }
}
