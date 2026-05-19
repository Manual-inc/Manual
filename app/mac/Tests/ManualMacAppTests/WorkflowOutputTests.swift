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
}
