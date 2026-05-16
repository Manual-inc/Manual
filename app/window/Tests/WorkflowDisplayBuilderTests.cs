using System.Text.Json.Nodes;
using ManualWindow.Models;
using Xunit;

namespace ManualWindow.Tests;

public class WorkflowDisplayBuilderTests
{
    [Fact]
    public void ThreeNodeLinearChain_BuildsCorrectly()
    {
        var workflow = JsonNode.Parse("""
            {
              "id": "test",
              "nodes": [
                { "id": "a", "kind": "constant" },
                { "id": "b", "kind": "pi" },
                { "id": "c", "kind": "template" }
              ],
              "dependencies": [
                { "node": "b", "depends_on": "a" },
                { "node": "c", "depends_on": "b" }
              ]
            }
            """)!.AsObject();

        var display = WorkflowDisplayBuilder.Build(workflow);

        Assert.Equal(3, display.Nodes.Count);
        Assert.Equal(2, display.Edges.Count);

        // Leftmost node (a) should have smallest x
        var nodeA = display.Nodes.First(n => n.Id == "a");
        var nodeC = display.Nodes.First(n => n.Id == "c");
        Assert.True(nodeA.Position.X < nodeC.Position.X);
    }
}
