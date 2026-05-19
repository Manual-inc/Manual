import Foundation

// See docs/wiki/features/workflow-starters.md: native clients should surface
// the generated review result directly, not only raw event dictionaries.
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
