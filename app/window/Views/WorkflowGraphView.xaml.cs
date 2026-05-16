using ManualWindow.Models;
using ManualWindow.Theme;
using ManualWindow.Views.Graph;
using Microsoft.Graphics.Canvas;
using Microsoft.Graphics.Canvas.Geometry;
using Microsoft.Graphics.Canvas.Text;
using Microsoft.Graphics.Canvas.UI.Xaml;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using System;
using System.Collections.Generic;
using System.Linq;
using System.Numerics;
using Windows.Foundation;
using Windows.UI;

namespace ManualWindow.Views;

public sealed partial class WorkflowGraphView : UserControl
{
    // ── Dependency Properties ──────────────────────────────────────────────

    public static readonly DependencyProperty NodesProperty =
        DependencyProperty.Register(nameof(Nodes), typeof(IReadOnlyList<WorkflowNodeModel>),
            typeof(WorkflowGraphView), new PropertyMetadata(null, OnGraphDataChanged));

    public static readonly DependencyProperty EdgesProperty =
        DependencyProperty.Register(nameof(Edges), typeof(IReadOnlyList<WorkflowEdgeModel>),
            typeof(WorkflowGraphView), new PropertyMetadata(null, OnGraphDataChanged));

    public static readonly DependencyProperty SelectedNodeIdProperty =
        DependencyProperty.Register(nameof(SelectedNodeId), typeof(string),
            typeof(WorkflowGraphView), new PropertyMetadata(null, OnSelectionChanged));

    public static readonly DependencyProperty IsRunningProperty =
        DependencyProperty.Register(nameof(IsRunning), typeof(bool),
            typeof(WorkflowGraphView), new PropertyMetadata(false, OnGraphDataChanged));

    public IReadOnlyList<WorkflowNodeModel>? Nodes
    {
        get => (IReadOnlyList<WorkflowNodeModel>?)GetValue(NodesProperty);
        set => SetValue(NodesProperty, value);
    }

    public IReadOnlyList<WorkflowEdgeModel>? Edges
    {
        get => (IReadOnlyList<WorkflowEdgeModel>?)GetValue(EdgesProperty);
        set => SetValue(EdgesProperty, value);
    }

    public string? SelectedNodeId
    {
        get => (string?)GetValue(SelectedNodeIdProperty);
        set => SetValue(SelectedNodeIdProperty, value);
    }

    public bool IsRunning
    {
        get => (bool)GetValue(IsRunningProperty);
        set => SetValue(IsRunningProperty, value);
    }

    // ── Events ────────────────────────────────────────────────────────────

    public event Action<string>? NodeSelected;
    public event Action<string>? NodeRun;
    public event Action<string>? NodeRestart;
    public event Action? NodeStop;
    public event Action<string>? NodeOverride;

    // ── Private state ─────────────────────────────────────────────────────

    private double _zoom = 1.0, _panX, _panY;
    private double _dragStartX, _dragStartY, _panStartX, _panStartY;
    private bool _isDragging;
    private string? _hoveredNodeId;
    private float _dashPhase;
    private DispatcherTimer? _animTimer;
    private NodeActionOverlay? _actionOverlay;
    private string? _overlayNodeId;

    // ── Constructor ───────────────────────────────────────────────────────

    public WorkflowGraphView()
    {
        InitializeComponent();
        Surface.PointerMoved        += Surface_PointerMoved;
        Surface.PointerPressed      += Surface_PointerPressed;
        Surface.PointerReleased     += Surface_PointerReleased;
        Surface.PointerWheelChanged += Surface_PointerWheelChanged;
        Surface.PointerExited       += Surface_PointerExited;
        Loaded   += (_, _) => StartAnimation();
        Unloaded += (_, _) => StopAnimation();
    }

    // ── Property-change callbacks ─────────────────────────────────────────

    private static void OnGraphDataChanged(DependencyObject d, DependencyPropertyChangedEventArgs e)
    {
        var self = (WorkflowGraphView)d;
        self.Surface.Invalidate();
        self.UpdateActionOverlay();
    }

    private static void OnSelectionChanged(DependencyObject d, DependencyPropertyChangedEventArgs e)
    {
        var self = (WorkflowGraphView)d;
        self.Surface.Invalidate();
        self.UpdateActionOverlay();
    }

    // ── Animation ─────────────────────────────────────────────────────────

    private void StartAnimation()
    {
        _animTimer = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(33) };
        _animTimer.Tick += (_, _) =>
        {
            if (IsRunning) { _dashPhase -= 0.5f; Surface.Invalidate(); }
        };
        _animTimer.Start();
    }

    private void StopAnimation()
    {
        _animTimer?.Stop();
        _animTimer = null;
    }

    // ── Draw ──────────────────────────────────────────────────────────────

    private void Surface_Draw(CanvasControl sender, CanvasDrawEventArgs args)
    {
        var ds = args.DrawingSession;
        float w = (float)sender.ActualWidth;
        float h = (float)sender.ActualHeight;

        ds.Clear(AppTheme.Canvas);

        // Apply pan+zoom transform centred on viewport
        ds.Transform =
            Matrix3x2.CreateScale((float)_zoom, new Vector2(w / 2f, h / 2f)) *
            Matrix3x2.CreateTranslation((float)_panX, (float)_panY);

        DrawDotGrid(ds, w, h);
        DrawEdges(ds, w, h);
        DrawNodes(ds, w, h);

        ds.Transform = Matrix3x2.Identity;
    }

    // ── Dot grid ─────────────────────────────────────────────────────────

    private void DrawDotGrid(CanvasDrawingSession ds, float w, float h)
    {
        const float spacing = 22f;
        const float dotRadius = 0.9f;
        var color = AppTheme.CanvasGrid;

        for (float x = -500f; x < w + 500f; x += spacing)
            for (float y = -500f; y < h + 500f; y += spacing)
                ds.FillCircle(x, y, dotRadius, color);
    }

    // ── Edges ─────────────────────────────────────────────────────────────

    private void DrawEdges(CanvasDrawingSession ds, float w, float h)
    {
        if (Edges is null || Nodes is null) return;

        var nodeMap = Nodes.ToDictionary(n => n.Id);

        foreach (var edge in Edges)
        {
            if (!nodeMap.TryGetValue(edge.From, out var fromNode) ||
                !nodeMap.TryGetValue(edge.To,   out var toNode)) continue;

            var start = GraphLayoutMath.RightPort(fromNode.Position, w, h);
            var end   = GraphLayoutMath.LeftPort(toNode.Position,   w, h);
            var (c1, c2) = GraphLayoutMath.BezierControls(start, end);

            bool isActive = toNode.Status == WorkflowNodeStatus.Running ||
                            (fromNode.Status == WorkflowNodeStatus.Succeeded &&
                             (toNode.Status == WorkflowNodeStatus.Running ||
                              toNode.Status == WorkflowNodeStatus.Succeeded));

            Color edgeColor = isActive ? AppTheme.EdgeActive : AppTheme.Edge;
            float lineWidth = isActive ? 2.4f : 1.6f;

            using var pb = new CanvasPathBuilder(ds);
            pb.BeginFigure(new Vector2((float)start.X, (float)start.Y));
            pb.AddCubicBezier(
                new Vector2((float)c1.X, (float)c1.Y),
                new Vector2((float)c2.X, (float)c2.Y),
                new Vector2((float)end.X,   (float)end.Y));
            pb.EndFigure(CanvasFigureLoop.Open);

            using var geometry = CanvasGeometry.CreatePath(pb);

            if (isActive)
            {
                var strokeStyle = new CanvasStrokeStyle
                {
                    DashStyle       = CanvasDashStyle.Custom,
                    CustomDashStyle = new float[] { 6f, 7f },
                    DashOffset      = _dashPhase,
                };
                ds.DrawGeometry(geometry, edgeColor, lineWidth, strokeStyle);
            }
            else
            {
                ds.DrawGeometry(geometry, edgeColor, lineWidth);
            }

            DrawArrowhead(ds, end, c2, edgeColor);
        }
    }

    private void DrawArrowhead(CanvasDrawingSession ds,
        (double X, double Y) tip, (double X, double Y) control, Color color)
    {
        double dx = tip.X - control.X;
        double dy = tip.Y - control.Y;
        double len = Math.Sqrt(dx * dx + dy * dy);
        if (len < 0.001) return;

        double ux = dx / len;
        double uy = dy / len;
        const double size = 6.0;

        // Triangle: tip, left-wing, right-wing
        var apex  = new Vector2((float)tip.X, (float)tip.Y);
        var wing1 = new Vector2(
            (float)(tip.X - ux * size - uy * size * 0.5),
            (float)(tip.Y - uy * size + ux * size * 0.5));
        var wing2 = new Vector2(
            (float)(tip.X - ux * size + uy * size * 0.5),
            (float)(tip.Y - uy * size - ux * size * 0.5));

        using var pb = new CanvasPathBuilder(ds);
        pb.BeginFigure(apex);
        pb.AddLine(wing1);
        pb.AddLine(wing2);
        pb.EndFigure(CanvasFigureLoop.Closed);

        using var geo = CanvasGeometry.CreatePath(pb);
        ds.FillGeometry(geo, color);
    }

    // ── Nodes ─────────────────────────────────────────────────────────────

    private void DrawNodes(CanvasDrawingSession ds, float w, float h)
    {
        if (Nodes is null) return;

        foreach (var node in Nodes)
        {
            bool isSelected = node.Id == SelectedNodeId;
            var (cx, cy) = GraphLayoutMath.NodeCenter(node.Position, w, h);
            double l = cx - GraphLayoutMath.NodeWidth  / 2;
            double t = cy - GraphLayoutMath.NodeHeight / 2;
            float  fw = (float)GraphLayoutMath.NodeWidth;
            float  fh = (float)GraphLayoutMath.NodeHeight;

            // 1. Node card background
            var cardRect = new Rect(l, t, GraphLayoutMath.NodeWidth, GraphLayoutMath.NodeHeight);
            ds.FillRoundedRectangle(cardRect, 12f, 12f, AppTheme.NodeCard);

            if (isSelected)
            {
                // Glow (simulated with thick semi-transparent stroke)
                var glowColor = Color.FromArgb(64, AppTheme.Accent.R, AppTheme.Accent.G, AppTheme.Accent.B);
                ds.DrawRoundedRectangle(cardRect, 12f, 12f, glowColor, 5f);
                ds.DrawRoundedRectangle(cardRect, 12f, 12f, AppTheme.Accent, 2f);
            }
            else
            {
                ds.DrawRoundedRectangle(cardRect, 12f, 12f, AppTheme.NodeStroke, 1f);
            }

            // 2. Kind chip (left side, 46x46)
            const float chipSize   = 46f;
            const float chipMargin = 10f;
            const float chipRadius = 11f;
            float chipL = (float)l + chipMargin;
            float chipT = (float)t + ((float)GraphLayoutMath.NodeHeight - chipSize) / 2f;

            var kindColor = AppTheme.KindColor(node.Kind);
            var chipFill  = Color.FromArgb(46,  kindColor.R, kindColor.G, kindColor.B);
            var chipStroke = Color.FromArgb(140, kindColor.R, kindColor.G, kindColor.B);

            ds.FillRoundedRectangle(chipL, chipT, chipSize, chipSize, chipRadius, chipRadius, chipFill);
            ds.DrawRoundedRectangle(chipL, chipT, chipSize, chipSize, chipRadius, chipRadius, chipStroke, 1f);

            // Kind glyph centered in chip
            string kindGlyph = GlyphForKind(node.Kind);
            ds.DrawText(kindGlyph,
                chipL + chipSize / 2f, chipT + chipSize / 2f,
                kindColor,
                new CanvasTextFormat
                {
                    FontFamily           = SymbolGlyphs.FontFamily,
                    FontSize             = 18f,
                    HorizontalAlignment  = CanvasHorizontalAlignment.Center,
                    VerticalAlignment    = CanvasVerticalAlignment.Center,
                });

            // 3. Text area (right of chip)
            float textX = chipL + chipSize + 10f;
            float textMaxW = (float)l + fw - textX - 36f; // leave room for status badge

            // Title
            ds.DrawText(node.Title,
                new Rect(textX, (float)t + 16f, textMaxW, 20f),
                AppTheme.Text,
                new CanvasTextFormat
                {
                    FontSize             = 14f,
                    FontWeight           = new Windows.UI.Text.FontWeight(600),
                    HorizontalAlignment  = CanvasHorizontalAlignment.Left,
                    VerticalAlignment    = CanvasVerticalAlignment.Center,
                    Options              = CanvasDrawTextOptions.Clip,
                });

            // Kind label (subtitle / kind name)
            ds.DrawText(node.Kind.DisplayName().ToUpperInvariant(),
                new Rect(textX, (float)t + 38f, textMaxW, 16f),
                AppTheme.TextMuted,
                new CanvasTextFormat
                {
                    FontSize             = 9f,
                    HorizontalAlignment  = CanvasHorizontalAlignment.Left,
                    VerticalAlignment    = CanvasVerticalAlignment.Center,
                    Options              = CanvasDrawTextOptions.Clip,
                });

            // 4. Status badge (right side)
            float badgeCx = (float)l + fw - 20f;
            float badgeCy = (float)cy;
            DrawStatusBadge(ds, node.Status, badgeCx, badgeCy);

            // 5. Ports
            // Left port: filled circle, kind color
            var (lx, ly) = GraphLayoutMath.LeftPort(node.Position, w, h);
            ds.FillCircle((float)lx, (float)ly, 4f, kindColor);
            ds.DrawCircle((float)lx, (float)ly, 4f, AppTheme.NodeCard, 1.5f);

            // Right port: hollow
            var (rx, ry) = GraphLayoutMath.RightPort(node.Position, w, h);
            ds.FillCircle((float)rx, (float)ry, 4f, AppTheme.NodeCard);
            ds.DrawCircle((float)rx, (float)ry, 4f, kindColor, 1.5f);
        }
    }

    private void DrawStatusBadge(CanvasDrawingSession ds,
        WorkflowNodeStatus status, float cx, float cy)
    {
        var color = AppTheme.StatusColor(status);

        switch (status)
        {
            case WorkflowNodeStatus.Idle:
                ds.DrawCircle(cx, cy, 4f, AppTheme.TextFaint, 1.5f);
                break;

            case WorkflowNodeStatus.Running:
                var ringColor = Color.FromArgb(64, color.R, color.G, color.B);
                ds.FillCircle(cx, cy, 9f, ringColor);
                ds.FillCircle(cx, cy, 4f, color);
                break;

            case WorkflowNodeStatus.Succeeded:
                DrawStatusGlyph(ds, SymbolGlyphs.CheckCircle, cx, cy, color, 14f);
                break;

            case WorkflowNodeStatus.Failed:
                DrawStatusGlyph(ds, SymbolGlyphs.ErrorCircle, cx, cy, color, 14f);
                break;

            case WorkflowNodeStatus.Skipped:
                DrawStatusGlyph(ds, SymbolGlyphs.SkipCircle, cx, cy, AppTheme.TextMuted, 14f);
                break;

            case WorkflowNodeStatus.Paused:
                DrawStatusGlyph(ds, SymbolGlyphs.PauseCircle, cx, cy, color, 14f);
                break;

            case WorkflowNodeStatus.Cancelled:
                DrawStatusGlyph(ds, SymbolGlyphs.SlashCircle, cx, cy, AppTheme.TextMuted, 14f);
                break;
        }
    }

    private static void DrawStatusGlyph(CanvasDrawingSession ds,
        string glyph, float cx, float cy, Color color, float fontSize)
    {
        ds.DrawText(glyph, cx, cy, color,
            new CanvasTextFormat
            {
                FontFamily          = SymbolGlyphs.FontFamily,
                FontSize            = fontSize,
                HorizontalAlignment = CanvasHorizontalAlignment.Center,
                VerticalAlignment   = CanvasVerticalAlignment.Center,
            });
    }

    private static string GlyphForKind(WorkflowNodeKind kind) => kind switch
    {
        WorkflowNodeKind.Context => SymbolGlyphs.Document,
        WorkflowNodeKind.Script  => SymbolGlyphs.Code,
        WorkflowNodeKind.Agent   => SymbolGlyphs.Sparkles,
        WorkflowNodeKind.Claude  => SymbolGlyphs.ChatBubble,
        WorkflowNodeKind.Codex   => SymbolGlyphs.Braces,
        WorkflowNodeKind.Digest  => SymbolGlyphs.Archive,
        _                        => SymbolGlyphs.Document,
    };

    // ── Pointer events ────────────────────────────────────────────────────

    private void Surface_PointerPressed(object sender, PointerRoutedEventArgs e)
    {
        Surface.CapturePointer(e.Pointer);
        var pos = e.GetCurrentPoint(Surface).Position;
        _dragStartX = pos.X;
        _dragStartY = pos.Y;
        _panStartX  = _panX;
        _panStartY  = _panY;
        _isDragging = true;
    }

    private void Surface_PointerMoved(object sender, PointerRoutedEventArgs e)
    {
        var pos = e.GetCurrentPoint(Surface).Position;

        if (_isDragging)
        {
            _panX = _panStartX + (pos.X - _dragStartX);
            _panY = _panStartY + (pos.Y - _dragStartY);
            Surface.Invalidate();
            return;
        }

        // Hover hit-test
        double w = Surface.ActualWidth, h = Surface.ActualHeight;
        var (wx, wy) = NodeHitTester.ScreenToWorld(pos.X, pos.Y, _panX, _panY, _zoom, w / 2, h / 2);
        string? hit = Nodes is not null ? NodeHitTester.HitTest(wx, wy, Nodes, w, h) : null;

        if (hit != _hoveredNodeId)
        {
            _hoveredNodeId = hit;
            Surface.Invalidate();
            UpdateActionOverlay();
        }
    }

    private void Surface_PointerReleased(object sender, PointerRoutedEventArgs e)
    {
        Surface.ReleasePointerCapture(e.Pointer);

        var pos = e.GetCurrentPoint(Surface).Position;
        double dx = pos.X - _dragStartX;
        double dy = pos.Y - _dragStartY;
        bool wasClick = Math.Sqrt(dx * dx + dy * dy) < 4.0;

        _isDragging = false;

        if (wasClick)
        {
            double w = Surface.ActualWidth, h = Surface.ActualHeight;
            var (wx, wy) = NodeHitTester.ScreenToWorld(pos.X, pos.Y, _panX, _panY, _zoom, w / 2, h / 2);
            string? hit = Nodes is not null ? NodeHitTester.HitTest(wx, wy, Nodes, w, h) : null;

            if (hit is not null)
                NodeSelected?.Invoke(hit);
            else
                SelectedNodeId = null;

            Surface.Invalidate();
        }
    }

    private void Surface_PointerWheelChanged(object sender, PointerRoutedEventArgs e)
    {
        int delta = e.GetCurrentPoint(Surface).Properties.MouseWheelDelta;
        double newZoom = GraphLayoutMath.ClampZoom(_zoom + delta / 1200.0);
        _zoom = newZoom;
        Surface.Invalidate();
    }

    private void Surface_PointerExited(object sender, PointerRoutedEventArgs e)
    {
        _hoveredNodeId = null;
        UpdateActionOverlay();
    }

    // ── Action overlay ────────────────────────────────────────────────────

    private void UpdateActionOverlay()
    {
        string? targetId = SelectedNodeId ?? _hoveredNodeId;
        if (targetId is null || Nodes is null)
        {
            if (_actionOverlay is not null) _actionOverlay.Visibility = Visibility.Collapsed;
            return;
        }

        var node = Nodes.FirstOrDefault(n => n.Id == targetId);
        if (node is null)
        {
            if (_actionOverlay is not null) _actionOverlay.Visibility = Visibility.Collapsed;
            return;
        }

        double w = Surface.ActualWidth, h = Surface.ActualHeight;
        var (cx, cy) = GraphLayoutMath.NodeCenter(node.Position, w, h);

        // Screen coordinates of the node center after zoom+pan
        double screenCx = (cx - w / 2) * _zoom + w / 2 + _panX;
        double screenCy = (cy - h / 2) * _zoom + h / 2 + _panY;
        double cardW    = GraphLayoutMath.NodeWidth  * _zoom;
        double cardH    = GraphLayoutMath.NodeHeight * _zoom;

        double overlayLeft = screenCx + cardW / 2 - 120;
        double overlayTop  = screenCy - cardH / 2 + 4;

        if (_actionOverlay is null || _overlayNodeId != targetId)
        {
            if (_actionOverlay is not null) ActionLayer.Children.Remove(_actionOverlay);

            _actionOverlay = new NodeActionOverlay();
            _actionOverlay.RunClicked      += (_, _) => NodeRun?.Invoke(targetId);
            _actionOverlay.RestartClicked  += (_, _) => NodeRestart?.Invoke(targetId);
            _actionOverlay.StopClicked     += (_, _) => NodeStop?.Invoke();
            _actionOverlay.OverrideClicked += (_, _) => NodeOverride?.Invoke(targetId);
            ActionLayer.Children.Add(_actionOverlay);
            _overlayNodeId = targetId;
        }

        _actionOverlay.NodeStatus        = node.Status;
        _actionOverlay.IsWorkflowRunning = IsRunning;
        _actionOverlay.Visibility        = Visibility.Visible;
        Canvas.SetLeft(_actionOverlay, overlayLeft);
        Canvas.SetTop(_actionOverlay,  overlayTop);
    }

    // ── Zoom buttons ──────────────────────────────────────────────────────

    private void ZoomIn_Click(object sender, RoutedEventArgs e)
    {
        _zoom = GraphLayoutMath.ClampZoom(_zoom + 0.1);
        Surface.Invalidate();
    }

    private void ZoomOut_Click(object sender, RoutedEventArgs e)
    {
        _zoom = GraphLayoutMath.ClampZoom(_zoom - 0.1);
        Surface.Invalidate();
    }

    private void ZoomReset_Click(object sender, RoutedEventArgs e)
    {
        _zoom = 1.0; _panX = 0; _panY = 0;
        Surface.Invalidate();
        UpdateActionOverlay();
    }

    // ── Size changed ──────────────────────────────────────────────────────

    private void Surface_SizeChanged(object sender, SizeChangedEventArgs e)
        => UpdateActionOverlay();

}
