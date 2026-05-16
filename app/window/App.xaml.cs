using ManualWindow.Services;
using ManualWindow.Stores;
using Microsoft.UI.Xaml;

namespace ManualWindow;

public partial class App : Application
{
    public static WorkflowRunStore? Store { get; private set; }
    public static UiPreferencesStore? Prefs { get; private set; }

    private Window? _window;

    public App() { InitializeComponent(); }

    protected override void OnLaunched(LaunchActivatedEventArgs args)
    {
        // Store must be created on UI thread (captures DispatcherQueue)
        Prefs = new UiPreferencesStore();
        Store = new WorkflowRunStore(new AppServerClient());

        _window = new MainWindow();
        _window.Activate();

        // Bootstrap after window is active (daemon spawn is async)
        _ = Store.Bootstrap();
    }
}
