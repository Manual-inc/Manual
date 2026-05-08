import Foundation

@MainActor
final class WorkflowRunStore: ObservableObject {
    @Published private(set) var nodes = BusinessWorkflowExample.nodes
    @Published private(set) var edges = BusinessWorkflowExample.edges
    @Published private(set) var events: [WorkflowEventModel] = []
    @Published private(set) var isRunning = false
    @Published private(set) var selectedNodeID: String? = BusinessWorkflowExample.nodes.first?.id
    @Published private(set) var runID: String?

    private let client: AppServerClient

    init(client: AppServerClient = AppServerClient()) {
        self.client = client
    }

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
        runID = nil
        events.removeAll()
        resetNodes()

        Task { [weak self] in
            guard let self else { return }
            await self.startViaJSONRPC()
        }
    }

    private func startViaJSONRPC() async {
        do {
            try await client.createWorkflow(BusinessWorkflowExample.jsonDefinition)
            let runID = try await client.startWorkflow(id: BusinessWorkflowExample.workflowID)
            self.runID = runID
            try await streamEvents(runID: runID)
        } catch {
            appendEvent(nodeID: nil, title: "Workflow failed", detail: error.localizedDescription)
            isRunning = false
        }
    }

    private func streamEvents(runID: String) async throws {
        var cursor = 0
        var completed = false

        while !completed {
            let page = try await client.events(runID: runID, cursor: cursor)
            cursor = page.nextCursor
            completed = page.completed

            for event in page.events {
                applyServerEvent(event)
                try? await Task.sleep(for: .milliseconds(320))
            }

            if !completed {
                try? await Task.sleep(for: .milliseconds(350))
            }
        }

        isRunning = false
    }

    private func applyServerEvent(_ event: [String: Any]) {
        let type = event["type"] as? String ?? "event"
        let nodeID = event["node_id"] as? String

        switch type {
        case "workflow_started":
            appendEvent(nodeID: nil, title: "Workflow started", detail: BusinessWorkflowExample.workflowID)
        case "workflow_completed":
            appendEvent(nodeID: nil, title: "Workflow completed", detail: "Operator digest is ready")
        case "node_started":
            if let nodeID {
                mark(nodeID, as: .running)
                appendEvent(nodeID: nodeID, title: "Node started", detail: nodeTitle(nodeID))
            }
        case "node_completed":
            if let nodeID {
                let result = displayString(for: event["result"])
                complete(nodeID, result: result)
                appendEvent(nodeID: nodeID, title: "Node completed", detail: result)
            }
        default:
            appendEvent(nodeID: nodeID, title: type, detail: displayString(for: event))
        }
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

    private func displayString(for value: Any?) -> String {
        switch value {
        case let value as String:
            value
        case let value as NSNumber:
            value.stringValue
        case let value as [String: Any]:
            value
                .keys
                .sorted()
                .compactMap { key in
                    guard let nested = value[key] else { return nil }
                    return "\(key)=\(displayString(for: nested))"
                }
                .joined(separator: ", ")
        case let value as [Any]:
            value.map { displayString(for: $0) }.joined(separator: ", ")
        case .none:
            "null"
        default:
            String(describing: value!)
        }
    }
}
