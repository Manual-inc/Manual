import Darwin
import Foundation
import ManualMacApp

// See docs/wiki/architecture/manual-app-architecture.md: headless cucumber calls the same UI intent used by mac controls.
final class MacAppUIDriver {
    private let appServer: AppServerScenarioDriver

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
}

private final class AsyncResultBox<T>: @unchecked Sendable {
    var result: Result<T, Error>?
}
