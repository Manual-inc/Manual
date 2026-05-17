import Foundation

// See docs/wiki/architecture/manual-app-architecture.md: UI commands share this intent so headless tests exercise the same app-server path.
public struct WorkflowExecutionIntentResult: Sendable {
    public let workflowID: String
    public let runID: String
}

@MainActor
public final class WorkflowExecutionIntent {
    private let client: AppServerClient

    public convenience init() {
        self.init(client: AppServerClient())
    }

    init(client: AppServerClient) {
        self.client = client
    }

    public func executeExampleWorkflow() async throws -> WorkflowExecutionIntentResult {
        let workflow = BusinessWorkflowExample.jsonDefinition
        let workflowID = workflow["id"] as? String ?? BusinessWorkflowExample.workflowID
        try await upsert(workflow: workflow, workflowID: workflowID)
        let runID = try await client.startWorkflow(id: workflowID)
        return WorkflowExecutionIntentResult(workflowID: workflowID, runID: runID)
    }

    func execute(workflow: [String: Any], knownWorkflows: [WorkflowSummary]) async throws -> WorkflowExecutionIntentResult {
        let workflowID = workflow["id"] as? String ?? BusinessWorkflowExample.workflowID
        if knownWorkflows.contains(where: { $0.workflowID == workflowID }) {
            _ = try await client.updateWorkflow(id: workflowID, workflow: workflow)
        } else {
            _ = try await client.createWorkflow(workflow)
        }
        let runID = try await client.startWorkflow(id: workflowID)
        return WorkflowExecutionIntentResult(workflowID: workflowID, runID: runID)
    }

    private func upsert(workflow: [String: Any], workflowID: String) async throws {
        let summaries = try await client.workflows()
        if summaries.contains(where: { $0.workflowID == workflowID }) {
            _ = try await client.updateWorkflow(id: workflowID, workflow: workflow)
        } else {
            _ = try await client.createWorkflow(workflow)
        }
    }
}
