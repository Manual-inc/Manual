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
    private var input: FileHandle?
    private var output: FileHandle?
    private var nextID = 1

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
                "workflow": workflow
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

    func events(runID: String, cursor: Int) async throws -> WorkflowEventsPage {
        let result = try await request(
            method: "workflow.events",
            params: [
                "run_id": runID,
                "cursor": cursor
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

        return WorkflowEventsPage(events: events, nextCursor: nextCursor, completed: completed, run: run)
    }

    private func request(method: String, params: [String: Any]) async throws -> Any {
        try launchIfNeeded()

        let requestID = nextID
        nextID += 1

        let payload: [String: Any] = [
            "jsonrpc": "2.0",
            "id": requestID,
            "method": method,
            "params": params
        ]

        let requestData = try JSONSerialization.data(withJSONObject: payload)
        guard let input, let output else {
            throw AppServerClientError.binaryNotFound
        }

        input.write(requestData)
        input.write(Data([0x0A]))

        let responseData = output.readLineData()
        guard !responseData.isEmpty else {
            throw AppServerClientError.emptyResponse
        }

        let response = try JSONSerialization.jsonObject(with: responseData)
        guard let object = response as? [String: Any] else {
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

    private func launchIfNeeded() throws {
        if process?.isRunning == true {
            return
        }

        guard let binary = resolvedAppServerBinary() else {
            throw AppServerClientError.binaryNotFound
        }

        let process = Process()
        let stdin = Pipe()
        let stdout = Pipe()
        let workflowStorageURL = try workflowStorageDirectory()

        process.executableURL = URL(fileURLWithPath: binary)
        process.currentDirectoryURL = workflowStorageURL
        process.environment = ProcessInfo.processInfo.environment.merging(
            ["MANUAL_RS_WORKFLOW_DIR": workflowStorageURL.path],
            uniquingKeysWith: { _, newValue in newValue }
        )
        process.standardInput = stdin
        process.standardOutput = stdout
        process.standardError = FileHandle.standardError

        try process.run()

        self.process = process
        self.input = stdin.fileHandleForWriting
        self.output = stdout.fileHandleForReading
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
        let discoveredPath = repositoryRoot
            .appendingPathComponent("manual-rs/target/debug/app-server")
            .path

        if fileManager.isExecutableFile(atPath: discoveredPath) {
            return discoveredPath
        }

        return nil
    }

    private func workflowStorageDirectory() throws -> URL {
        let applicationSupportURL = try FileManager.default.url(
            for: .applicationSupportDirectory,
            in: .userDomainMask,
            appropriateFor: nil,
            create: true
        )
        let storageURL = applicationSupportURL
            .appendingPathComponent("ManualMac", isDirectory: true)
            .appendingPathComponent("workflows", isDirectory: true)

        try FileManager.default.createDirectory(
            at: storageURL,
            withIntermediateDirectories: true
        )

        return storageURL
    }
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
}

private extension FileHandle {
    func readLineData() -> Data {
        var data = Data()

        while true {
            let byte = readData(ofLength: 1)
            if byte.isEmpty || byte == Data([0x0A]) {
                break
            }
            data.append(byte)
        }

        return data
    }
}
