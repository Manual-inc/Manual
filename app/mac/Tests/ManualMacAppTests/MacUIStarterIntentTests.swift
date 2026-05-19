import Testing
@testable import ManualCucumber

// See docs/wiki/systems/기능-계약-테스트.md and
// docs/wiki/features/workflow-starters.md: mac starter intents should prove
// recommended and recent-rerun paths with real app-server evidence.
@Suite("mac UI starter intents")
struct MacUIStarterIntentTests {
    @Test func recommendedStarterStepCreatesStarterWorkflowViaMacIntent() throws {
        let world = ManualWorld()

        try CucumberFeatureIntent.perform(
            world: world,
            step: "mac UI intent가 테스트 app-server discovery를 사용한다",
            captures: [],
            file: #filePath,
            line: #line
        )
        try CucumberFeatureIntent.perform(
            world: world,
            step: "사용자가 UI에서 추천 starter 실행을 선택한다",
            captures: [],
            file: #filePath,
            line: #line
        )

        let workflowID = try #require(world.currentWorkflowID)
        #expect(workflowID.hasPrefix("starter-"))

        let response = try world.appServer.rpc(
            method: "workflow.get",
            params: ["workflow_id": workflowID]
        )
        let workflow = try #require((response["result"] as? [String: Any])?["workflow"] as? [String: Any])
        let nodes = try #require(workflow["nodes"] as? [[String: Any]])

        #expect(nodes.contains { ($0["id"] as? String) == "collect_diff" })
        #expect(nodes.contains { ($0["id"] as? String) == "summary" })
    }

    @Test func recentStarterRerunStepLoadsSharedHistoryAndStartsSamePreset() throws {
        let world = ManualWorld()

        try CucumberFeatureIntent.perform(
            world: world,
            step: "mac UI intent가 테스트 app-server discovery를 사용한다",
            captures: [],
            file: #filePath,
            line: #line
        )
        try CucumberFeatureIntent.perform(
            world: world,
            step: "사용자가 UI에서 code review starter 실행을 선택한다",
            captures: [],
            file: #filePath,
            line: #line
        )

        let starterListCallsBefore = world.appServer.evidence.filter { $0 == "starter.list" }.count

        try CucumberFeatureIntent.perform(
            world: world,
            step: "사용자가 UI에서 recent starter rerun을 선택한다",
            captures: [],
            file: #filePath,
            line: #line
        )

        let workflowID = try #require(world.currentWorkflowID)
        #expect(workflowID.hasPrefix("starter-"))
        #expect(world.appServer.evidence.filter { $0 == "starter.list" }.count == starterListCallsBefore + 1)

        let response = try world.appServer.rpc(
            method: "workflow.get",
            params: ["workflow_id": workflowID]
        )
        let workflow = try #require((response["result"] as? [String: Any])?["workflow"] as? [String: Any])
        let nodes = try #require(workflow["nodes"] as? [[String: Any]])

        #expect(nodes.contains { ($0["id"] as? String) == "collect_diff" })
        #expect(nodes.contains { ($0["id"] as? String) == "review" })
    }
}
