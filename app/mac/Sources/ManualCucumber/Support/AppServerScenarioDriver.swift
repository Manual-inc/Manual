import Foundation
import ManualMacApp

// See docs/wiki/systems/기능-계약-테스트.md for why mac cucumber steps must exercise app-server, not only in-memory fixtures.
final class AppServerScenarioDriver {
    private let fileManager = FileManager.default
    private let repositoryRoot: URL
    private let storageURL: URL
    private let discoveryURL: URL
    private let token: String
    private var process: Process?
    private var serverURL: URL?
    private var nextID = 1

    private(set) var evidence: [String] = []

    var clientEnvironment: [String: String] {
        get throws {
            try ensureStarted()
            return [
                "MANUAL_APP_SERVER_DISCOVERY": discoveryURL.path,
                "MANUAL_RS_WORKFLOW_DIR": stateURL.path,
            ]
        }
    }

    var macPackageURL: URL {
        repositoryRoot.appendingPathComponent("app/mac", isDirectory: true)
    }

    private var stateURL: URL {
        storageURL.appendingPathComponent("state", isDirectory: true)
    }

    init() {
        repositoryRoot = Self.resolveRepositoryRoot()
        let unique = "manual-mac-cucumber-\(ProcessInfo.processInfo.processIdentifier)-\(UUID().uuidString)"
        storageURL = fileManager.temporaryDirectory.appendingPathComponent(unique, isDirectory: true)
        discoveryURL = storageURL.appendingPathComponent("app-server.json")
        token = UUID().uuidString.replacingOccurrences(of: "-", with: "")
    }

    deinit {
        process?.terminate()
    }

    func rpc(method: String, params: [String: Any] = [:]) throws -> [String: Any] {
        let response = try rpcAllowingErrors(method: method, params: params)
        if let error = response["error"] as? [String: Any] {
            let message = error["message"] as? String ?? "unknown JSON-RPC error"
            throw StepError.assertion("app-server RPC failed for \(method): \(message)")
        }
        return response
    }

    func rpcAllowingErrors(method: String, params: [String: Any] = [:]) throws -> [String: Any] {
        try ensureStarted()
        guard let serverURL else {
            throw StepError.assertion("app-server URL was not discovered")
        }

        let payload: [String: Any] = [
            "jsonrpc": "2.0",
            "id": nextID,
            "method": method,
            "params": params,
        ]
        nextID += 1

        var request = URLRequest(url: serverURL.appendingPathComponent("rpc"))
        request.httpMethod = "POST"
        request.addValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        request.addValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONSerialization.data(withJSONObject: payload, options: [])

        let (data, response) = try synchronousData(for: request)

        guard (response as? HTTPURLResponse)?.statusCode == 200 else {
            throw StepError.assertion("app-server HTTP response was not 200 for \(method)")
        }
        guard
            let object = try JSONSerialization.jsonObject(with: data) as? [String: Any]
        else {
            throw StepError.assertion("app-server response was not a JSON object for \(method)")
        }

        evidence.append(method)
        return object
    }

    func poll(
        method: String,
        params: [String: Any],
        timeout: TimeInterval = 3,
        until predicate: ([String: Any]) -> Bool
    ) throws -> [String: Any] {
        let deadline = Date().addingTimeInterval(timeout)
        var latest: [String: Any] = [:]

        repeat {
            latest = try rpc(method: method, params: params)
            if predicate(latest) {
                return latest
            }
            Thread.sleep(forTimeInterval: 0.02)
        } while Date() < deadline

        throw StepError.assertion("timed out waiting for app-server \(method)")
    }

    private func ensureStarted() throws {
        if let serverURL, try healthCheck(serverURL) {
            return
        }

        try fileManager.createDirectory(at: storageURL, withIntermediateDirectories: true)
        try fileManager.createDirectory(at: stateURL, withIntermediateDirectories: true)
        let binary = try resolvedAppServerBinary()

        let process = Process()
        process.executableURL = binary
        process.arguments = [
            "--listen", "127.0.0.1:0",
            "--auth-token", token,
            "--discovery-file", discoveryURL.path,
        ]
        var environment = ProcessInfo.processInfo.environment
        environment["MANUAL_RS_WORKFLOW_DIR"] = stateURL.path
        process.environment = environment
        process.currentDirectoryURL = repositoryRoot.appendingPathComponent("manual-rs", isDirectory: true)
        process.standardInput = nil
        process.standardOutput = nil
        process.standardError = FileHandle.standardError

        try process.run()
        self.process = process

        let deadline = Date().addingTimeInterval(5)
        while Date() < deadline {
            if
                let discovery = try? readDiscovery(),
                try healthCheck(discovery.serverURL)
            {
                serverURL = discovery.serverURL
                return
            }
            Thread.sleep(forTimeInterval: 0.05)
        }

        throw StepError.assertion("app-server did not become healthy")
    }

    private func resolvedAppServerBinary() throws -> URL {
        if
            let path = ProcessInfo.processInfo.environment["MANUAL_APP_SERVER_BIN"],
            fileManager.isExecutableFile(atPath: path)
        {
            return URL(fileURLWithPath: path)
        }

        let binary = defaultManualAppServerBinaryURL(repositoryRoot: repositoryRoot)
        if fileManager.isExecutableFile(atPath: binary.path) {
            return binary
        }

        try buildAppServer()
        if fileManager.isExecutableFile(atPath: binary.path) {
            return binary
        }

        throw StepError.assertion("app-server binary was not found after cargo build")
    }

    private func buildAppServer() throws {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
        process.arguments = ["cargo", "build", "-p", "app-server"]
        process.currentDirectoryURL = repositoryRoot.appendingPathComponent("manual-rs", isDirectory: true)
        process.standardInput = nil
        process.standardOutput = FileHandle.standardOutput
        process.standardError = FileHandle.standardError
        try process.run()
        process.waitUntilExit()

        guard process.terminationStatus == 0 else {
            throw StepError.assertion("cargo build -p app-server failed")
        }
    }

    private func healthCheck(_ serverURL: URL) throws -> Bool {
        var request = URLRequest(url: serverURL.appendingPathComponent("health"))
        request.timeoutInterval = 1
        let (_, response) = try synchronousData(for: request)
        return (response as? HTTPURLResponse)?.statusCode == 200
    }

    private func readDiscovery() throws -> (serverURL: URL, authToken: String) {
        let data = try Data(contentsOf: discoveryURL)
        guard
            let object = try JSONSerialization.jsonObject(with: data) as? [String: Any],
            let urlString = object["url"] as? String,
            let serverURL = URL(string: urlString),
            let authToken = object["auth_token"] as? String
        else {
            throw StepError.assertion("app-server discovery file was invalid")
        }
        return (serverURL, authToken)
    }

    private static func resolveRepositoryRoot() -> URL {
        URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent() // Support
            .deletingLastPathComponent() // ManualCucumber
            .deletingLastPathComponent() // Sources
            .deletingLastPathComponent() // mac
            .deletingLastPathComponent() // app
            .deletingLastPathComponent() // Manual
    }

    func workflowRunIDs() -> Set<String> {
        let runsURL = stateURL.appendingPathComponent("runs", isDirectory: true)
        guard let files = try? fileManager.contentsOfDirectory(at: runsURL, includingPropertiesForKeys: nil) else {
            return []
        }

        return Set(files.compactMap { file in
            guard file.pathExtension == "json" else { return nil }
            return Self.hexDecodedString(file.deletingPathExtension().lastPathComponent)
        })
    }

    func waitForNewWorkflowRunID(after existing: Set<String>, timeout: TimeInterval = 5) throws -> String {
        let deadline = Date().addingTimeInterval(timeout)
        repeat {
            let created = workflowRunIDs().subtracting(existing).sorted()
            if let runID = created.first {
                return runID
            }
            Thread.sleep(forTimeInterval: 0.05)
        } while Date() < deadline

        throw StepError.assertion("UI action did not create a new app-server workflow run")
    }

    private static func hexDecodedString(_ hex: String) -> String? {
        guard hex.count.isMultiple(of: 2) else { return nil }
        var bytes: [UInt8] = []
        var index = hex.startIndex
        while index < hex.endIndex {
            let next = hex.index(index, offsetBy: 2)
            guard let byte = UInt8(hex[index..<next], radix: 16) else { return nil }
            bytes.append(byte)
            index = next
        }
        return String(bytes: bytes, encoding: .utf8)
    }
}

private func synchronousData(for request: URLRequest) throws -> (Data, URLResponse) {
    let semaphore = DispatchSemaphore(value: 0)
    nonisolated(unsafe) var result: Result<(Data, URLResponse), Error>?

    let task = URLSession.shared.dataTask(with: request) { data, response, error in
        if let error {
            result = .failure(error)
        } else if let data, let response {
            result = .success((data, response))
        } else {
            result = .failure(StepError.assertion("URLSession returned no data and no response"))
        }
        semaphore.signal()
    }
    task.resume()

    semaphore.wait()
    return try result!.get()
}
