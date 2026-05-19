import Testing
@testable import ManualMacApp

@Suite("Workflow Output Formatting")
struct WorkflowOutputTests {
    @Test func workflowOutputText_prefersStdoutForAgentResults() {
        let text = workflowOutputText(from: [
            "status_code": 0,
            "stdout": "No findings.",
            "stderr": "",
        ])

        #expect(text == "No findings.")
    }

    @Test func workflowOutputText_fallsBackToKeyValueRendering() {
        let text = workflowOutputText(from: [
            "qualified": 42,
            "signal": "demo follow-up needed",
        ])

        #expect(text.contains("qualified=42"))
        #expect(text.contains("signal=demo follow-up needed"))
    }

    @Test func starterOutcomeSummary_prefersStarterPrimaryNode() {
        let nodes = [
            WorkflowNodeModel(
                id: "collect_diff",
                title: "Collect Diff",
                subtitle: "Rust Script",
                kind: .script,
                position: .zero,
                result: "diff summary"
            ),
            WorkflowNodeModel(
                id: "review",
                title: "Review",
                subtitle: "Codex Review",
                kind: .codex,
                position: .zero,
                result: "Looks good overall."
            ),
        ]

        let summary = starterOutcomeSummary(
            workflowID: "starter-demo-review",
            runID: "run-1",
            nodes: nodes
        )

        #expect(summary?.workflowID == "starter-demo-review")
        #expect(summary?.label == "Review Output")
        #expect(summary?.text == "Looks good overall.")
        #expect(summary?.rerunCommand == "manual workflow run starter-demo-review --human")
    }

    @Test func starterOutcomeShareText_includesWorkflowAndReusableCommand() {
        let summary = StarterOutcomeSummary(
            workflowID: "starter-demo-summary",
            runID: "run-2",
            label: "Summary Output",
            text: "Changed docs and updated the guide."
        )

        let shareText = starterOutcomeShareText(summary)

        #expect(shareText.contains("Starter Outcome"))
        #expect(shareText.contains("Workflow ID: starter-demo-summary"))
        #expect(shareText.contains("Reusable command: manual workflow run starter-demo-summary --human"))
        #expect(shareText.contains("Changed docs and updated the guide."))
    }

    @Test func starterOutcomeSummary_fromRecentEntry_usesStoredOutcome() {
        let entry = WorkflowStarterRecentEntry(
            presetID: "code-review",
            repositoryRootPath: "/tmp/repo",
            workflowID: "starter-demo-review",
            recommendationReason: "Detected implementation changes",
            outcomeLabel: "Review Output",
            outcomeText: "Looks good overall."
        )

        let summary = starterOutcomeSummary(from: entry)

        #expect(summary?.workflowID == "starter-demo-review")
        #expect(summary?.label == "Review Output")
        #expect(summary?.text == "Looks good overall.")
        #expect(summary?.runID == nil)
    }

    @Test func starterOutcomeShareText_fromRecentEntry_includesStoredOutcome() {
        let entry = WorkflowStarterRecentEntry(
            presetID: "change-summary",
            repositoryRootPath: "/tmp/repo",
            workflowID: "starter-demo-summary",
            recommendationReason: "Detected mostly documentation or markdown changes.",
            outcomeLabel: "Summary Output",
            outcomeText: "Updated the docs and guide."
        )

        let shareText = starterOutcomeShareText(from: entry)

        #expect(shareText?.contains("Starter Outcome") == true)
        #expect(shareText?.contains("Workflow ID: starter-demo-summary") == true)
        #expect(shareText?.contains("Updated the docs and guide.") == true)
    }
}
