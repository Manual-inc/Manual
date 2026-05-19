import Foundation

// See docs/wiki/architecture/manual-app-architecture.md: UI commands share this intent so headless tests exercise the same app-server path.
public struct WorkflowExecutionIntentResult: Sendable {
    public let workflowID: String
    public let runID: String
}

@MainActor
protocol WorkflowExecutionClient: AnyObject {
    func workflows() async throws -> [WorkflowSummary]
    func createWorkflow(_ workflow: [String: Any]) async throws -> WorkflowMutationResult
    func updateWorkflow(id workflowID: String, workflow: [String: Any]) async throws -> WorkflowMutationResult
    func startWorkflow(id workflowID: String) async throws -> String
    func availableAgents() async throws -> [AppServerAgentAvailability]
}

extension AppServerClient: WorkflowExecutionClient {}

@MainActor
public final class WorkflowExecutionIntent {
    private let client: any WorkflowExecutionClient

    public convenience init() {
        self.init(client: AppServerClient())
    }

    init(client: any WorkflowExecutionClient) {
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

    public func executeCodeReviewStarter(repositoryRootPath: String) async throws -> WorkflowExecutionIntentResult {
        try await executeStarter(presetID: "code-review", repositoryRootPath: repositoryRootPath)
    }

    public func executeChangeSummaryStarter(repositoryRootPath: String) async throws -> WorkflowExecutionIntentResult {
        try await executeStarter(presetID: "change-summary", repositoryRootPath: repositoryRootPath)
    }

    public func executeStarter(
        presetID: String,
        repositoryRootPath: String
    ) async throws -> WorkflowExecutionIntentResult {
        // See docs/wiki/features/workflow-starters.md: mac UI should offer the
        // same first-success starter path as the CLI surface.
        let workflowID = WorkflowStarterDefinition.suggestedWorkflowID(
            repositoryRootPath: repositoryRootPath,
            presetID: presetID
        )
        let agents = try await client.availableAgents()
        guard let agent = WorkflowStarterDefinition.preferredAgent(from: agents) else {
            throw WorkflowStarterError.noAvailableAgent
        }
        let workflow: [String: Any]
        switch presetID {
        case "code-review":
            workflow = try WorkflowStarterDefinition.codeReviewWorkflow(
                workflowID: workflowID,
                repositoryRootPath: repositoryRootPath,
                agent: agent
            )
        case "change-summary":
            workflow = try WorkflowStarterDefinition.changeSummaryWorkflow(
                workflowID: workflowID,
                repositoryRootPath: repositoryRootPath,
                agent: agent
            )
        default:
            throw WorkflowStarterError.unsupportedPreset(presetID)
        }
        let knownWorkflows = try await client.workflows()
        return try await execute(workflow: workflow, knownWorkflows: knownWorkflows)
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
