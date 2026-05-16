using System.Diagnostics;
using System.Net.Http.Headers;
using System.Runtime.CompilerServices;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using ManualWindow.Models;

namespace ManualWindow.Services;

public sealed class AppServerClient : IAppServerClient, IDisposable
{
    private readonly HttpClient _http = new();
    private Uri? _serverUrl;
    private string? _authToken;
    private Process? _daemon;
    private int _nextId = 1;

    // --- Public API ---

    public async Task<List<WorkflowSummary>> WorkflowsAsync(CancellationToken ct = default)
    {
        var result = await RequestAsync("workflow.list", new JsonObject(), ct);
        var arr = result["workflows"]?.AsArray()
            ?? throw AppServerClientException.InvalidResponse();
        return arr.Select(n => WorkflowSummary.FromJson(n!)).ToList();
    }

    public async Task<JsonObject> WorkflowAsync(string id, CancellationToken ct = default)
    {
        var result = await RequestAsync("workflow.get",
            new JsonObject { ["workflow_id"] = id }, ct);
        return result["workflow"]?.AsObject()
            ?? throw AppServerClientException.InvalidResponse();
    }

    public async Task<WorkflowMutationResult> CreateWorkflowAsync(JsonObject workflow, CancellationToken ct = default)
    {
        var result = await RequestAsync("workflow.create",
            new JsonObject { ["workflow"] = workflow.DeepClone() }, ct);
        return WorkflowMutationResult.FromJson(result);
    }

    public async Task<WorkflowMutationResult> UpdateWorkflowAsync(string id, JsonObject workflow, CancellationToken ct = default)
    {
        var result = await RequestAsync("workflow.update",
            new JsonObject { ["workflow_id"] = id, ["workflow"] = workflow.DeepClone() }, ct);
        return WorkflowMutationResult.FromJson(result);
    }

    public async Task<WorkflowDeleteResult> DeleteWorkflowAsync(string id, CancellationToken ct = default)
    {
        var result = await RequestAsync("workflow.delete",
            new JsonObject { ["workflow_id"] = id }, ct);
        return WorkflowDeleteResult.FromJson(result);
    }

    public async Task<string> StartWorkflowAsync(string workflowId, WorkflowStartOptions? options = null, CancellationToken ct = default)
    {
        var p = new JsonObject { ["workflow_id"] = workflowId };
        if (options is not null)
        {
            if (options.StartNodeId is not null) p["start_node_id"] = options.StartNodeId;
            if (options.ResumeFromFailure) p["resume_from_failure"] = true;
            if (options.InputOverrides.Count > 0) p["input_overrides"] = (JsonObject)options.InputOverrides.DeepClone();
            if (options.Mode != ExecutionMode.Auto) p["mode"] = options.Mode.ToString().ToLowerInvariant();
            if (options.ResumeRunId is not null) p["resume_run_id"] = options.ResumeRunId;
        }
        var result = await RequestAsync("workflow.start", p, ct);
        return result["run_id"]?.GetValue<string>() ?? throw AppServerClientException.InvalidResponse();
    }

    public async Task<StopWorkflowResult> StopWorkflowAsync(string runId, CancellationToken ct = default)
    {
        var result = await RequestAsync("workflow.stop",
            new JsonObject { ["run_id"] = runId }, ct);
        return StopWorkflowResult.FromJson(result);
    }

    public async Task<string> ResumeWorkflowAsync(string runId, WorkflowStartOptions? options = null, CancellationToken ct = default)
    {
        var p = new JsonObject { ["run_id"] = runId };
        if (options is not null)
        {
            if (options.StartNodeId is not null) p["start_node_id"] = options.StartNodeId;
            if (options.ResumeFromFailure) p["resume_from_failure"] = true;
            if (options.InputOverrides.Count > 0) p["input_overrides"] = (JsonObject)options.InputOverrides.DeepClone();
            if (options.Mode != ExecutionMode.Auto) p["mode"] = options.Mode.ToString().ToLowerInvariant();
        }
        var result = await RequestAsync("workflow.resume", p, ct);
        return result["run_id"]?.GetValue<string>() ?? throw AppServerClientException.InvalidResponse();
    }

    public async Task<WorkflowEventsPage> EventsAsync(string runId, int cursor, CancellationToken ct = default)
    {
        var result = await RequestAsync("workflow.events",
            new JsonObject { ["run_id"] = runId, ["cursor"] = cursor }, ct);

        var events = result["events"]?.AsArray()
            ?.Select(n => n?.AsObject() ?? throw AppServerClientException.InvalidResponse())
            .ToList()
            as IReadOnlyList<JsonObject>
            ?? throw AppServerClientException.InvalidResponse();

        var nextCursor = result["next_cursor"]?.GetValue<int>() ?? throw AppServerClientException.InvalidResponse();
        var completed  = result["completed"]?.GetValue<bool>()  ?? throw AppServerClientException.InvalidResponse();
        var run        = result["run"]?.AsObject()              ?? throw AppServerClientException.InvalidResponse();

        return new WorkflowEventsPage(events, nextCursor, completed, run);
    }

    public async IAsyncEnumerable<AppServerLiveEvent> LiveEventsAsync(
        [EnumeratorCancellation] CancellationToken ct = default)
    {
        await EnsureDaemonAsync(ct);

        var components = new UriBuilder(_serverUrl!.AbsoluteUri.TrimEnd('/') + "/events")
        {
            Query = $"token={Uri.EscapeDataString(_authToken!)}"
        };

        using var request = new HttpRequestMessage(HttpMethod.Get, components.Uri);
        using var response = await _http.SendAsync(request, HttpCompletionOption.ResponseHeadersRead, ct);
        response.EnsureSuccessStatusCode();

        using var stream = await response.Content.ReadAsStreamAsync(ct);
        using var reader = new StreamReader(stream, Encoding.UTF8);
        string eventName = "message";

        while (!reader.EndOfStream && !ct.IsCancellationRequested)
        {
            var line = await reader.ReadLineAsync(ct);
            if (line is null) break;
            if (line.StartsWith("event: ", StringComparison.Ordinal))
                eventName = line[7..];
            else if (line.StartsWith("data: ", StringComparison.Ordinal))
            {
                var payload = JsonNode.Parse(line[6..])?.AsObject();
                if (payload is not null)
                    yield return new AppServerLiveEvent(eventName, payload);
            }
        }
    }

    // --- Private ---

    private async Task<JsonObject> RequestAsync(string method, JsonObject @params, CancellationToken ct)
    {
        await EnsureDaemonAsync(ct);

        var id = Interlocked.Increment(ref _nextId);
        var payload = new JsonObject
        {
            ["jsonrpc"] = "2.0",
            ["id"]      = id,
            ["method"]  = method,
            ["params"]  = @params.DeepClone(),
        };

        using var request = new HttpRequestMessage(HttpMethod.Post, new Uri(_serverUrl!, "/rpc"));
        request.Headers.Authorization = new AuthenticationHeaderValue("Bearer", _authToken);
        request.Content = new StringContent(payload.ToJsonString(), Encoding.UTF8, "application/json");

        using var response = await _http.SendAsync(request, ct);
        if (!response.IsSuccessStatusCode)
            throw AppServerClientException.InvalidResponse();

        var body = await response.Content.ReadAsStringAsync(ct);
        if (string.IsNullOrEmpty(body))
            throw AppServerClientException.EmptyResponse();

        var doc = JsonNode.Parse(body)?.AsObject()
            ?? throw AppServerClientException.InvalidResponse();

        if (doc["error"] is JsonObject err)
            throw AppServerClientException.RpcError(
                err["code"]?.GetValue<int>() ?? 0,
                err["message"]?.GetValue<string>() ?? "JSON-RPC error");

        return doc["result"]?.AsObject()
            ?? throw AppServerClientException.InvalidResponse();
    }

    private async Task EnsureDaemonAsync(CancellationToken ct)
    {
        if (_serverUrl is not null && await HealthCheckAsync(_serverUrl, ct)) return;

        // Try existing discovery file
        var discoveryPath = DiscoveryFilePath();
        var discovery = TryReadDiscovery(discoveryPath);
        if (discovery is not null && await HealthCheckAsync(discovery.Value.Url, ct))
        {
            (_serverUrl, _authToken) = discovery.Value;
            return;
        }

        // Spawn daemon
        var binary = ResolveAppServerBinary()
            ?? throw AppServerClientException.BinaryNotFound();

        var token = Guid.NewGuid().ToString("N");
        var info = new ProcessStartInfo(binary)
        {
            UseShellExecute  = false,
            CreateNoWindow   = true,
            RedirectStandardOutput = false,
            RedirectStandardError  = false,
        };
        info.ArgumentList.Add("--listen");
        info.ArgumentList.Add("127.0.0.1:0");
        info.ArgumentList.Add("--auth-token");
        info.ArgumentList.Add(token);
        info.ArgumentList.Add("--discovery-file");
        info.ArgumentList.Add(discoveryPath);

        _daemon = Process.Start(info);

        for (int i = 0; i < 60; i++)
        {
            await Task.Delay(50, ct);
            discovery = TryReadDiscovery(discoveryPath);
            if (discovery is not null && await HealthCheckAsync(discovery.Value.Url, ct))
            {
                (_serverUrl, _authToken) = discovery.Value;
                return;
            }
        }

        throw AppServerClientException.EmptyResponse();
    }

    private async Task<bool> HealthCheckAsync(Uri url, CancellationToken ct)
    {
        try
        {
            using var response = await _http.GetAsync(new Uri(url, "/health"), ct);
            return response.IsSuccessStatusCode;
        }
        catch { return false; }
    }

    private static (Uri Url, string AuthToken)? TryReadDiscovery(string path)
    {
        try
        {
            var text = File.ReadAllText(path);
            var obj  = JsonNode.Parse(text)?.AsObject();
            var urlStr    = obj?["url"]?.GetValue<string>();
            var authToken = obj?["auth_token"]?.GetValue<string>();
            if (urlStr is null || authToken is null) return null;
            return (new Uri(urlStr), authToken);
        }
        catch { return null; }
    }

    private static string DiscoveryFilePath()
    {
        var envPath = Environment.GetEnvironmentVariable("MANUAL_APP_SERVER_DISCOVERY");
        if (!string.IsNullOrEmpty(envPath)) return envPath;

        var localAppData = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
        var dir = Path.Combine(localAppData, "Manual");
        Directory.CreateDirectory(dir);
        return Path.Combine(dir, "app-server.json");
    }

    private static string? ResolveAppServerBinary()
    {
        var envBin = Environment.GetEnvironmentVariable("MANUAL_APP_SERVER_BIN");
        if (!string.IsNullOrEmpty(envBin) && File.Exists(envBin)) return envBin;

        // Walk 5 levels up from base dir to reach repo root
        // app/window/bin/x64/Debug/net10.0-windows.../ → manual/
        var dir = new DirectoryInfo(AppContext.BaseDirectory);
        for (int i = 0; i < 5; i++) dir = dir.Parent ?? dir;

        var candidate = Path.Combine(dir.FullName, "manual-rs", "target", "debug", "app-server.exe");
        return File.Exists(candidate) ? candidate : null;
    }

    public void Dispose()
    {
        try { _daemon?.Kill(entireProcessTree: true); } catch { }
        _daemon?.Dispose();
        _http.Dispose();
    }
}
