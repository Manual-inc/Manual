namespace ManualWindow.Models;

public sealed class AppServerClientException : Exception
{
    private AppServerClientException(string message) : base(message) { }

    public static AppServerClientException BinaryNotFound()
        => new("The app-server binary was not found.");

    public static AppServerClientException EmptyResponse()
        => new("The app-server process returned an empty response.");

    public static AppServerClientException InvalidResponse()
        => new("The app-server response was not valid JSON-RPC.");

    public static AppServerClientException RpcError(int code, string message)
        => new($"app-server error {code}: {message}");
}
