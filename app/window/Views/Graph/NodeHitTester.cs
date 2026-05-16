using ManualWindow.Models;

namespace ManualWindow.Views.Graph;

public static class NodeHitTester
{
    public static string? HitTest(
        double worldX, double worldY,
        IReadOnlyList<WorkflowNodeModel> nodes,
        double canvasWidth, double canvasHeight)
    {
        foreach (var node in nodes)
        {
            var (l, t, r, b) = GraphLayoutMath.NodeRect(node.Position, canvasWidth, canvasHeight);
            if (worldX >= l && worldX <= r && worldY >= t && worldY <= b)
                return node.Id;
        }
        return null;
    }

    // Convert screen coordinates to canvas world coordinates given pan and zoom
    public static (double WorldX, double WorldY) ScreenToWorld(
        double screenX, double screenY,
        double panX, double panY,
        double zoom,
        double viewCenterX, double viewCenterY)
    {
        // Screen → world: reverse the scale-then-translate applied during draw
        double worldX = (screenX - viewCenterX - panX) / zoom + viewCenterX;
        double worldY = (screenY - viewCenterY - panY) / zoom + viewCenterY;
        return (worldX, worldY);
    }
}
