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
    case claude = "Claude Review"
    case codex = "Codex Review"
    case digest = "Digest"

    static func serverKind(_ value: String) -> WorkflowNodeKind {
        switch value {
        case "claude":
            .claude
        case "constant":
            .context
        case "codex":
            .codex
        case "pi":
            .agent
        case "template":
            .digest
        default:
            .script
        }
    }
}

struct WorkflowNodeModel: Identifiable, Equatable {
    let id: String
    let title: String
    let subtitle: String
    let kind: WorkflowNodeKind
    let position: CGPoint
    var sandboxPolicyID: String? = nil
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

struct WorkflowDisplayModel {
    let workflowID: String
    let nodes: [WorkflowNodeModel]
    let edges: [WorkflowEdgeModel]
}

enum WorkflowDisplayBuilder {
    static func build(from workflow: [String: Any]) -> WorkflowDisplayModel {
        let workflowID = workflow["id"] as? String ?? "untitled-workflow"
        let nodeObjects = workflow["nodes"] as? [[String: Any]] ?? []
        let dependencyObjects = workflow["dependencies"] as? [[String: Any]] ?? []
        let nodeIDs = nodeObjects.compactMap { $0["id"] as? String }
        let stages = layoutStages(nodeIDs: nodeIDs, dependencies: dependencyObjects)
        let positions = positionsByNodeID(stages: stages)

        let nodes = nodeObjects.enumerated().compactMap { offset, object -> WorkflowNodeModel? in
            guard let id = object["id"] as? String else { return nil }
            let serverKind = object["kind"] as? String ?? "template"
            return WorkflowNodeModel(
                id: id,
                title: title(for: id),
                subtitle: subtitle(for: object),
                kind: WorkflowNodeKind.serverKind(serverKind),
                position: positions[id] ?? fallbackPosition(offset: offset, count: max(nodeObjects.count, 1)),
                sandboxPolicyID: sandboxPolicyID(for: object)
            )
        }

        let edges = dependencyObjects.compactMap { dependency -> WorkflowEdgeModel? in
            guard
                let from = dependency["depends_on"] as? String,
                let to = dependency["node"] as? String
            else { return nil }

            return WorkflowEdgeModel(from: from, to: to)
        }

        return WorkflowDisplayModel(workflowID: workflowID, nodes: nodes, edges: edges)
    }

    private static func layoutStages(nodeIDs: [String], dependencies: [[String: Any]]) -> [[String]] {
        var remaining = Set(nodeIDs)
        var completed = Set<String>()
        var stages: [[String]] = []

        while !remaining.isEmpty {
            let ready = remaining
                .filter { nodeID in
                    dependencies
                        .filter { $0["node"] as? String == nodeID }
                        .allSatisfy { dependency in
                            guard let upstream = dependency["depends_on"] as? String else { return true }
                            return completed.contains(upstream)
                        }
                }
                .sorted()

            let stage = ready.isEmpty ? remaining.sorted() : ready
            stages.append(stage)
            completed.formUnion(stage)
            remaining.subtract(stage)
        }

        return stages
    }

    private static func positionsByNodeID(stages: [[String]]) -> [String: CGPoint] {
        var positions: [String: CGPoint] = [:]
        let stageCount = max(stages.count, 1)

        for (stageIndex, stage) in stages.enumerated() {
            let x = stageCount == 1
                ? 0.50
                : 0.12 + (0.76 * CGFloat(stageIndex) / CGFloat(stageCount - 1))
            let rowCount = max(stage.count, 1)

            for (rowIndex, nodeID) in stage.enumerated() {
                let y = rowCount == 1
                    ? 0.50
                    : 0.24 + (0.52 * CGFloat(rowIndex) / CGFloat(rowCount - 1))
                positions[nodeID] = CGPoint(x: x, y: y)
            }
        }

        return positions
    }

    private static func fallbackPosition(offset: Int, count: Int) -> CGPoint {
        CGPoint(
            x: 0.15 + (0.70 * CGFloat(offset) / CGFloat(max(count - 1, 1))),
            y: 0.50
        )
    }

    private static func title(for id: String) -> String {
        id
            .split(separator: "_")
            .map { word in
                word.prefix(1).uppercased() + word.dropFirst()
            }
            .joined(separator: " ")
    }

    private static func subtitle(for object: [String: Any]) -> String {
        let kind = object["kind"] as? String ?? "template"

        switch kind {
        case "constant":
            return "Constant payload"
        case "delay":
            return "\(object["duration_ms"] as? Int ?? 0) ms delay"
        case "fail":
            return object["error"] as? String ?? "Failure node"
        case "pi":
            return object["model"] as? String ?? "Pi agent"
        case "claude":
            return object["model"] as? String ?? "Claude reviewer"
        case "codex":
            return object["model"] as? String ?? "Codex reviewer"
        case "template":
            return object["template"] as? String ?? "Template"
        default:
            return kind
        }
    }

    private static func sandboxPolicyID(for object: [String: Any]) -> String? {
        // See docs/wiki/architecture/agent-sandboxing.md: node-level sandbox IDs are the bridge between workflow JSON and OS policy application.
        guard let policy = object["sandbox_policy"] as? [String: Any] else { return nil }
        return policy["sandbox_id"] as? String ?? policy["id"] as? String
    }
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
            position: CGPoint(x: 0.58, y: 0.30)
        ),
        WorkflowNodeModel(
            id: "chaos_script",
            title: "Chaos Script",
            subtitle: "Intentional script failure",
            kind: .script,
            position: CGPoint(x: 0.58, y: 0.66)
        ),
        WorkflowNodeModel(
            id: "operator_digest",
            title: "Operator Digest",
            subtitle: "Final run summary",
            kind: .digest,
            position: CGPoint(x: 0.88, y: 0.45)
        ),
    ]

    static let edges: [WorkflowEdgeModel] = [
        WorkflowEdgeModel(from: "weekly_context", to: "sales_health"),
        WorkflowEdgeModel(from: "weekly_context", to: "support_health"),
        WorkflowEdgeModel(from: "sales_health", to: "pi_recommendation"),
        WorkflowEdgeModel(from: "support_health", to: "pi_recommendation"),
        WorkflowEdgeModel(from: "pi_recommendation", to: "chaos_script"),
        WorkflowEdgeModel(from: "weekly_context", to: "operator_digest"),
        WorkflowEdgeModel(from: "sales_health", to: "operator_digest"),
        WorkflowEdgeModel(from: "support_health", to: "operator_digest"),
        WorkflowEdgeModel(from: "pi_recommendation", to: "operator_digest"),
        WorkflowEdgeModel(from: "chaos_script", to: "operator_digest"),
    ]

    static let stages: [[String]] = [
        ["weekly_context"],
        ["sales_health", "support_health"],
        ["pi_recommendation"],
        ["chaos_script"],
        ["operator_digest"],
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
                        "goal": "Decide one light intervention for next week",
                    ],
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
                        "signal": "lead quality is acceptable but demo booking needs attention",
                    ],
                ],
                [
                    "id": "support_health",
                    "kind": "constant",
                    "value": [
                        "open_tickets": 37,
                        "stale_tickets": 9,
                        "stale_rate": 24.3,
                        "signal": "stale tickets are the clearest retention risk",
                    ],
                ],
                [
                    "id": "pi_recommendation",
                    "kind": "pi",
                    "prompt":
                        "You are reviewing a tiny weekly business health workflow. Based only on the input, return exactly two short bullet points: one risk and one next action.",
                ],
                [
                    "id": "chaos_script",
                    "kind": "fail",
                    "error": "script panic: simulated Rust script failure after recommendation",
                ],
                [
                    "id": "operator_digest",
                    "kind": "template",
                    "template":
                        "Weekly digest for {{weekly_context.business}}: {{sales_health.qualified}} qualified leads, {{support_health.open_tickets}} open tickets. Recommendation: {{pi_recommendation}}",
                ],
            ],
            "dependencies": [
                ["node": "sales_health", "depends_on": "weekly_context"],
                ["node": "support_health", "depends_on": "weekly_context"],
                ["node": "pi_recommendation", "depends_on": "sales_health"],
                ["node": "pi_recommendation", "depends_on": "support_health"],
                ["node": "chaos_script", "depends_on": "pi_recommendation"],
                ["node": "operator_digest", "depends_on": "weekly_context"],
                ["node": "operator_digest", "depends_on": "sales_health"],
                ["node": "operator_digest", "depends_on": "support_health"],
                ["node": "operator_digest", "depends_on": "pi_recommendation"],
                ["node": "operator_digest", "depends_on": "chaos_script"],
            ],
        ]
    }
}
