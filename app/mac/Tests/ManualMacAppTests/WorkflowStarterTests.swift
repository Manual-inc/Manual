import Testing
@testable import ManualMacApp

struct WorkflowStarterDefinitionTests {
    @Test func suggestedWorkflowID_sanitizesRepositoryName() throws {
        let repositoryRootPath = "/tmp/My Cool.Repo"

        #expect(
            WorkflowStarterDefinition.suggestedWorkflowID(repositoryRootPath: repositoryRootPath)
                == "starter-my-cool-repo-review"
        )
    }

    @Test func codeReviewWorkflow_usesBoundedDiffScriptAndReviewNode() throws {
        let repositoryRootPath = "/tmp/manual-repo"
        let workflow = try WorkflowStarterDefinition.codeReviewWorkflow(
            workflowID: "starter-manual-review",
            repositoryRootPath: repositoryRootPath,
            agent: "codex"
        )

        let nodes = try #require(workflow["nodes"] as? [[String: Any]])
        let collectDiff = try #require(nodes.first(where: { $0["id"] as? String == "collect_diff" }))
        let review = try #require(nodes.first(where: { $0["id"] as? String == "review" }))
        let script = try #require(collectDiff["script"] as? String)

        #expect(script.contains("--- FILE SUMMARY ---"))
        #expect(script.contains("PATCH TRUNCATED AFTER 220 LINES"))
        #expect(review["kind"] as? String == "codex")
        #expect(review["cwd"] as? String == repositoryRootPath)
    }
}

@MainActor
struct WorkflowStarterIntentTests {
    @Test func executeCodeReviewStarter_picksFirstAvailableAgent_andStartsWorkflow() async throws {
        let client = StubWorkflowExecutionClient(
            workflows: [WorkflowSummary(workflowID: "business-pipeline-health", nodeCount: 6)],
            agents: [
                AppServerAgentAvailability(name: "codex", available: false, path: nil),
                AppServerAgentAvailability(name: "claude", available: true, path: "/usr/bin/claude"),
            ],
            nextRunID: "run-starter-1"
        )
        let intent = WorkflowExecutionIntent(client: client)

        let result = try await intent.executeCodeReviewStarter(repositoryRootPath: "/tmp/starter-repo")

        #expect(result.workflowID == "starter-starter-repo-review")
        #expect(result.runID == "run-starter-1")
        #expect(client.startedWorkflowID == "starter-starter-repo-review")

        let createdWorkflow = try #require(client.createdWorkflow)
        let nodes = try #require(createdWorkflow["nodes"] as? [[String: Any]])
        let review = try #require(nodes.first(where: { $0["id"] as? String == "review" }))
        #expect(review["kind"] as? String == "claude")
    }
}

@MainActor
private final class StubWorkflowExecutionClient: WorkflowExecutionClient {
    let workflowsResult: [WorkflowSummary]
    let agentsResult: [AppServerAgentAvailability]
    let nextRunID: String

    private(set) var createdWorkflow: [String: Any]?
    private(set) var updatedWorkflow: [String: Any]?
    private(set) var startedWorkflowID: String?

    init(
        workflows: [WorkflowSummary],
        agents: [AppServerAgentAvailability],
        nextRunID: String
    ) {
        self.workflowsResult = workflows
        self.agentsResult = agents
        self.nextRunID = nextRunID
    }

    func workflows() async throws -> [WorkflowSummary] { workflowsResult }

    func createWorkflow(_ workflow: [String: Any]) async throws -> WorkflowMutationResult {
        createdWorkflow = workflow
        return WorkflowMutationResult(
            workflowID: workflow["id"] as? String ?? "starter-workflow",
            nodeCount: (workflow["nodes"] as? [[String: Any]])?.count ?? 0
        )
    }

    func updateWorkflow(id workflowID: String, workflow: [String: Any]) async throws -> WorkflowMutationResult {
        updatedWorkflow = workflow
        return WorkflowMutationResult(
            workflowID: workflowID,
            nodeCount: (workflow["nodes"] as? [[String: Any]])?.count ?? 0
        )
    }

    func startWorkflow(id workflowID: String) async throws -> String {
        startedWorkflowID = workflowID
        return nextRunID
    }

    func availableAgents() async throws -> [AppServerAgentAvailability] {
        agentsResult
    }
}
