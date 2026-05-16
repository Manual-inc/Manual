using Windows.UI;
using ManualWindow.Models;

namespace ManualWindow.Theme;

public static class AppTheme
{
    // Background layers
    public static Color Canvas        { get; } = Color.FromArgb(255, 41, 43, 54);
    public static Color CanvasGrid    { get; } = Color.FromArgb(15, 255, 255, 255);
    public static Color Panel         { get; } = Color.FromArgb(255, 33, 36, 43);
    public static Color PanelElev     { get; } = Color.FromArgb(255, 46, 48, 59);
    public static Color Rail          { get; } = Color.FromArgb(255, 26, 28, 33);
    public static Color TopBar        { get; } = Color.FromArgb(255, 33, 36, 43);

    // Node
    public static Color NodeCard      { get; } = Color.FromArgb(255, 54, 56, 69);
    public static Color NodeStroke    { get; } = Color.FromArgb(26, 255, 255, 255);

    // Borders
    public static Color Stroke        { get; } = Color.FromArgb(20, 255, 255, 255);
    public static Color StrokeStrong  { get; } = Color.FromArgb(41, 255, 255, 255);

    // Text
    public static Color Text          { get; } = Color.FromArgb(235, 255, 255, 255);
    public static Color TextMuted     { get; } = Color.FromArgb(140, 255, 255, 255);
    public static Color TextFaint     { get; } = Color.FromArgb(82, 255, 255, 255);

    // Accent
    public static Color Accent        { get; } = Color.FromArgb(255, 255, 110, 92);
    public static Color AccentMuted   { get; } = Color.FromArgb(46, 255, 110, 92);

    // Graph edges
    public static Color Edge          { get; } = Color.FromArgb(51, 255, 255, 255);
    public static Color EdgeActive    { get; } = Color.FromArgb(255, 255, 110, 92);

    public static Color StatusColor(WorkflowNodeStatus status) => status switch
    {
        WorkflowNodeStatus.Idle      => Color.FromArgb(115, 255, 255, 255),
        WorkflowNodeStatus.Running   => Color.FromArgb(255, 102, 166, 255),
        WorkflowNodeStatus.Succeeded => Color.FromArgb(255, 107, 217, 128),
        WorkflowNodeStatus.Failed    => Color.FromArgb(255, 245, 115, 115),
        WorkflowNodeStatus.Skipped   => Color.FromArgb(77, 255, 255, 255),
        WorkflowNodeStatus.Paused    => Color.FromArgb(255, 255, 204, 77),
        WorkflowNodeStatus.Cancelled => Color.FromArgb(64, 255, 255, 255),
        _                            => Color.FromArgb(115, 255, 255, 255),
    };

    public static Color KindColor(WorkflowNodeKind kind) => kind switch
    {
        WorkflowNodeKind.Context => Color.FromArgb(255, 128, 158, 255),
        WorkflowNodeKind.Script  => Color.FromArgb(255, 255, 140, 77),
        WorkflowNodeKind.Agent   => Color.FromArgb(255, 237, 99, 181),
        WorkflowNodeKind.Claude  => Color.FromArgb(255, 191, 148, 255),
        WorkflowNodeKind.Codex   => Color.FromArgb(255, 102, 189, 255),
        WorkflowNodeKind.Digest  => Color.FromArgb(255, 107, 217, 128),
        _                        => Color.FromArgb(255, 128, 158, 255),
    };
}
