import CoreGraphics
import Foundation

enum WorkflowNodeStatus: String {
    case idle = "Idle"
    case running = "Running"
    case succeeded = "Succeeded"
    case failed = "Failed"

    var symbolName: String {
        switch self {
        case .idle: "circle"
        case .running: "play.circle.fill"
        case .succeeded: "checkmark.circle.fill"
        case .failed: "xmark.octagon.fill"
        }
    }
}

enum WorkflowNodeKind: String {
    case context = "Context"
    case script = "Rust Script"
    case agent = "Pi Agent"
    case digest = "Digest"
}

struct WorkflowNodeModel: Identifiable, Equatable {
    let id: String
    let title: String
    let subtitle: String
    let kind: WorkflowNodeKind
    let position: CGPoint
    var status: WorkflowNodeStatus = .idle
    var result: String?
}

struct WorkflowEdgeModel: Identifiable, Equatable {
    let from: String
    let to: String

    var id: String {
        "\(from)->\(to)"
    }
}

struct WorkflowEventModel: Identifiable, Equatable {
    let id = UUID()
    let time: Date
    let nodeID: String?
    let title: String
    let detail: String
}

struct BusinessWorkflowExample {
    static let nodes: [WorkflowNodeModel] = [
        WorkflowNodeModel(
            id: "weekly_context",
            title: "Weekly Context",
            subtitle: "B2B SaaS, 2026-W19",
            kind: .context,
            position: CGPoint(x: 0.10, y: 0.45)
        ),
        WorkflowNodeModel(
            id: "sales_health",
            title: "Sales Health",
            subtitle: "Rust script metrics",
            kind: .script,
            position: CGPoint(x: 0.34, y: 0.25)
        ),
        WorkflowNodeModel(
            id: "support_health",
            title: "Support Health",
            subtitle: "Rust script queue scan",
            kind: .script,
            position: CGPoint(x: 0.34, y: 0.66)
        ),
        WorkflowNodeModel(
            id: "pi_recommendation",
            title: "Pi Recommendation",
            subtitle: "Risk and next action",
            kind: .agent,
            position: CGPoint(x: 0.62, y: 0.45)
        ),
        WorkflowNodeModel(
            id: "operator_digest",
            title: "Operator Digest",
            subtitle: "Final run summary",
            kind: .digest,
            position: CGPoint(x: 0.88, y: 0.45)
        )
    ]

    static let edges: [WorkflowEdgeModel] = [
        WorkflowEdgeModel(from: "weekly_context", to: "sales_health"),
        WorkflowEdgeModel(from: "weekly_context", to: "support_health"),
        WorkflowEdgeModel(from: "sales_health", to: "pi_recommendation"),
        WorkflowEdgeModel(from: "support_health", to: "pi_recommendation"),
        WorkflowEdgeModel(from: "weekly_context", to: "operator_digest"),
        WorkflowEdgeModel(from: "sales_health", to: "operator_digest"),
        WorkflowEdgeModel(from: "support_health", to: "operator_digest"),
        WorkflowEdgeModel(from: "pi_recommendation", to: "operator_digest")
    ]

    static let stages: [[String]] = [
        ["weekly_context"],
        ["sales_health", "support_health"],
        ["pi_recommendation"],
        ["operator_digest"]
    ]
}
