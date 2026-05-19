import Darwin
import Foundation
import ManualMacApp

// See docs/wiki/architecture/manual-app-architecture.md: headless cucumber calls the same UI intent used by mac controls.
final class MacAppUIDriver {
    private let appServer: AppServerScenarioDriver

    private enum StarterRepositoryKind {
        case generic
        case docsOnly
    }

    init(appServer: AppServerScenarioDriver) {
        self.appServer = appServer
    }

    func launch() throws {
        for (key, value) in try appServer.clientEnvironment {
            setenv(key, value, 1)
        }
    }

    func chooseExecuteWorkflowFromUI() throws -> WorkflowExecutionIntentResult {
        try launch()
        let semaphore = DispatchSemaphore(value: 0)
        let box = AsyncResultBox<WorkflowExecutionIntentResult>()

        Task { @MainActor in
            do {
                box.result = .success(try await WorkflowExecutionIntent().executeExampleWorkflow())
            } catch {
                box.result = .failure(error)
            }
            semaphore.signal()
        }

        semaphore.wait()
        return try box.result!.get()
    }

    func chooseCreateCodeReviewStarterFromUI() throws -> WorkflowExecutionIntentResult {
        try launch()
        let repositoryPath = try temporaryStarterRepository(kind: .generic)
        let semaphore = DispatchSemaphore(value: 0)
        let box = AsyncResultBox<WorkflowExecutionIntentResult>()

        Task { @MainActor in
            do {
                box.result = .success(
                    try await WorkflowExecutionIntent().executeCodeReviewStarter(
                        repositoryRootPath: repositoryPath
                    )
                )
            } catch {
                box.result = .failure(error)
            }
            semaphore.signal()
        }

        semaphore.wait()
        return try box.result!.get()
    }

    func chooseCreateRecommendedStarterFromUI() throws -> WorkflowExecutionIntentResult {
        try launch()
        let repositoryPath = try temporaryStarterRepository(kind: .docsOnly)
        let semaphore = DispatchSemaphore(value: 0)
        let box = AsyncResultBox<WorkflowExecutionIntentResult>()

        Task { @MainActor in
            do {
                box.result = .success(
                    try await WorkflowExecutionIntent().executeRecommendedStarter(
                        repositoryRootPath: repositoryPath
                    )
                )
            } catch {
                box.result = .failure(error)
            }
            semaphore.signal()
        }

        semaphore.wait()
        return try box.result!.get()
    }

    func chooseRerunRecentStarterFromUI() throws -> WorkflowExecutionIntentResult {
        try launch()
        let recent = try latestSharedRecentStarter()
        let semaphore = DispatchSemaphore(value: 0)
        let box = AsyncResultBox<WorkflowExecutionIntentResult>()

        Task { @MainActor in
            do {
                box.result = .success(
                    try await WorkflowExecutionIntent().executeStarter(
                        presetID: recent.presetID,
                        repositoryRootPath: recent.repositoryRootPath
                    )
                )
            } catch {
                box.result = .failure(error)
            }
            semaphore.signal()
        }

        semaphore.wait()
        return try box.result!.get()
    }

    private func temporaryStarterRepository(kind: StarterRepositoryKind) throws -> String {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent("manual-mac-starter-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)

        try runGit(["init", "-q"], in: root)
        try runGit(["config", "user.email", "starter@example.com"], in: root)
        try runGit(["config", "user.name", "Starter"], in: root)

        let relativePath: String
        switch kind {
        case .generic:
            relativePath = "note.txt"
        case .docsOnly:
            let docsURL = root.appendingPathComponent("docs", isDirectory: true)
            try FileManager.default.createDirectory(at: docsURL, withIntermediateDirectories: true)
            relativePath = "docs/guide.md"
        }

        let fileURL = root.appendingPathComponent(relativePath)
        try "hello\n".write(to: fileURL, atomically: true, encoding: .utf8)
        try runGit(["add", relativePath], in: root)
        try runGit(["commit", "-q", "-m", "init"], in: root)
        try "hello world\n".write(to: fileURL, atomically: true, encoding: .utf8)
        return root.path
    }

    private func latestSharedRecentStarter() throws -> (presetID: String, repositoryRootPath: String, workflowID: String) {
        let response = try appServer.rpc(method: "starter.list", params: ["limit": 1])
        guard
            let result = response["result"] as? [String: Any],
            let starters = result["starters"] as? [[String: Any]],
            let starter = starters.first,
            let presetID = starter["preset_id"] as? String,
            let repositoryRootPath = starter["repository_root"] as? String,
            let workflowID = starter["workflow_id"] as? String
        else {
            throw StepError.assertion("shared recent starter history should include at least one starter")
        }

        return (
            presetID: presetID,
            repositoryRootPath: repositoryRootPath,
            workflowID: workflowID
        )
    }

    private func runGit(_ arguments: [String], in repositoryURL: URL) throws {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
        process.arguments = ["git"] + arguments
        process.currentDirectoryURL = repositoryURL
        process.standardOutput = nil
        process.standardError = nil
        try process.run()
        process.waitUntilExit()
        guard process.terminationStatus == 0 else {
            throw StepError.assertion("starter test git command failed: \(arguments.joined(separator: " "))")
        }
    }
}

private final class AsyncResultBox<T>: @unchecked Sendable {
    var result: Result<T, Error>?
}
