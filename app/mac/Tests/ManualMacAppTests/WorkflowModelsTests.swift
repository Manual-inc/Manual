import Testing
@testable import ManualMacApp

@Suite("WorkflowModels")
struct WorkflowModelsTests {
    @Test("WorkflowNodeStatus 신규 케이스 rawValue")
    func newStatusRawValues() {
        #expect(WorkflowNodeStatus.skipped.rawValue == "Skipped")
        #expect(WorkflowNodeStatus.paused.rawValue == "Paused")
        #expect(WorkflowNodeStatus.cancelled.rawValue == "Cancelled")
    }

    @Test("WorkflowNodeStatus symbolName 모든 케이스 반환값 있음")
    func allSymbolNames() {
        let statuses: [WorkflowNodeStatus] = [.idle, .running, .succeeded, .failed, .skipped, .paused, .cancelled]
        for status in statuses {
            #expect(!status.symbolName.isEmpty)
        }
    }

    @Test("WorkflowNodeModel previousResult 기본값 nil")
    func previousResultDefault() {
        let node = WorkflowNodeModel(
            id: "n1", title: "Test", subtitle: "sub",
            kind: .script, position: .zero
        )
        #expect(node.previousResult == nil)
        #expect(node.inputOverride == nil)
    }

    @Test("WorkflowNodeModel Equatable은 inputOverride 무시")
    func equatableIgnoresInputOverride() {
        var a = WorkflowNodeModel(id: "n1", title: "T", subtitle: "s", kind: .script, position: .zero)
        var b = a
        b.inputOverride = ["x": 1]
        #expect(a == b)
        b.previousResult = "changed"
        #expect(a != b)
    }
}
