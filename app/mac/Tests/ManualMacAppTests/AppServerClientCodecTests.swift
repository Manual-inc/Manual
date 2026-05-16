import Testing
@testable import ManualMacApp

@Suite("AppServerClient 코덱")
struct AppServerClientCodecTests {
    @Test("StopWorkflowResult — cancelled=true 응답 파싱")
    func stopResultCancelled() throws {
        let raw: [String: Any] = ["run_id": "run-1", "cancelled": true]
        let result = try StopWorkflowResult(raw)
        #expect(result.runID == "run-1")
        #expect(result.cancelled == true)
        #expect(result.message == nil)
    }

    @Test("StopWorkflowResult — cancelled=false + message 파싱")
    func stopResultNotCancelled() throws {
        let raw: [String: Any] = ["run_id": "run-2", "cancelled": false, "message": "run already completed"]
        let result = try StopWorkflowResult(raw)
        #expect(result.cancelled == false)
        #expect(result.message == "run already completed")
    }

    @Test("StopWorkflowResult — 필드 누락 시 에러")
    func stopResultInvalid() {
        #expect {
            _ = try StopWorkflowResult(["cancelled": true] as [String: Any])
        } throws: { error in
            if case AppServerClientError.invalidResponse = error { return true }
            return false
        }
    }

    @Test("WorkflowEventsPage — resumable/paused/firstFailedNode 파싱")
    func eventsPageSummaryFields() {
        let run: [String: Any] = [
            "run_id": "run-3",
            "status": "failed",
            "first_failed_node": "node_a",
            "resumable": true,
            "paused": false,
        ]
        let page = WorkflowEventsPage(events: [], nextCursor: 0, completed: true, run: run)
        #expect(page.firstFailedNode == "node_a")
        #expect(page.resumable == true)
        #expect(page.paused == false)
    }

    @Test("WorkflowEventsPage — 신규 필드 없어도 기본값")
    func eventsPageDefaults() {
        let run: [String: Any] = ["run_id": "run-4", "status": "running"]
        let page = WorkflowEventsPage(events: [], nextCursor: 0, completed: false, run: run)
        #expect(page.firstFailedNode == nil)
        #expect(page.resumable == false)
        #expect(page.paused == false)
    }

    @Test("WorkflowStartOptions 기본값")
    func startOptionsDefaults() {
        let opts = WorkflowStartOptions()
        #expect(opts.startNodeID == nil)
        #expect(opts.resumeFromFailure == false)
        #expect(opts.inputOverrides.isEmpty)
        #expect(opts.mode == .auto)
        #expect(opts.resumeRunID == nil)
    }

    @Test("ExecutionMode rawValue")
    func executionModeRaw() {
        #expect(ExecutionMode.auto.rawValue == "auto")
        #expect(ExecutionMode.step.rawValue == "step")
    }
}
