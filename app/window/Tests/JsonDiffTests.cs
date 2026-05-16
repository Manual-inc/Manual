using ManualWindow.Utilities;
using Xunit;

namespace ManualWindow.Tests;

public class JsonDiffTests
{
    [Fact]
    public void IdenticalStrings_AllSame()
    {
        var lines = JsonDiff.Diff("a\nb\nc", "a\nb\nc");
        Assert.All(lines, l => Assert.Equal(DiffOp.Same, l.Op));
        Assert.Equal(3, lines.Count);
    }

    [Fact]
    public void AddedLine_DetectedCorrectly()
    {
        var lines = JsonDiff.Diff("a\nb", "a\nb\nc");
        var added = lines.Where(l => l.Op == DiffOp.Added).ToList();
        Assert.Single(added);
        Assert.Equal("c", added[0].Text);
    }

    [Fact]
    public void RemovedLine_DetectedCorrectly()
    {
        var lines = JsonDiff.Diff("a\nb\nc", "a\nb");
        var removed = lines.Where(l => l.Op == DiffOp.Removed).ToList();
        Assert.Single(removed);
        Assert.Equal("c", removed[0].Text);
    }

    [Fact]
    public void NullInputs_ProduceEmpty()
    {
        var lines = JsonDiff.Diff(null, null);
        Assert.Empty(lines);
    }

    [Fact]
    public void EmptyOld_AllAdded()
    {
        var lines = JsonDiff.Diff(null, "x\ny");
        Assert.Equal(2, lines.Count(l => l.Op == DiffOp.Added));
        Assert.Empty(lines.Where(l => l.Op == DiffOp.Removed));
    }
}
