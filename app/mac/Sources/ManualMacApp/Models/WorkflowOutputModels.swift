import Foundation

// See docs/wiki/features/workflow-starters.md: native clients should surface
// the generated review result directly, not only raw event dictionaries.
struct StarterOutcomeSummary: Equatable, Sendable {
    let workflowID: String
    let runID: String?
    let label: String
    let text: String

    var rerunCommand: String {
        "manual workflow run \(workflowID) --human"
    }
}

func workflowOutputText(from value: Any?) -> String {
    switch value {
    case let value as String:
        return value
    case let value as NSNumber:
        return value.stringValue
    case let value as [String: Any]:
        if let stdout = value["stdout"] as? String, !stdout.isEmpty {
            return stdout
        }
        return value
            .keys
            .sorted()
            .compactMap { key in
                guard let nested = value[key] else { return nil }
                return "\(key)=\(workflowOutputText(from: nested))"
            }
            .joined(separator: ", ")
    case let value as [Any]:
        return value.map { workflowOutputText(from: $0) }.joined(separator: ", ")
    case .none:
        return "null"
    default:
        return String(describing: value!)
    }
}

func starterOutcomeSummary(
    workflowID: String?,
    runID: String?,
    nodes: [WorkflowNodeModel]
) -> StarterOutcomeSummary? {
    guard let workflowID, workflowID.hasPrefix("starter-") else { return nil }
    let preferredNodeIDs = ["review", "summary", "test_plan"]
    let node = preferredNodeIDs
        .compactMap { preferredID in
            nodes.first { $0.id == preferredID && $0.result != nil }
        }
        .first

    guard let node, let text = node.result, !text.isEmpty else { return nil }
    return StarterOutcomeSummary(
        workflowID: workflowID,
        runID: runID,
        label: "\(node.title) Output",
        text: text
    )
}

func starterOutcomeSummary(from entry: WorkflowStarterRecentEntry) -> StarterOutcomeSummary? {
    guard let outcomeText = entry.outcomeText, !outcomeText.isEmpty else { return nil }
    return StarterOutcomeSummary(
        workflowID: entry.workflowID,
        runID: nil,
        label: entry.outcomeLabel ?? "Starter Output",
        text: outcomeText
    )
}

func starterOutcomeShareText(_ summary: StarterOutcomeSummary) -> String {
    var lines = [
        "Starter Outcome",
        "Workflow ID: \(summary.workflowID)",
    ]
    if let runID = summary.runID {
        lines.append("Run ID: \(runID)")
    }
    lines.append("Reusable command: \(summary.rerunCommand)")
    lines.append(summary.label)
    lines.append(summary.text)
    return lines.joined(separator: "\n")
}

func starterOutcomeShareText(from entry: WorkflowStarterRecentEntry) -> String? {
    // See docs/wiki/features/workflow-starters.md: recent starter history
    // should let users reuse the last outcome without rerunning first.
    guard let summary = starterOutcomeSummary(from: entry) else { return nil }
    return starterOutcomeShareText(summary)
}

func starterOutcomePreviewText(_ outcomeText: String) -> String {
    let firstLine = outcomeText
        .split(separator: "\n", omittingEmptySubsequences: true)
        .first
        .map(String.init)?
        .trimmingCharacters(in: .whitespacesAndNewlines)
        ?? outcomeText.trimmingCharacters(in: .whitespacesAndNewlines)
    let preview = firstLine.isEmpty ? outcomeText.trimmingCharacters(in: .whitespacesAndNewlines) : firstLine
    if preview.count > 120 {
        return String(preview.prefix(117)) + "..."
    }
    return preview
}
