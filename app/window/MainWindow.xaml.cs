using Microsoft.UI.Xaml;

namespace ManualWindow;

public sealed partial class MainWindow : Window
{
    public MainWindow()
    {
        InitializeComponent();
        Title = "Manual";
        ExtendsContentIntoTitleBar = true;

        // Initialize shell after Store is ready
        if (App.Store is not null && App.Prefs is not null)
            Shell.Initialize(App.Store, App.Prefs);
    }
}
