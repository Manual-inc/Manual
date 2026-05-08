import Foundation

@MainActor
final class WorkflowRunStore: ObservableObject {
    @Published private(set) var nodes = BusinessWorkflowExample.nodes
    @Published private(set) var edges = BusinessWorkflowExample.edges
    @Published private(set) var events: [WorkflowEventModel] = []
    @Published private(set) var isRunning = false
    @Published private(set) var selectedNodeID: String? = BusinessWorkflowExample.nodes.first?.id
    @Published private(set) var runID: String?

    var selectedNode: WorkflowNodeModel? {
        guard let selectedNodeID else { return nil }
        return nodes.first { $0.id == selectedNodeID }
    }

    var completedCount: Int {
        nodes.filter { $0.status == .succeeded }.count
    }

    var progressText: String {
        "\(completedCount) of \(nodes.count) nodes"
    }

    func selectNode(_ id: String) {
        selectedNodeID = id
    }

    func start() {
        guard !isRunning else { return }

        isRunning = true
        runID = "run-\(Int(Date().timeIntervalSince1970))"
        events.removeAll()
        resetNodes()
        appendEvent(nodeID: nil, title: "Workflow started", detail: "Business pipeline health check")

        Task { [weak self] in
            guard let self else { return }
            await self.executeStages()
        }
    }

    private func executeStages() async {
        for stage in BusinessWorkflowExample.stages {
            for nodeID in stage {
                mark(nodeID, as: .running)
                appendEvent(nodeID: nodeID, title: "Node started", detail: nodeTitle(nodeID))
            }

            try? await Task.sleep(for: .milliseconds(stage.count == 1 ? 650 : 900))

            for nodeID in stage {
                let result = resultForNode(nodeID)
                complete(nodeID, result: result)
                appendEvent(nodeID: nodeID, title: "Node completed", detail: result)
            }
        }

        appendEvent(nodeID: nil, title: "Workflow completed", detail: "Operator digest is ready")
        isRunning = false
    }

    private func resetNodes() {
        nodes = BusinessWorkflowExample.nodes
        selectedNodeID = nodes.first?.id
    }

    private func mark(_ nodeID: String, as status: WorkflowNodeStatus) {
        guard let index = nodes.firstIndex(where: { $0.id == nodeID }) else { return }
        nodes[index].status = status
    }

    private func complete(_ nodeID: String, result: String) {
        guard let index = nodes.firstIndex(where: { $0.id == nodeID }) else { return }
        nodes[index].status = .succeeded
        nodes[index].result = result
    }

    private func appendEvent(nodeID: String?, title: String, detail: String) {
        events.append(
            WorkflowEventModel(
                time: Date(),
                nodeID: nodeID,
                title: title,
                detail: detail
            )
        )
    }

    private func nodeTitle(_ nodeID: String) -> String {
        nodes.first { $0.id == nodeID }?.title ?? nodeID
    }

    private func resultForNode(_ nodeID: String) -> String {
        switch nodeID {
        case "weekly_context":
            "week=2026-W19, business=B2B SaaS"
        case "sales_health":
            "128 leads, 42 qualified, demo rate 42.9%"
        case "support_health":
            "37 open tickets, 9 stale, stale rate 24.3%"
        case "pi_recommendation":
            "Risk: stale support tickets. Next: clear aged queue before improving demos."
        case "operator_digest":
            "Weekly digest assembled from context, metrics, support scan, and Pi recommendation."
        default:
            "Completed"
        }
    }
}
