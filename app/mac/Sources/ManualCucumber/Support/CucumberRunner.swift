import Foundation

public struct FeatureFailure: Sendable {
    public let scenario: String
    public let line: Int
    public let message: String
}

public enum CucumberRunner {
    public static func runFeature(relativePath: String) -> [FeatureFailure] {
        StepRegistry.shared.ensureRegistered()

        let featureURL = featureURL(relativePath: relativePath)
        let feature: GherkinFeature
        do {
            feature = try GherkinParser.parse(path: featureURL.path)
        } catch {
            return [FeatureFailure(scenario: "<feature>", line: 0, message: "Feature 파일 파싱 실패 \(featureURL.path): \(error)")]
        }

        var failures: [FeatureFailure] = []
        for scenario in feature.scenarios {
            let world = ManualWorld()
            failures.append(contentsOf: runSteps(feature.background, scenarioName: scenario.name, world: world))
            failures.append(contentsOf: runSteps(scenario.steps, scenarioName: scenario.name, world: world))
        }
        return failures
    }

    private static func runSteps(
        _ steps: [GherkinStep],
        scenarioName: String,
        world: ManualWorld
    ) -> [FeatureFailure] {
        var failures: [FeatureFailure] = []
        for step in steps {
            guard let (entry, captures) = StepRegistry.shared.match(step.text) else {
                failures.append(FeatureFailure(
                    scenario: scenarioName,
                    line: step.line,
                    message: "정의되지 않은 단계: \(step.text)"
                ))
                return failures
            }
            do {
                try entry.handler(world, captures, step.text, #filePath, UInt(step.line))
            } catch {
                failures.append(FeatureFailure(
                    scenario: scenarioName,
                    line: step.line,
                    message: "단계 실패 [\(entry.source)]: \(error)"
                ))
                return failures
            }
        }
        return failures
    }

    private static func featureURL(relativePath: String) -> URL {
        let here = URL(fileURLWithPath: #filePath)
        let repoRoot = here
            .deletingLastPathComponent() // Support
            .deletingLastPathComponent() // ManualCucumber
            .deletingLastPathComponent() // Sources
            .deletingLastPathComponent() // mac
            .deletingLastPathComponent() // app
            .deletingLastPathComponent() // Manual
        let sharedUsecaseURL = repoRoot
            .appendingPathComponent("docs", isDirectory: true)
            .appendingPathComponent("usecase", isDirectory: true)
            .appendingPathComponent(relativePath)
        if FileManager.default.fileExists(atPath: sharedUsecaseURL.path) {
            return sharedUsecaseURL
        }

        return repoRoot
            .appendingPathComponent("app/mac/Features", isDirectory: true)
            .appendingPathComponent(relativePath)
    }
}
