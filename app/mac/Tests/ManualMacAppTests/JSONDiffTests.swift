import Testing
@testable import ManualMacApp

@Suite("JSONDiff")
struct JSONDiffTests {
    @Test("동일한 텍스트는 모두 .same")
    func sameText() {
        let lines = JSONDiff.diff(old: "hello\nworld", new: "hello\nworld")
        #expect(lines.allSatisfy { $0.op == .same })
        #expect(lines.count == 2)
    }

    @Test("추가된 라인은 .added")
    func addedLine() {
        let lines = JSONDiff.diff(old: "a", new: "a\nb")
        let ops = lines.map(\.op)
        #expect(ops.contains(.added))
        #expect(ops.filter { $0 == .added }.count == 1)
    }

    @Test("삭제된 라인은 .removed")
    func removedLine() {
        let lines = JSONDiff.diff(old: "a\nb", new: "a")
        let ops = lines.map(\.op)
        #expect(ops.contains(.removed))
        #expect(ops.filter { $0 == .removed }.count == 1)
    }

    @Test("빈 입력 처리")
    func emptyInputs() {
        #expect(JSONDiff.diff(old: nil, new: nil).isEmpty)
        #expect(JSONDiff.diff(old: "", new: "").isEmpty)
        #expect(JSONDiff.diff(old: nil, new: "x").count == 1)
    }

    @Test("prettyJSON은 딕셔너리를 직렬화")
    func prettyJSON() {
        let result = JSONDiff.prettyJSON(["key": "value"] as [String: Any])
        #expect(result != nil)
        #expect(result!.contains("key"))
    }
}
