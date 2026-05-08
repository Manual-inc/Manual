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
    static let workflowID = "business-pipeline-health"

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

    static var jsonDefinition: [String: Any] {
        [
            "id": workflowID,
            "nodes": [
                [
                    "id": "weekly_context",
                    "kind": "constant",
                    "value": [
                        "week": "2026-W19",
                        "business": "B2B SaaS",
                        "goal": "Decide one light intervention for next week"
                    ]
                ],
                [
                    "id": "sales_health",
                    "kind": "constant",
                    "value": [
                        "leads": 128,
                        "qualified": 42,
                        "demos": 18,
                        "conversion_rate": 32.8,
                        "demo_rate": 42.9,
                        "signal": "lead quality is acceptable but demo booking needs attention"
                    ]
                ],
                [
                    "id": "support_health",
                    "kind": "constant",
                    "value": [
                        "open_tickets": 37,
                        "stale_tickets": 9,
                        "stale_rate": 24.3,
                        "signal": "stale tickets are the clearest retention risk"
                    ]
                ],
                [
                    "id": "pi_recommendation",
                    "kind": "template",
                    "template": "Risk: {{support_health.signal}}. Next: reduce {{support_health.stale_tickets}} stale tickets before tuning demo booking."
                ],
                [
                    "id": "operator_digest",
                    "kind": "template",
                    "template": "Weekly digest for {{weekly_context.business}}: {{sales_health.qualified}} qualified leads, {{support_health.open_tickets}} open tickets. Recommendation: {{pi_recommendation}}"
                ]
            ],
            "dependencies": [
                ["node": "sales_health", "depends_on": "weekly_context"],
                ["node": "support_health", "depends_on": "weekly_context"],
                ["node": "pi_recommendation", "depends_on": "sales_health"],
                ["node": "pi_recommendation", "depends_on": "support_health"],
                ["node": "operator_digest", "depends_on": "weekly_context"],
                ["node": "operator_digest", "depends_on": "sales_health"],
                ["node": "operator_digest", "depends_on": "support_health"],
                ["node": "operator_digest", "depends_on": "pi_recommendation"]
            ]
        ]
    }
}
