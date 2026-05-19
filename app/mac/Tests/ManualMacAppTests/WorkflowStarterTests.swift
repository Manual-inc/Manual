import Testing
@testable import ManualMacApp

struct WorkflowStarterDefinitionTests {
    @Test func availablePresets_includeCodeReviewAndChangeSummary() {
        let presets = WorkflowStarterDefinition.availablePresets

        #expect(presets.map(\.id) == ["code-review", "change-summary", "test-plan"])
    }

    @Test func repositoryDisplayName_usesLastPathComponent() {
        #expect(
            WorkflowStarterDefinition.repositoryDisplayName(repositoryRootPath: "/tmp/My Project Repo")
                == "My Project Repo"
        )
    }

    @Test func recommendedPreset_prefersChangeSummaryForDocsOnlyChanges() {
        let recommendation = WorkflowStarterDefinition.recommendedPreset(
            forChangedFiles: ["docs/guide.md", "README.md"]
        )

        #expect(recommendation.preset.id == "change-summary")
        #expect(recommendation.reason.contains("documentation"))
    }

    @Test func recommendedPreset_prefersTestPlanForCodeChangesWithoutTests() {
        let recommendation = WorkflowStarterDefinition.recommendedPreset(
            forChangedFiles: ["src/lib.rs", "app/main.swift"]
        )

        #expect(recommendation.preset.id == "test-plan")
        #expect(recommendation.reason.contains("without matching test updates"))
    }

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

    @Test func changeSummaryWorkflow_usesSummaryNodeAndPrompt() throws {
        let repositoryRootPath = "/tmp/manual-repo"
        let workflow = try WorkflowStarterDefinition.changeSummaryWorkflow(
            workflowID: "starter-manual-summary",
            repositoryRootPath: repositoryRootPath,
            agent: "codex"
        )

        let nodes = try #require(workflow["nodes"] as? [[String: Any]])
        let summary = try #require(nodes.first(where: { $0["id"] as? String == "summary" }))
        let prompt = try #require(summary["prompt"] as? String)

        #expect(summary["kind"] as? String == "codex")
        #expect(prompt.localizedCaseInsensitiveContains("summarize"))
        #expect(prompt.localizedCaseInsensitiveContains("what changed"))
    }

    @Test func testPlanWorkflow_usesTestPlanNodeAndPrompt() throws {
        let repositoryRootPath = "/tmp/manual-repo"
        let workflow = try WorkflowStarterDefinition.testPlanWorkflow(
            workflowID: "starter-manual-test-plan",
            repositoryRootPath: repositoryRootPath,
            agent: "codex"
        )

        let nodes = try #require(workflow["nodes"] as? [[String: Any]])
        let testPlan = try #require(nodes.first(where: { $0["id"] as? String == "test_plan" }))
        let prompt = try #require(testPlan["prompt"] as? String)

        #expect(testPlan["kind"] as? String == "codex")
        #expect(prompt.localizedCaseInsensitiveContains("automated"))
        #expect(prompt.localizedCaseInsensitiveContains("manual checks"))
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

    @Test func executeChangeSummaryStarter_buildsSummaryWorkflow() async throws {
        let client = StubWorkflowExecutionClient(
            workflows: [],
            agents: [
                AppServerAgentAvailability(name: "codex", available: true, path: "/usr/bin/codex"),
            ],
            nextRunID: "run-starter-2"
        )
        let intent = WorkflowExecutionIntent(client: client)

        let result = try await intent.executeStarter(
            presetID: "change-summary",
            repositoryRootPath: "/tmp/starter-repo"
        )

        #expect(result.workflowID == "starter-starter-repo-summary")
        #expect(result.runID == "run-starter-2")
        let createdWorkflow = try #require(client.createdWorkflow)
        let nodes = try #require(createdWorkflow["nodes"] as? [[String: Any]])
        let summary = try #require(nodes.first(where: { $0["id"] as? String == "summary" }))
        #expect(summary["kind"] as? String == "codex")
    }

    @Test func executeTestPlanStarter_buildsTestPlanWorkflow() async throws {
        let client = StubWorkflowExecutionClient(
            workflows: [],
            agents: [
                AppServerAgentAvailability(name: "codex", available: true, path: "/usr/bin/codex"),
            ],
            nextRunID: "run-starter-3"
        )
        let intent = WorkflowExecutionIntent(client: client)

        let result = try await intent.executeStarter(
            presetID: "test-plan",
            repositoryRootPath: "/tmp/starter-repo"
        )

        #expect(result.workflowID == "starter-starter-repo-test-plan")
        #expect(result.runID == "run-starter-3")
        let createdWorkflow = try #require(client.createdWorkflow)
        let nodes = try #require(createdWorkflow["nodes"] as? [[String: Any]])
        let testPlan = try #require(nodes.first(where: { $0["id"] as? String == "test_plan" }))
        #expect(testPlan["kind"] as? String == "codex")
    }

    @Test func executeRecommendedStarter_returnsPresetAndReasonMetadata() async throws {
        let client = StubWorkflowExecutionClient(
            workflows: [],
            agents: [
                AppServerAgentAvailability(name: "codex", available: true, path: "/usr/bin/codex"),
            ],
            nextRunID: "run-starter-4"
        )
        let intent = WorkflowExecutionIntent(client: client)

        let result = try await intent.executeRecommendedStarter(
            repositoryRootPath: "/tmp/docs-repo",
            changedFiles: ["docs/guide.md", "README.md"]
        )

        #expect(result.workflowID == "starter-docs-repo-summary")
        #expect(result.runID == "run-starter-4")
        #expect(result.starterPresetID == "change-summary")
        #expect(result.starterRecommendationReason?.contains("documentation") == true)
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
