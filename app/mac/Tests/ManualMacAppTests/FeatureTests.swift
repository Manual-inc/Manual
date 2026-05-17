import Testing
import ManualCucumber

private func recordFailures(_ failures: [FeatureFailure]) {
    for failure in failures {
        Issue.record(Comment(rawValue: "[\(failure.scenario)] line \(failure.line): \(failure.message)"))
    }
}

private func runFeatureOffMainThread(relativePath: String) async -> [FeatureFailure] {
    await Task.detached {
        CucumberRunner.runFeature(relativePath: relativePath)
    }.value
}

@Suite("매뉴얼 관리")
struct ManualManagementFeatureTests {
    @Test func runFeature() async {
        recordFailures(await runFeatureOffMainThread(relativePath: "매뉴얼-관리.feature"))
    }
}

@Suite("샌드박스 기능")
struct SandboxFeatureTests {
    @Test func runFeature() async {
        recordFailures(await runFeatureOffMainThread(relativePath: "샌드박스-기능.feature"))
    }
}

@Suite("부분 실행과 재시작")
struct PartialRunFeatureTests {
    @Test func runFeature() async {
        recordFailures(await runFeatureOffMainThread(relativePath: "부분-실행과-재시작.feature"))
    }
}

@Suite("매뉴얼 자기진화 기능")
struct SelfEvolutionFeatureTests {
    @Test func runFeature() async {
        recordFailures(await runFeatureOffMainThread(relativePath: "매뉴얼-자기진화-기능.feature"))
    }
}

@Suite("노드 스토리북")
struct NodeStorybookFeatureTests {
    @Test func runFeature() async {
        recordFailures(await runFeatureOffMainThread(relativePath: "노드-스토리북.feature"))
    }
}

@Suite("에이전트 스킬 지정")
struct AgentSkillFeatureTests {
    @Test func runFeature() async {
        recordFailures(await runFeatureOffMainThread(relativePath: "에이전트-스킬-지정.feature"))
    }
}

@Suite("매뉴얼 최적화 기능")
struct OptimizationFeatureTests {
    @Test func runFeature() async {
        recordFailures(await runFeatureOffMainThread(relativePath: "매뉴얼-최적화-기능.feature"))
    }
}

@Suite("mac UI app-server 실행")
struct MacUIAppServerFeatureTests {
    @Test func runFeature() async {
        recordFailures(await runFeatureOffMainThread(relativePath: "mac-ui-app-server.feature"))
    }
}
