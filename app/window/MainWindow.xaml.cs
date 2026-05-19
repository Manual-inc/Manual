using Microsoft.UI.Xaml;

namespace ManualWindow;

public sealed partial class MainWindow : Window
{
    public MainWindow()
    {
        InitializeComponent();
        // Why this exists: docs/wiki/architecture/manual-app-architecture.md
        // defines native clients as shared workflow + optimization surfaces.
        Title = "Manual Optimization Console";
    }
}
