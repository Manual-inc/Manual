using Microsoft.Windows.ApplicationModel.DynamicDependency;

namespace ManualWindow;

class Program
{
    [STAThread]
    static void Main(string[] args)
    {
        Bootstrap.Initialize(0x00010008); // Windows App Runtime 1.8
        WinRT.ComWrappersSupport.InitializeComWrappers();
        Microsoft.UI.Xaml.Application.Start(p =>
        {
            var ctx = new Microsoft.UI.Dispatching.DispatcherQueueSynchronizationContext(
                Microsoft.UI.Dispatching.DispatcherQueue.GetForCurrentThread());
            System.Threading.SynchronizationContext.SetSynchronizationContext(ctx);
            new App();
        });
    }
}
