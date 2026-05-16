using ManualWindow.Models;

namespace ManualWindow.Views.Graph;

public static class GraphLayoutMath
{
    // Node card size in logical pixels (matches macOS 230×96)
    public const double NodeWidth  = 230.0;
    public const double NodeHeight = 96.0;

    // Convert fractional position (0–1) to canvas-space center point
    public static (double X, double Y) NodeCenter(WorkflowNodePosition pos, double canvasWidth, double canvasHeight)
        => (pos.X * canvasWidth, pos.Y * canvasHeight);

    // Right-edge midpoint of a node (edge start)
    public static (double X, double Y) RightPort(WorkflowNodePosition pos, double canvasWidth, double canvasHeight)
    {
        var (cx, cy) = NodeCenter(pos, canvasWidth, canvasHeight);
        return (cx + NodeWidth / 2, cy);
    }

    // Left-edge midpoint of a node (edge end)
    public static (double X, double Y) LeftPort(WorkflowNodePosition pos, double canvasWidth, double canvasHeight)
    {
        var (cx, cy) = NodeCenter(pos, canvasWidth, canvasHeight);
        return (cx - NodeWidth / 2, cy);
    }

    // Bezier control points for a curved edge
    public static ((double X, double Y) C1, (double X, double Y) C2) BezierControls(
        (double X, double Y) start,
        (double X, double Y) end)
    {
        double dx = Math.Max(60, Math.Abs(end.X - start.X) * 0.45);
        return ((start.X + dx, start.Y), (end.X - dx, end.Y));
    }

    // Bounding rect for a node card given its center
    public static (double Left, double Top, double Right, double Bottom) NodeRect(
        WorkflowNodePosition pos, double canvasWidth, double canvasHeight)
    {
        var (cx, cy) = NodeCenter(pos, canvasWidth, canvasHeight);
        return (cx - NodeWidth / 2, cy - NodeHeight / 2,
                cx + NodeWidth / 2, cy + NodeHeight / 2);
    }

    // Clamp zoom level
    public static double ClampZoom(double zoom) => Math.Clamp(zoom, 0.4, 2.0);
}
