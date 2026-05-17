import Foundation

struct GherkinStep {
    enum Kind { case given, when, then }
    let kind: Kind
    let text: String
    let line: Int
}

struct GherkinScenario {
    let name: String
    let line: Int
    let steps: [GherkinStep]
}

struct GherkinFeature {
    let name: String
    let path: String
    let background: [GherkinStep]
    let scenarios: [GherkinScenario]
}

enum GherkinParseError: Error, CustomStringConvertible {
    case missingFeature(String)
    case stepWithoutScenario(line: Int)

    var description: String {
        switch self {
        case let .missingFeature(path): "기능 헤더가 없습니다: \(path)"
        case let .stepWithoutScenario(line): "시나리오 없이 단계가 존재합니다 (line \(line))"
        }
    }
}

enum GherkinParser {
    private static let featureKeyword = "기능:"
    private static let backgroundKeyword = "배경:"
    private static let scenarioKeywords = ["시나리오:", "시나리오 개요:"]
    private static let givenKeywords = ["조건 ", "전제 "]
    private static let whenKeywords = ["만일 ", "만약 "]
    private static let thenKeywords = ["그러면 "]
    private static let andKeywords = ["그리고 ", "단 "]

    static func parse(path: String) throws -> GherkinFeature {
        let url = URL(fileURLWithPath: path)
        let source = try String(contentsOf: url, encoding: .utf8)
        return try parse(source: source, path: path)
    }

    static func parse(source: String, path: String) throws -> GherkinFeature {
        var featureName: String?
        var background: [GherkinStep] = []
        var scenarios: [GherkinScenario] = []

        enum Section { case none, background, scenario }
        var section: Section = .none
        var currentScenarioName = ""
        var currentScenarioLine = 0
        var currentScenarioSteps: [GherkinStep] = []
        var lastKind: GherkinStep.Kind = .given

        func flushScenario() {
            guard section == .scenario else { return }
            scenarios.append(
                GherkinScenario(
                    name: currentScenarioName,
                    line: currentScenarioLine,
                    steps: currentScenarioSteps
                )
            )
            currentScenarioSteps = []
        }

        let lines = source.components(separatedBy: "\n")
        for (index, raw) in lines.enumerated() {
            let lineNumber = index + 1
            let trimmed = raw.trimmingCharacters(in: .whitespaces)
            if trimmed.isEmpty { continue }
            if trimmed.hasPrefix("#") { continue }

            if trimmed.hasPrefix(featureKeyword) {
                featureName = String(trimmed.dropFirst(featureKeyword.count))
                    .trimmingCharacters(in: .whitespaces)
                continue
            }

            if trimmed.hasPrefix(backgroundKeyword) {
                flushScenario()
                section = .background
                continue
            }

            if let scenarioPrefix = scenarioKeywords.first(where: { trimmed.hasPrefix($0) }) {
                flushScenario()
                section = .scenario
                currentScenarioName = String(trimmed.dropFirst(scenarioPrefix.count))
                    .trimmingCharacters(in: .whitespaces)
                currentScenarioLine = lineNumber
                continue
            }

            if let (kind, text) = matchStep(trimmed: trimmed, lastKind: lastKind) {
                lastKind = kind
                let step = GherkinStep(kind: kind, text: text, line: lineNumber)
                switch section {
                case .background:
                    background.append(step)
                case .scenario:
                    currentScenarioSteps.append(step)
                case .none:
                    throw GherkinParseError.stepWithoutScenario(line: lineNumber)
                }
                continue
            }

            if section == .scenario, trimmed.hasPrefix("- ") || trimmed.hasPrefix("\"\"\"") {
                continue
            }

            if !trimmed.isEmpty,
               !trimmed.hasPrefix("|"),
               !trimmed.hasPrefix(":") {
                continue
            }
        }

        flushScenario()

        guard let featureName else {
            throw GherkinParseError.missingFeature(path)
        }

        return GherkinFeature(name: featureName, path: path, background: background, scenarios: scenarios)
    }

    private static func matchStep(trimmed: String, lastKind: GherkinStep.Kind) -> (GherkinStep.Kind, String)? {
        for keyword in givenKeywords where trimmed.hasPrefix(keyword) {
            return (.given, String(trimmed.dropFirst(keyword.count)).trimmingCharacters(in: .whitespaces))
        }
        for keyword in whenKeywords where trimmed.hasPrefix(keyword) {
            return (.when, String(trimmed.dropFirst(keyword.count)).trimmingCharacters(in: .whitespaces))
        }
        for keyword in thenKeywords where trimmed.hasPrefix(keyword) {
            return (.then, String(trimmed.dropFirst(keyword.count)).trimmingCharacters(in: .whitespaces))
        }
        for keyword in andKeywords where trimmed.hasPrefix(keyword) {
            return (lastKind, String(trimmed.dropFirst(keyword.count)).trimmingCharacters(in: .whitespaces))
        }
        return nil
    }
}
