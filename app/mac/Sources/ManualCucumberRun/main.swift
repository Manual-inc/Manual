import Foundation
import ManualCucumber

// See docs/wiki/systems/기능-계약-테스트.md: run cucumber off the main thread so MainActor UI intents can execute.
Task.detached {
    let features: [(String, String)] = [
        ("매뉴얼 관리", "매뉴얼-관리.feature"),
        ("샌드박스 기능", "샌드박스-기능.feature"),
        ("부분 실행과 재시작", "부분-실행과-재시작.feature"),
        ("매뉴얼 자기진화 기능", "매뉴얼-자기진화-기능.feature"),
        ("노드 스토리북", "노드-스토리북.feature"),
        ("에이전트 스킬 지정", "에이전트-스킬-지정.feature"),
        ("매뉴얼 최적화 기능", "매뉴얼-최적화-기능.feature"),
        ("mac UI app-server 실행", "mac-ui-app-server.feature"),
    ]

    var totalFailures = 0
    for (label, path) in features {
        print("\n▶ \(label) (\(path))")
        let failures = CucumberRunner.runFeature(relativePath: path)
        if failures.isEmpty {
            print("  ✓ all scenarios passed")
        } else {
            totalFailures += failures.count
            for failure in failures {
                print("  ✗ [\(failure.scenario)] line \(failure.line): \(failure.message)")
            }
        }
    }

    print("\n==================================")
    if totalFailures == 0 {
        print("✅ 모든 feature 통과")
        exit(0)
    } else {
        print("❌ \(totalFailures)개 실패")
        exit(1)
    }
}

dispatchMain()
