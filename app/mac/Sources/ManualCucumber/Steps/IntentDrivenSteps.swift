import Foundation

// See docs/wiki/systems/기능-계약-테스트.md: every cucumber step is routed through an app intent before reaching app-server.
enum IntentDrivenSteps {
    static func register(in registry: StepRegistry) {
        for stepText in allStepTexts() {
            let pattern = NSRegularExpression.escapedPattern(for: stepText)
            registry.step(pattern) { world, captures, text, file, line in
                try CucumberFeatureIntent.perform(world: world, step: text, captures: captures, file: file, line: line)
                try CucumberFeatureAssertions.assert(world: world, step: text, file: file, line: line)
            }
        }
    }

    private static func allStepTexts() -> [String] {
        let root = repositoryRoot()
        let directories = [
            root.appendingPathComponent("docs/usecase", isDirectory: true),
            root.appendingPathComponent("app/mac/Features", isDirectory: true),
        ]

        var texts = Set<String>()
        for directory in directories {
            guard let files = try? FileManager.default.contentsOfDirectory(at: directory, includingPropertiesForKeys: nil) else {
                continue
            }

            for file in files where file.pathExtension == "feature" {
                guard let feature = try? GherkinParser.parse(path: file.path) else { continue }
                for step in feature.background + feature.scenarios.flatMap(\.steps) {
                    texts.insert(step.text)
                }
            }
        }

        return texts.sorted()
    }

    private static func repositoryRoot() -> URL {
        URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent() // Steps
            .deletingLastPathComponent() // ManualCucumber
            .deletingLastPathComponent() // Sources
            .deletingLastPathComponent() // mac
            .deletingLastPathComponent() // app
            .deletingLastPathComponent() // Manual
    }
}
