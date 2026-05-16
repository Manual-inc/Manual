using ManualWindow.Models;
using ManualWindow.Theme;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Windows.UI;

namespace ManualWindow.Views;

public sealed partial class NodeKindIcon : UserControl
{
    public static readonly DependencyProperty KindProperty =
        DependencyProperty.Register(nameof(Kind), typeof(WorkflowNodeKind),
            typeof(NodeKindIcon), new PropertyMetadata(WorkflowNodeKind.Script, OnKindChanged));

    public static readonly DependencyProperty SymbolSizeProperty =
        DependencyProperty.Register(nameof(SymbolSize), typeof(double),
            typeof(NodeKindIcon), new PropertyMetadata(16.0, OnKindChanged));

    public WorkflowNodeKind Kind
    {
        get => (WorkflowNodeKind)GetValue(KindProperty);
        set => SetValue(KindProperty, value);
    }

    public double SymbolSize
    {
        get => (double)GetValue(SymbolSizeProperty);
        set => SetValue(SymbolSizeProperty, value);
    }

    public NodeKindIcon() => InitializeComponent();

    private static void OnKindChanged(DependencyObject d, DependencyPropertyChangedEventArgs e)
        => ((NodeKindIcon)d).Update();

    private void Update()
    {
        Icon.FontFamily = new FontFamily(SymbolGlyphs.FontFamily);
        Icon.FontSize   = SymbolSize;
        Icon.Glyph      = GlyphFor(Kind);
        Icon.Foreground = new SolidColorBrush(AppTheme.KindColor(Kind));
    }

    private static string GlyphFor(WorkflowNodeKind kind) => kind switch
    {
        WorkflowNodeKind.Context => SymbolGlyphs.Document,
        WorkflowNodeKind.Script  => SymbolGlyphs.Code,
        WorkflowNodeKind.Agent   => SymbolGlyphs.Sparkles,
        WorkflowNodeKind.Claude  => SymbolGlyphs.ChatBubble,
        WorkflowNodeKind.Codex   => SymbolGlyphs.Braces,
        WorkflowNodeKind.Digest  => SymbolGlyphs.Archive,
        _                        => SymbolGlyphs.Document,
    };
}
