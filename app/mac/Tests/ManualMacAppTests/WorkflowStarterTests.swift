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

    @Test func recentStarterEntries_roundTripAndKeepNewestFirst() {
        let first = WorkflowStarterRecentEntry(
            presetID: "code-review",
            repositoryRootPath: "/tmp/repo-a",
            workflowID: "starter-repo-a-review",
            recommendationReason: "Detected implementation changes",
            outcomeLabel: "Review Output",
            outcomeText: "Looks good overall."
        )
        let second = WorkflowStarterRecentEntry(
            presetID: "test-plan",
            repositoryRootPath: "/tmp/repo-b",
            workflowID: "starter-repo-b-test-plan"
        )

        let updated = WorkflowStarterDefinition.updatedRecentEntries([], with: first)
        let reordered = WorkflowStarterDefinition.updatedRecentEntries(updated, with: second)
        let movedToFront = WorkflowStarterDefinition.updatedRecentEntries(reordered, with: first)
        let encoded = WorkflowStarterDefinition.encodeRecentEntries(movedToFront)
        let decoded = WorkflowStarterDefinition.recentEntries(from: encoded)

        #expect(decoded.map(\.workflowID) == [
            "starter-repo-a-review",
            "starter-repo-b-test-plan",
        ])
        #expect(decoded.first?.recommendationReason == "Detected implementation changes")
        #expect(decoded.first?.outcomeLabel == "Review Output")
        #expect(decoded.first?.outcomeText == "Looks good overall.")
    }

    @Test func mergedRecentEntries_prefersSharedOrder_andDeduplicatesLocalEntries() {
        let local = [
            WorkflowStarterRecentEntry(
                presetID: "code-review",
                repositoryRootPath: "/tmp/repo-a",
                workflowID: "starter-repo-a-review-local"
            ),
            WorkflowStarterRecentEntry(
                presetID: "test-plan",
                repositoryRootPath: "/tmp/repo-b",
                workflowID: "starter-repo-b-test-plan"
            ),
        ]
        let shared = [
            WorkflowStarterRecentEntry(
                presetID: "change-summary",
                repositoryRootPath: "/tmp/repo-c",
                workflowID: "starter-repo-c-summary",
                recommendationReason: "Detected mostly documentation or markdown changes.",
                outcomeLabel: "Summary Output",
                outcomeText: "Updated the docs and guide."
            ),
            WorkflowStarterRecentEntry(
                presetID: "code-review",
                repositoryRootPath: "/tmp/repo-a",
                workflowID: "starter-repo-a-review-shared"
            ),
        ]

        let merged = WorkflowStarterDefinition.mergedRecentEntries(local: local, shared: shared)

        #expect(merged.map(\.workflowID) == [
            "starter-repo-c-summary",
            "starter-repo-a-review-shared",
            "starter-repo-b-test-plan",
        ])
        #expect(merged.first?.recommendationReason == "Detected mostly documentation or markdown changes.")
        #expect(merged.first?.outcomeLabel == "Summary Output")
        #expect(merged.first?.outcomeText == "Updated the docs and guide.")
    }

    @Test func recommendedPreset_prefersChangeSummaryForDocsOnlyChanges() {
        let recommendation = WorkflowStarterDefinition.recommendedPreset(
            forChangedFiles: ["docs/guide.md", "README.md"]
        )

        #expect(recommendation.preset.id == "change-summary")
        #expect(recommendation.reason.contains("documentation"))
    }

    @Test func recommendedStarterPreview_surfacesReasonAndOutcomeForDocsChanges() {
        let preview = WorkflowStarterDefinition.recommendedStarterPreview(
            forChangedFiles: ["docs/guide.md", "README.md"]
        )

        #expect(preview.preset.id == "change-summary")
        #expect(preview.reason.contains("documentation"))
        #expect(preview.changedFilesHint.contains("docs/guide.md"))
        #expect(preview.expectedOutcome.contains("change update"))
    }

    @Test func recommendedStarterSelectionSummary_explainsHowManualChooses() {
        let summary = WorkflowStarterDefinition.recommendedStarterSelectionSummary()

        #expect(summary.contains("Docs-only changes"))
        #expect(summary.contains("Change Summary"))
        #expect(summary.contains("Code without matching tests"))
        #expect(summary.contains("Test Plan"))
        #expect(summary.contains("otherwise"))
        #expect(summary.contains("Code Review"))
    }

    @Test func recommendedPreset_prefersTestPlanForCodeChangesWithoutTests() {
        let recommendation = WorkflowStarterDefinition.recommendedPreset(
            forChangedFiles: ["src/lib.rs", "app/main.swift"]
        )

        #expect(recommendation.preset.id == "test-plan")
        #expect(recommendation.reason.contains("without matching test updates"))
    }

    @Test func recommendedStarterPreview_surfacesReasonAndOutcomeForCodeChangesWithoutTests() {
        let preview = WorkflowStarterDefinition.recommendedStarterPreview(
            forChangedFiles: ["src/lib.rs", "app/main.swift"]
        )

        #expect(preview.preset.id == "test-plan")
        #expect(preview.reason.contains("without matching test updates"))
        #expect(preview.changedFilesHint.contains("src/lib.rs"))
        #expect(preview.expectedOutcome.contains("test plan"))
    }

    @Test func changedFilesHint_capsListAndShowsOverflowCount() {
        let hint = WorkflowStarterDefinition.changedFilesHint(
            forChangedFiles: ["docs/a.md", "docs/b.md", "docs/c.md"]
        )

        #expect(hint.contains("docs/a.md"))
        #expect(hint.contains("docs/b.md"))
        #expect(hint.contains("+1 more"))
    }

    @Test func suggestedWorkflowID_sanitizesRepositoryName() throws {
        let repositoryRootPath = "/tmp/My Cool.Repo"

        #expect(
            WorkflowStarterDefinition.suggestedWorkflowID(repositoryRootPath: repositoryRootPath)
                == "starter-my-cool-repo-review"
        )
    }

    @Test func availablePresets_exposeExpectedOutcomeSummaries() {
        let review = try! #require(WorkflowStarterDefinition.availablePresets.first(where: { $0.id == "code-review" }))
        let summary = try! #require(WorkflowStarterDefinition.availablePresets.first(where: { $0.id == "change-summary" }))
        let testPlan = try! #require(WorkflowStarterDefinition.availablePresets.first(where: { $0.id == "test-plan" }))

        #expect(review.expectedOutcome.contains("bugs"))
        #expect(summary.expectedOutcome.contains("change update"))
        #expect(testPlan.expectedOutcome.contains("test plan"))
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

    @Test func executeCodeReviewStarter_recordsSharedRecentStarter() async throws {
        let client = StubWorkflowExecutionClient(
            workflows: [],
            agents: [
                AppServerAgentAvailability(name: "codex", available: true, path: "/usr/bin/codex"),
            ],
            nextRunID: "run-starter-5"
        )
        let intent = WorkflowExecutionIntent(client: client)

        _ = try await intent.executeCodeReviewStarter(repositoryRootPath: "/tmp/starter-repo")

        #expect(client.recordedStarter?.presetID == "code-review")
        #expect(client.recordedStarter?.repositoryRootPath == "/tmp/starter-repo")
        #expect(client.recordedStarter?.workflowID == "starter-starter-repo-review")
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
    private(set) var recordedStarter: WorkflowStarterRecentEntry?

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

    func recordStarter(_ entry: WorkflowStarterRecentEntry, recommendationReason: String?) async throws {
        recordedStarter = entry
    }
}
