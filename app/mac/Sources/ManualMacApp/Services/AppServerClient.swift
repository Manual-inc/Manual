import Foundation

enum AppServerClientError: Error, LocalizedError {
    case binaryNotFound
    case emptyResponse
    case rpcError(Int, String)
    case invalidResponse

    var errorDescription: String? {
        switch self {
        case .binaryNotFound:
            "The app-server binary was not found."
        case .emptyResponse:
            "The app-server process returned an empty response."
        case let .rpcError(code, message):
            "app-server error \(code): \(message)"
        case .invalidResponse:
            "The app-server response was not valid JSON-RPC."
        }
    }
}

@MainActor
final class AppServerClient {
    private var process: Process?
    private var nextID = 1
    private var serverURL: URL?
    private var authToken: String?

    func createWorkflow(_ workflow: [String: Any]) async throws -> WorkflowMutationResult {
        let result = try await request(
            method: "workflow.create",
            params: ["workflow": workflow]
        )
        return try WorkflowMutationResult(result)
    }

    func workflow(id workflowID: String) async throws -> [String: Any] {
        let result = try await request(
            method: "workflow.get",
            params: ["workflow_id": workflowID]
        )

        guard
            let object = result as? [String: Any],
            let workflow = object["workflow"] as? [String: Any]
        else {
            throw AppServerClientError.invalidResponse
        }

        return workflow
    }

    func workflows() async throws -> [WorkflowSummary] {
        let result = try await request(method: "workflow.list", params: [:])

        guard
            let object = result as? [String: Any],
            let workflows = object["workflows"] as? [[String: Any]]
        else {
            throw AppServerClientError.invalidResponse
        }

        return try workflows.map(WorkflowSummary.init)
    }

    func updateWorkflow(id workflowID: String, workflow: [String: Any]) async throws -> WorkflowMutationResult {
        let result = try await request(
            method: "workflow.update",
            params: [
                "workflow_id": workflowID,
                "workflow": workflow,
            ]
        )
        return try WorkflowMutationResult(result)
    }

    func deleteWorkflow(id workflowID: String) async throws -> WorkflowDeleteResult {
        let result = try await request(
            method: "workflow.delete",
            params: ["workflow_id": workflowID]
        )
        return try WorkflowDeleteResult(result)
    }

    func startWorkflow(id workflowID: String) async throws -> String {
        let result = try await request(
            method: "workflow.start",
            params: ["workflow_id": workflowID]
        )

        guard
            let object = result as? [String: Any],
            let runID = object["run_id"] as? String
        else {
            throw AppServerClientError.invalidResponse
        }

        return runID
    }

    func availableAgents() async throws -> [AppServerAgentAvailability] {
        let result = try await request(method: "agent.list", params: [:])

        guard
            let object = result as? [String: Any],
            let agents = object["agents"] as? [[String: Any]]
        else {
            throw AppServerClientError.invalidResponse
        }

        return try agents.map(AppServerAgentAvailability.init)
    }

    func events(runID: String, cursor: Int) async throws -> WorkflowEventsPage {
        let result = try await request(
            method: "workflow.events",
            params: [
                "run_id": runID,
                "cursor": cursor,
            ]
        )

        guard
            let object = result as? [String: Any],
            let events = object["events"] as? [[String: Any]],
            let nextCursor = object["next_cursor"] as? Int,
            let completed = object["completed"] as? Bool,
            let run = object["run"] as? [String: Any]
        else {
            throw AppServerClientError.invalidResponse
        }

        let optimizationReport = (object["optimization_report"] as? [String: Any]).flatMap { try? WorkflowOptimizationReport($0) }
        let optimizationAnalysis = (object["optimization_analysis"] as? [String: Any]).flatMap { try? WorkflowOptimizationAnalysis($0) }

        return WorkflowEventsPage(
            events: events,
            nextCursor: nextCursor,
            completed: completed,
            run: run,
            optimizationReport: optimizationReport,
            optimizationAnalysis: optimizationAnalysis
        )
    }

    func liveEvents() async throws -> AsyncThrowingStream<AppServerLiveEvent, Error> {
        try await ensureDaemon()
        guard let serverURL, let authToken else {
            throw AppServerClientError.binaryNotFound
        }

        let eventsURL = serverURL.appendingPathComponent("events")
        var components = URLComponents(url: eventsURL, resolvingAgainstBaseURL: false)
        components?.queryItems = [URLQueryItem(name: "token", value: authToken)]
        guard let url = components?.url else {
            throw AppServerClientError.invalidResponse
        }

        return AsyncThrowingStream { continuation in
            let task = Task {
                do {
                    let (bytes, response) = try await URLSession.shared.bytes(from: url)
                    guard
                        let httpResponse = response as? HTTPURLResponse,
                        httpResponse.statusCode == 200
                    else {
                        throw AppServerClientError.invalidResponse
                    }

                    var eventName = "message"
                    for try await line in bytes.lines {
                        if line.hasPrefix("event: ") {
                            eventName = String(line.dropFirst("event: ".count))
                        } else if line.hasPrefix("data: ") {
                            let data = Data(line.dropFirst("data: ".count).utf8)
                            if
                                let object = try JSONSerialization.jsonObject(with: data) as? [String: Any]
                            {
                                continuation.yield(AppServerLiveEvent(name: eventName, payload: object))
                            }
                        }
                    }

                    continuation.finish()
                } catch {
                    continuation.finish(throwing: error)
                }
            }

            continuation.onTermination = { _ in
                task.cancel()
            }
        }
    }

    func sandboxes() async throws -> SandboxListResult {
        let result = try await request(method: "sandbox.list", params: [:])
        return try SandboxListResult(result)
    }

    func createSandbox(_ draft: SandboxPolicyDraft) async throws -> SandboxPolicyModel {
        let result = try await request(method: "sandbox.create", params: draft.asParameters)
        return try SandboxPolicyResult(result).sandbox
    }

    func updateSandbox(id sandboxID: String, draft: SandboxPolicyDraft) async throws -> SandboxPolicyModel {
        let result = try await request(
            method: "sandbox.update",
            params: [
                "sandbox_id": sandboxID,
                "changes": draft.asParameters,
            ]
        )
        return try SandboxPolicyResult(result).sandbox
    }

    func evaluateSandbox(id sandboxID: String, operation: String, target: String) async throws -> SandboxDecisionModel {
        let result = try await request(
            method: "sandbox.evaluate",
            params: [
                "sandbox_id": sandboxID,
                "operation": operation,
                "target": target,
            ]
        )
        return try SandboxEvaluationResult(result).decision
    }

    func optimizationReport(workflowID: String) async throws -> WorkflowOptimizationReport {
        // See docs/wiki/systems/매뉴얼-최적화-기능.md: mac UI reads the
        // latest workflow-specific optimization report from app-server.
        let result = try await request(
            method: "optimization.report",
            params: [
                "workflow_id": workflowID
            ]
        )
        return try WorkflowOptimizationReport(result)
    }

    func optimizationAnalysis(workflowID: String) async throws -> WorkflowOptimizationAnalysis {
        // See docs/wiki/systems/매뉴얼-최적화-기능.md: report cards need the
        // underlying bottleneck and regression signals, not only headline text.
        let result = try await request(
            method: "optimization.analyze",
            params: [
                "workflow_id": workflowID
            ]
        )
        return try WorkflowOptimizationAnalysis(result)
    }

    private func request(method: String, params: [String: Any]) async throws -> Any {
        try await ensureDaemon()
        guard let serverURL, let authToken else {
            throw AppServerClientError.binaryNotFound
        }

        let requestID = nextID
        nextID += 1

        let payload: [String: Any] = [
            "jsonrpc": "2.0",
            "id": requestID,
            "method": method,
            "params": params,
        ]

        var request = URLRequest(url: serverURL.appendingPathComponent("rpc"))
        request.httpMethod = "POST"
        request.addValue("Bearer \(authToken)", forHTTPHeaderField: "Authorization")
        request.addValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONSerialization.data(withJSONObject: payload)

        let (responseData, response) = try await URLSession.shared.data(for: request)
        guard
            let httpResponse = response as? HTTPURLResponse,
            httpResponse.statusCode == 200
        else {
            throw AppServerClientError.invalidResponse
        }

        guard !responseData.isEmpty else {
            throw AppServerClientError.emptyResponse
        }

        let decoded = try JSONSerialization.jsonObject(with: responseData)
        guard let object = decoded as? [String: Any] else {
            throw AppServerClientError.invalidResponse
        }

        if let error = object["error"] as? [String: Any] {
            throw AppServerClientError.rpcError(
                error["code"] as? Int ?? 0,
                error["message"] as? String ?? "JSON-RPC error"
            )
        }

        guard let result = object["result"] else {
            throw AppServerClientError.invalidResponse
        }

        return result
    }

    private func ensureDaemon() async throws {
        if let serverURL, await healthCheck(serverURL: serverURL) {
            return
        }

        if let discovery = try? readDiscovery(), await healthCheck(serverURL: discovery.serverURL) {
            serverURL = discovery.serverURL
            authToken = discovery.authToken
            return
        }

        guard let binary = resolvedAppServerBinary() else {
            throw AppServerClientError.binaryNotFound
        }

        let token = UUID().uuidString.replacingOccurrences(of: "-", with: "")
        let discoveryURL = try discoveryFileURL()
        let process = Process()
        process.executableURL = URL(fileURLWithPath: binary)
        process.arguments = [
            "--listen", "127.0.0.1:0",
            "--auth-token", token,
            "--discovery-file", discoveryURL.path,
        ]
        process.standardInput = nil
        process.standardOutput = nil
        process.standardError = FileHandle.standardError

        try process.run()
        self.process = process

        let deadline = Date().addingTimeInterval(3)
        while Date() < deadline {
            if let discovery = try? readDiscovery(), await healthCheck(serverURL: discovery.serverURL) {
                serverURL = discovery.serverURL
                authToken = discovery.authToken
                return
            }

            try? await Task.sleep(for: .milliseconds(50))
        }

        throw AppServerClientError.emptyResponse
    }

    private func healthCheck(serverURL: URL) async -> Bool {
        do {
            let (_, response) = try await URLSession.shared.data(from: serverURL.appendingPathComponent("health"))
            return (response as? HTTPURLResponse)?.statusCode == 200
        } catch {
            return false
        }
    }

    private func readDiscovery() throws -> AppServerDiscovery {
        let data = try Data(contentsOf: discoveryFileURL())
        let object = try JSONSerialization.jsonObject(with: data) as? [String: Any]
        guard
            let urlString = object?["url"] as? String,
            let serverURL = URL(string: urlString),
            let authToken = object?["auth_token"] as? String
        else {
            throw AppServerClientError.invalidResponse
        }

        return AppServerDiscovery(serverURL: serverURL, authToken: authToken)
    }

    private func resolvedAppServerBinary() -> String? {
        let fileManager = FileManager.default
        let environmentPath = ProcessInfo.processInfo.environment["MANUAL_APP_SERVER_BIN"]

        if let environmentPath, fileManager.isExecutableFile(atPath: environmentPath) {
            return environmentPath
        }

        let bundleURL = Bundle.main.bundleURL
        let repositoryRoot = bundleURL
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
        let discoveredPath = defaultManualAppServerBinaryURL(repositoryRoot: repositoryRoot).path

        if fileManager.isExecutableFile(atPath: discoveredPath) {
            return discoveredPath
        }

        return nil
    }

    private func discoveryFileURL() throws -> URL {
        if let path = ProcessInfo.processInfo.environment["MANUAL_APP_SERVER_DISCOVERY"] {
            return URL(fileURLWithPath: path)
        }

        let applicationSupportURL = try FileManager.default.url(
            for: .applicationSupportDirectory,
            in: .userDomainMask,
            appropriateFor: nil,
            create: true
        )
        let manualURL = applicationSupportURL.appendingPathComponent("Manual", isDirectory: true)
        try FileManager.default.createDirectory(at: manualURL, withIntermediateDirectories: true)
        return manualURL.appendingPathComponent("app-server.json")
    }
}

private struct AppServerDiscovery {
    let serverURL: URL
    let authToken: String
}

struct AppServerLiveEvent: @unchecked Sendable {
    let name: String
    let payload: [String: Any]
}

struct WorkflowSummary: Identifiable, Equatable, Sendable {
    let workflowID: String
    let nodeCount: Int

    var id: String { workflowID }

    init(workflowID: String, nodeCount: Int) {
        self.workflowID = workflowID
        self.nodeCount = nodeCount
    }

    init(_ object: [String: Any]) throws {
        guard
            let workflowID = object["workflow_id"] as? String,
            let nodeCount = object["node_count"] as? Int
        else {
            throw AppServerClientError.invalidResponse
        }

        self.workflowID = workflowID
        self.nodeCount = nodeCount
    }
}

struct WorkflowMutationResult: Sendable {
    let workflowID: String
    let nodeCount: Int

    init(workflowID: String, nodeCount: Int) {
        self.workflowID = workflowID
        self.nodeCount = nodeCount
    }

    init(_ result: Any) throws {
        guard
            let object = result as? [String: Any],
            let workflowID = object["workflow_id"] as? String,
            let nodeCount = object["node_count"] as? Int
        else {
            throw AppServerClientError.invalidResponse
        }

        self.workflowID = workflowID
        self.nodeCount = nodeCount
    }
}

struct WorkflowDeleteResult: Sendable {
    let workflowID: String
    let deleted: Bool

    init(_ result: Any) throws {
        guard
            let object = result as? [String: Any],
            let workflowID = object["workflow_id"] as? String,
            let deleted = object["deleted"] as? Bool
        else {
            throw AppServerClientError.invalidResponse
        }

        self.workflowID = workflowID
        self.deleted = deleted
    }
}

struct WorkflowEventsPage: @unchecked Sendable {
    let events: [[String: Any]]
    let nextCursor: Int
    let completed: Bool
    let run: [String: Any]
    let optimizationReport: WorkflowOptimizationReport?
    let optimizationAnalysis: WorkflowOptimizationAnalysis?
}

struct SandboxListResult: @unchecked Sendable {
    let sandboxes: [SandboxPolicyModel]
    let backends: [String: [String]]
    let currentBackend: String

    init(_ result: Any) throws {
        guard
            let object = result as? [String: Any],
            let sandboxObjects = object["sandboxes"] as? [[String: Any]]
        else {
            throw AppServerClientError.invalidResponse
        }

        self.sandboxes = try sandboxObjects.map(SandboxPolicyModel.init)
        let backendObject = object["backends"] as? [String: Any] ?? [:]
        self.backends = backendObject.reduce(into: [String: [String]]()) { partial, entry in
            if let values = entry.value as? [Any] {
                partial[entry.key] = values.compactMap { $0 as? String }
            }
        }
        self.currentBackend = backendObject["current"] as? String ?? ""
    }
}

private struct SandboxPolicyResult: Sendable {
    let sandbox: SandboxPolicyModel

    init(_ result: Any) throws {
        guard
            let object = result as? [String: Any],
            let sandboxObject = object["sandbox"] as? [String: Any]
        else {
            throw AppServerClientError.invalidResponse
        }

        self.sandbox = try SandboxPolicyModel(sandboxObject)
    }
}

private struct SandboxEvaluationResult: Sendable {
    let decision: SandboxDecisionModel

    init(_ result: Any) throws {
        guard
            let object = result as? [String: Any],
            let decisionObject = object["decision"] as? [String: Any]
        else {
            throw AppServerClientError.invalidResponse
        }

        self.decision = try SandboxDecisionModel(decisionObject)
    }
}
