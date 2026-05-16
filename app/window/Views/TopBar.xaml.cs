using ManualWindow.Theme;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace ManualWindow.Views;

public sealed partial class TopBar : UserControl
{
    public static readonly DependencyProperty WorkflowIdProperty =
        DependencyProperty.Register(nameof(WorkflowId), typeof(string),
            typeof(TopBar), new PropertyMetadata(null,
                (d, e) => ((TopBar)d).WorkflowIdText.Text = e.NewValue as string ?? ""));

    public static readonly DependencyProperty IsRunningProperty =
        DependencyProperty.Register(nameof(IsRunning), typeof(bool),
            typeof(TopBar), new PropertyMetadata(false,
                (d, _) => ((TopBar)d).UpdateState()));

    public static readonly DependencyProperty IsPausedProperty =
        DependencyProperty.Register(nameof(IsPaused), typeof(bool),
            typeof(TopBar), new PropertyMetadata(false,
                (d, _) => ((TopBar)d).UpdateState()));

    public static readonly DependencyProperty StatusMessageProperty =
        DependencyProperty.Register(nameof(StatusMessage), typeof(string),
            typeof(TopBar), new PropertyMetadata("Ready",
                (d, e) => ((TopBar)d).StatusText.Text = e.NewValue as string ?? ""));

    public string? WorkflowId
    {
        get => (string?)GetValue(WorkflowIdProperty);
        set => SetValue(WorkflowIdProperty, value);
    }

    public bool IsRunning
    {
        get => (bool)GetValue(IsRunningProperty);
        set => SetValue(IsRunningProperty, value);
    }

    public bool IsPaused
    {
        get => (bool)GetValue(IsPausedProperty);
        set => SetValue(IsPausedProperty, value);
    }

    public string? StatusMessage
    {
        get => (string?)GetValue(StatusMessageProperty);
        set => SetValue(StatusMessageProperty, value);
    }

    public event Action? ExecuteRequested;
    public event Action? StepModeRequested;
    public event Action? StopRequested;
    public event Action? NextStepRequested;

    public TopBar()
    {
        InitializeComponent();
        ExecuteIcon.Glyph  = SymbolGlyphs.Play;
        StepModeIcon.Glyph = SymbolGlyphs.Forward;
        StopIcon.Glyph     = SymbolGlyphs.Stop;
        NextStepIcon.Glyph = SymbolGlyphs.Forward;
    }

    private void UpdateState()
    {
        bool running = IsRunning;
        bool paused  = IsPaused;

        ExecuteButton.Visibility  = Show(!running);
        StepModeButton.Visibility = Show(!running);
        StopButton.Visibility     = Show(running);
        PausedBanner.Visibility   = Show(paused);
        NextStepButton.Visibility = Show(paused);
    }

    private static Visibility Show(bool v) => v ? Visibility.Visible : Visibility.Collapsed;

    private void Execute_Click(object s, RoutedEventArgs e)  => ExecuteRequested?.Invoke();
    private void StepMode_Click(object s, RoutedEventArgs e) => StepModeRequested?.Invoke();
    private void Stop_Click(object s, RoutedEventArgs e)     => StopRequested?.Invoke();
    private void NextStep_Click(object s, RoutedEventArgs e) => NextStepRequested?.Invoke();
}
