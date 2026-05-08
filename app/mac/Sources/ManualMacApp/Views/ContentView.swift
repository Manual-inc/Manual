import SwiftUI

struct ContentView: View {
    @StateObject private var store = WorkflowRunStore()

    var body: some View {
        NavigationSplitView {
            WorkflowSidebarView(store: store)
                .navigationSplitViewColumnWidth(min: 240, ideal: 280, max: 340)
        } detail: {
            WorkflowDashboardView(store: store)
        }
        .onReceive(NotificationCenter.default.publisher(for: .startExampleWorkflow)) { _ in
            store.start()
        }
    }
}

private struct WorkflowSidebarView: View {
    @ObservedObject var store: WorkflowRunStore

    var body: some View {
        List(selection: Binding(
            get: { store.selectedNodeID },
            set: { if let id = $0 { store.selectNode(id) } }
        )) {
            Section("Example Workflow") {
                ForEach(store.nodes) { node in
                    WorkflowSidebarRow(node: node)
                        .tag(node.id)
                }
            }
        }
        .listStyle(.sidebar)
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Button {
                    store.start()
                } label: {
                    Label(store.isRunning ? "Running" : "Start", systemImage: store.isRunning ? "hourglass" : "play.fill")
                }
                .disabled(store.isRunning)
            }
        }
    }
}

private struct WorkflowSidebarRow: View {
    let node: WorkflowNodeModel

    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: node.status.symbolName)
                .foregroundStyle(statusColor)
                .frame(width: 16)

            VStack(alignment: .leading, spacing: 2) {
                Text(node.title)
                    .lineLimit(1)

                Text(node.kind.rawValue)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }
        }
    }

    private var statusColor: Color {
        switch node.status {
        case .idle: .secondary
        case .running: .blue
        case .succeeded: .green
        case .failed: .red
        }
    }
}

private struct WorkflowDashboardView: View {
    @ObservedObject var store: WorkflowRunStore

    var body: some View {
        VStack(spacing: 0) {
            WorkflowHeaderView(store: store)

            Divider()

            HSplitView {
                VStack(spacing: 12) {
                    WorkflowGraphView(
                        nodes: store.nodes,
                        edges: store.edges,
                        selectedNodeID: store.selectedNodeID,
                        onSelect: store.selectNode
                    )
                    .frame(minHeight: 420)

                    NodeInspectorView(node: store.selectedNode)
                        .frame(height: 140)
                }
                .frame(minWidth: 620)

                EventTimelineView(events: store.events)
                    .frame(minWidth: 300, idealWidth: 360)
            }
            .padding(16)
        }
    }
}

private struct WorkflowHeaderView: View {
    @ObservedObject var store: WorkflowRunStore

    var body: some View {
        HStack(spacing: 14) {
            VStack(alignment: .leading, spacing: 4) {
                Text("Business Pipeline Health")
                    .font(.title2.weight(.semibold))

                Text(store.runID ?? "Ready to run")
                    .foregroundStyle(.secondary)
            }

            Spacer()

            HStack(spacing: 10) {
                Label(store.progressText, systemImage: "checklist")
                    .foregroundStyle(.secondary)

                Button {
                    store.start()
                } label: {
                    Label(store.isRunning ? "Running" : "Start", systemImage: store.isRunning ? "hourglass" : "play.fill")
                        .frame(minWidth: 86)
                }
                .buttonStyle(.borderedProminent)
                .disabled(store.isRunning)
                .keyboardShortcut("r", modifiers: [.command])
            }
        }
        .padding(.horizontal, 18)
        .padding(.vertical, 14)
    }
}
