import Foundation

@MainActor
final class WorkflowRunStore: ObservableObject {
    @Published private(set) var workflows: [WorkflowSummary] = []
    @Published private(set) var nodes = BusinessWorkflowExample.nodes
    @Published private(set) var edges = BusinessWorkflowExample.edges
    @Published private(set) var events: [WorkflowEventModel] = []
    @Published private(set) var isRunning = false
    @Published private(set) var isLoading = false
    @Published private(set) var selectedNodeID: String? = BusinessWorkflowExample.nodes.first?.id
    @Published private(set) var selectedWorkflowID: String? = BusinessWorkflowExample.workflowID
    @Published private(set) var runID: String?
    @Published private(set) var statusMessage = "Ready"
    @Published private(set) var rawWorkflowJSON = "{}"

    private let client: AppServerClient
    private let executionIntent: WorkflowExecutionIntent
    private var currentWorkflow = BusinessWorkflowExample.jsonDefinition
    private var liveEventsTask: Task<Void, Never>?
    private var observedRunIDs = Set<String>()

    init(client: AppServerClient = AppServerClient()) {
        self.client = client
        self.executionIntent = WorkflowExecutionIntent(client: client)
        syncDisplay(with: currentWorkflow)
        rawWorkflowJSON = prettyJSONString(currentWorkflow)
    }

    var selectedNode: WorkflowNodeModel? {
        guard let selectedNodeID else { return nil }
        return nodes.first { $0.id == selectedNodeID }
    }

    var completedCount: Int {
        nodes.filter { $0.status == .succeeded }.count
    }

    var failedCount: Int {
        nodes.filter { $0.status == .failed }.count
    }

    var progressText: String {
        if failedCount > 0 {
            return "\(completedCount) succeeded, \(failedCount) failed"
        }

        return "\(completedCount) of \(nodes.count) nodes"
    }

    var hasSelectedWorkflow: Bool {
        selectedWorkflowID != nil
    }

    func selectNode(_ id: String) {
        selectedNodeID = id
    }

    func bootstrap() {
        startLiveUpdates()
        Task { [weak self] in
            guard let self else { return }
            await self.refreshWorkflows(createExampleIfMissing: true)
        }
    }

    func refresh() {
        Task { [weak self] in
            guard let self else { return }
            await self.refreshWorkflows(createExampleIfMissing: false)
        }
    }

    func selectWorkflow(_ workflowID: String) {
        guard selectedWorkflowID != workflowID else { return }
        selectedWorkflowID = workflowID
        Task { [weak self] in
            guard let self else { return }
            await self.loadWorkflow(id: workflowID)
        }
    }

    func saveSelectedWorkflow() {
        Task { [weak self] in
            guard let self else { return }
            await self.persistCurrentWorkflow()
        }
    }

    func deleteSelectedWorkflow() {
        guard let workflowID = selectedWorkflowID, !isRunning else { return }

        Task { [weak self] in
            guard let self else { return }
            await self.deleteWorkflow(id: workflowID)
        }
    }

    func start() {
        guard !isRunning, let workflowID = selectedWorkflowID else { return }

        isRunning = true
        runID = nil
        events.removeAll()
        resetNodes()
        statusMessage = "Starting \(workflowID)"

        Task { [weak self] in
            guard let self else { return }
            await self.startViaJSONRPC(workflowID: workflowID)
        }
    }

    private func refreshWorkflows(createExampleIfMissing: Bool) async {
        isLoading = true
        defer { isLoading = false }

        do {
            var summaries = try await client.workflows()

            if createExampleIfMissing && !summaries.contains(where: { $0.workflowID == BusinessWorkflowExample.workflowID }) {
                let result = try await client.createWorkflow(BusinessWorkflowExample.jsonDefinition)
                summaries.append(WorkflowSummary(workflowID: result.workflowID, nodeCount: result.nodeCount))
                summaries.sort { $0.workflowID < $1.workflowID }
            }

            workflows = summaries

            if let selectedWorkflowID, summaries.contains(where: { $0.workflowID == selectedWorkflowID }) {
                await loadWorkflow(id: selectedWorkflowID)
            } else if let first = summaries.first {
                selectedWorkflowID = first.workflowID
                await loadWorkflow(id: first.workflowID)
            } else {
                selectedWorkflowID = BusinessWorkflowExample.workflowID
                currentWorkflow = BusinessWorkflowExample.jsonDefinition
                syncDisplay(with: currentWorkflow)
                rawWorkflowJSON = prettyJSONString(currentWorkflow)
            }

            statusMessage = "\(summaries.count) workflow\(summaries.count == 1 ? "" : "s") loaded"
        } catch {
            statusMessage = error.localizedDescription
            appendEvent(nodeID: nil, title: "Refresh failed", detail: error.localizedDescription)
        }
    }

    private func startLiveUpdates() {
        liveEventsTask?.cancel()
        liveEventsTask = Task { [weak self] in
            guard let self else { return }

            do {
                let stream = try await client.liveEvents()
                for try await event in stream {
                    await self.applyLiveEvent(event)
                }
            } catch {
                await MainActor.run {
                    self.statusMessage = "Live updates disconnected: \(error.localizedDescription)"
                }
            }
        }
    }

    private func applyLiveEvent(_ event: AppServerLiveEvent) async {
        switch event.name {
        case "workflow_changed":
            await refreshWorkflows(createExampleIfMissing: false)
        case "run_changed":
            guard
                let runID = event.payload["run_id"] as? String,
                !observedRunIDs.contains(runID)
            else {
                return
            }

            observedRunIDs.insert(runID)
            self.runID = runID
            isRunning = true
            statusMessage = "Running \(runID)"
            Task { [weak self] in
                guard let self else { return }
                try? await self.streamEvents(runID: runID)
            }
        default:
            break
        }
    }

    private func loadWorkflow(id workflowID: String) async {
        isLoading = true
        defer { isLoading = false }

        do {
            let workflow = try await client.workflow(id: workflowID)
            currentWorkflow = workflow
            selectedWorkflowID = workflow["id"] as? String ?? workflowID
            syncDisplay(with: workflow)
            rawWorkflowJSON = prettyJSONString(workflow)
            statusMessage = "Loaded \(workflowID)"
        } catch {
            statusMessage = error.localizedDescription
            appendEvent(nodeID: nil, title: "Load failed", detail: error.localizedDescription)
        }
    }

    private func persistCurrentWorkflow() async {
        do {
            let workflowID = currentWorkflow["id"] as? String ?? BusinessWorkflowExample.workflowID
            let exists = workflows.contains { $0.workflowID == workflowID }
            let result = if exists {
                try await client.updateWorkflow(id: workflowID, workflow: currentWorkflow)
            } else {
                try await client.createWorkflow(currentWorkflow)
            }

            selectedWorkflowID = result.workflowID
            await refreshWorkflows(createExampleIfMissing: false)
            statusMessage = "Saved \(result.workflowID)"
        } catch {
            statusMessage = error.localizedDescription
            appendEvent(nodeID: nil, title: "Save failed", detail: error.localizedDescription)
        }
    }

    private func deleteWorkflow(id workflowID: String) async {
        do {
            let result = try await client.deleteWorkflow(id: workflowID)
            if result.deleted {
                workflows.removeAll { $0.workflowID == workflowID }
                events.removeAll()
                runID = nil
                statusMessage = "Deleted \(workflowID)"

                if let next = workflows.first {
                    selectedWorkflowID = next.workflowID
                    await loadWorkflow(id: next.workflowID)
                } else {
                    selectedWorkflowID = BusinessWorkflowExample.workflowID
                    currentWorkflow = BusinessWorkflowExample.jsonDefinition
                    syncDisplay(with: currentWorkflow)
                    rawWorkflowJSON = prettyJSONString(currentWorkflow)
                }
            }
        } catch {
            statusMessage = error.localizedDescription
            appendEvent(nodeID: nil, title: "Delete failed", detail: error.localizedDescription)
        }
    }

    private func startViaJSONRPC(workflowID: String) async {
        do {
            let result = try await executionIntent.execute(workflow: currentWorkflow, knownWorkflows: workflows)
            selectedWorkflowID = result.workflowID
            await refreshWorkflows(createExampleIfMissing: false)
            self.runID = result.runID
            observedRunIDs.insert(result.runID)
            statusMessage = "Running \(result.runID)"
            try await streamEvents(runID: result.runID)
        } catch {
            appendEvent(nodeID: nil, title: "Workflow failed", detail: error.localizedDescription)
            statusMessage = error.localizedDescription
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
            }
            applyRunSummary(page.run)

            if !completed {
                try? await Task.sleep(for: .milliseconds(350))
            }
        }

        statusMessage = "Completed \(runID)"
        isRunning = false
    }

    private func syncDisplay(with workflow: [String: Any]) {
        let display = WorkflowDisplayBuilder.build(from: workflow)
        nodes = display.nodes.isEmpty ? BusinessWorkflowExample.nodes : display.nodes
        edges = display.edges
        selectedNodeID = nodes.first?.id
    }

    private func applyServerEvent(_ event: [String: Any]) {
        let type = event["type"] as? String ?? "event"
        let nodeID = event["node_id"] as? String

        switch type {
        case "workflow_started":
            appendEvent(nodeID: nil, title: "Workflow started", detail: selectedWorkflowID ?? "workflow")
        case "workflow_completed":
            appendEvent(nodeID: nil, title: "Workflow completed", detail: "Run finished")
        case "workflow_failed":
            appendEvent(
                nodeID: nil,
                title: "Workflow failed",
                detail: displayString(for: event["error"])
            )
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
        case "node_failed":
            if let nodeID {
                let error = displayString(for: event["error"])
                fail(nodeID, error: error)
                appendEvent(nodeID: nodeID, title: "Node failed", detail: error)
            }
        default:
            appendEvent(nodeID: nodeID, title: type, detail: displayString(for: event))
        }
    }

    private func applyRunSummary(_ run: [String: Any]) {
        guard let status = run["status"] as? String else { return }
        statusMessage = "\(run["run_id"] as? String ?? "Run"): \(status)"
    }

    private func resetNodes() {
        syncDisplay(with: currentWorkflow)
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

    private func fail(_ nodeID: String, error: String) {
        guard let index = nodes.firstIndex(where: { $0.id == nodeID }) else { return }
        nodes[index].status = .failed
        nodes[index].result = error
        selectedNodeID = nodeID
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

private func prettyJSONString(_ object: [String: Any]) -> String {
    guard
        JSONSerialization.isValidJSONObject(object),
        let data = try? JSONSerialization.data(withJSONObject: object, options: [.prettyPrinted, .sortedKeys]),
        let string = String(data: data, encoding: .utf8)
    else {
        return "{}"
    }

    return string
}
