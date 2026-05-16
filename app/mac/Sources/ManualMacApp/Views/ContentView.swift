import SwiftUI

struct ContentView: View {
    @StateObject private var store = WorkflowRunStore()
    @SceneStorage("ManualMac.sidebarVisible") private var sidebarVisible = true
    @SceneStorage("ManualMac.inspectorVisible") private var inspectorVisible = false
    @SceneStorage("ManualMac.bottomPanelVisible") private var bottomPanelVisible = false
    @State private var showingOverrideSheet = false
    @State private var overrideTargetNodeID: String? = nil

    var body: some View {
        ZStack {
            AppTheme.canvas.ignoresSafeArea()

            HStack(spacing: 0) {
                LeftRail(sidebarVisible: $sidebarVisible)
                    .frame(width: 56)

                if sidebarVisible {
                    WorkflowSidebar(store: store)
                        .frame(width: 260)
                        .transition(.move(edge: .leading).combined(with: .opacity))
                }

                VStack(spacing: 0) {
                    TopBar(
                        store: store,
                        sidebarVisible: $sidebarVisible,
                        inspectorVisible: $inspectorVisible,
                        bottomPanelVisible: $bottomPanelVisible
                    )
                        .frame(height: 54)

                    HStack(spacing: 0) {
                        ZStack(alignment: .bottom) {
                            WorkflowGraphView(
                                nodes: store.nodes,
                                edges: store.edges,
                                selectedNodeID: store.selectedNodeID,
                                isRunning: store.isRunning,
                                onSelect: { nodeID in
                                    store.selectNode(nodeID)
                                    withAnimation(.easeInOut(duration: 0.18)) {
                                        inspectorVisible = true
                                    }
                                },
                                onRun: { nodeID in store.runNode(nodeID) },
                                onRestart: { _ in store.restartFromFailure() },
                                onStop: { store.stop() },
                                onOverride: { nodeID in
                                    overrideTargetNodeID = nodeID
                                    showingOverrideSheet = true
                                }
                            )
                            .frame(maxWidth: .infinity, maxHeight: .infinity)

                            VStack(spacing: 0) {
                                if bottomPanelVisible {
                                    BottomPanel(
                                        events: store.events,
                                        rawWorkflowJSON: store.rawWorkflowJSON,
                                        onClose: {
                                            withAnimation(.easeInOut(duration: 0.18)) {
                                                bottomPanelVisible = false
                                            }
                                        }
                                    )
                                    .frame(height: 220)
                                    .transition(.move(edge: .bottom).combined(with: .opacity))
                                }

                                BottomToggle(visible: $bottomPanelVisible, eventCount: store.events.count)
                                    .frame(height: 30)
                            }
                            .background(AppTheme.panel.opacity(0.98))
                            .shadow(color: .black.opacity(bottomPanelVisible ? 0.35 : 0), radius: 16, y: -4)
                        }
                        .frame(maxWidth: .infinity, maxHeight: .infinity)

                        if inspectorVisible {
                            Rectangle().fill(AppTheme.stroke).frame(width: 1)
                            NodeInspectorView(
                                node: store.selectedNode,
                                store: store,
                                onClose: {
                                    withAnimation(.easeInOut(duration: 0.18)) {
                                        inspectorVisible = false
                                    }
                                },
                                onOverride: { nodeID in
                                    overrideTargetNodeID = nodeID
                                    showingOverrideSheet = true
                                }
                            )
                            .frame(width: 340)
                            .transition(.move(edge: .trailing).combined(with: .opacity))
                        }
                    }
                }
            }
        }
        .sheet(isPresented: $showingOverrideSheet) {
            if let nodeID = overrideTargetNodeID,
               let node = store.nodes.first(where: { $0.id == nodeID }) {
                NodeInputOverrideSheet(
                    nodeID: nodeID,
                    nodeTitle: node.title,
                    onRun: { overrides in
                        store.runNode(nodeID, overrides: overrides)
                        showingOverrideSheet = false
                    },
                    onCancel: { showingOverrideSheet = false }
                )
            }
        }
        .preferredColorScheme(.dark)
        .onAppear {
            store.bootstrap()
        }
        .onReceive(NotificationCenter.default.publisher(for: .startExampleWorkflow)) { _ in
            withAnimation(.easeInOut(duration: 0.18)) {
                bottomPanelVisible = true
            }
            store.start()
        }
    }
}

// MARK: - Left rail

private struct LeftRail: View {
    @Binding var sidebarVisible: Bool

    var body: some View {
        VStack(spacing: 6) {
            BrandLogo()
                .padding(.top, 14)
                .padding(.bottom, 14)

            RailButton(symbol: "square.grid.2x2.fill", isActive: sidebarVisible) {
                withAnimation(.easeInOut(duration: 0.18)) {
                    sidebarVisible.toggle()
                }
            }
            /*
            RailButton(symbol: "clock.arrow.circlepath", isActive: false)
            RailButton(symbol: "key.fill", isActive: false)
            RailButton(symbol: "person.2.fill", isActive: false)
            */

            Spacer()

            /*
            RailButton(symbol: "questionmark.circle", isActive: false)
            RailButton(symbol: "gearshape", isActive: false)
                .padding(.bottom, 14)
            */
        }
        .frame(maxHeight: .infinity)
        .background(AppTheme.rail)
        .overlay(alignment: .trailing) {
            Rectangle().fill(AppTheme.stroke).frame(width: 1)
        }
    }
}

private struct BrandLogo: View {
    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 9)
                .fill(
                    LinearGradient(
                        colors: [AppTheme.accent, Color(red: 0.95, green: 0.30, blue: 0.50)],
                        startPoint: .topLeading,
                        endPoint: .bottomTrailing
                    )
                )
                .frame(width: 34, height: 34)
                .shadow(color: AppTheme.accent.opacity(0.4), radius: 8, y: 2)
            Text("M")
                .font(.system(size: 17, weight: .black, design: .rounded))
                .foregroundStyle(.white)
        }
    }
}

private struct WorkflowSidebar: View {
    @ObservedObject var store: WorkflowRunStore

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 8) {
                Text("Workflows")
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(AppTheme.text)
                Spacer()
                Button {
                    store.refresh()
                } label: {
                    Image(systemName: "arrow.clockwise")
                        .font(.system(size: 11, weight: .semibold))
                        .foregroundStyle(AppTheme.textMuted)
                        .frame(width: 26, height: 24)
                }
                .buttonStyle(.plain)
                .disabled(store.isLoading)
            }
            .padding(.horizontal, 14)
            .frame(height: 44)
            .overlay(alignment: .bottom) {
                Rectangle().fill(AppTheme.stroke).frame(height: 1)
            }

            ScrollView {
                LazyVStack(spacing: 6) {
                    ForEach(store.workflows) { workflow in
                        WorkflowListRow(
                            workflow: workflow,
                            isSelected: workflow.workflowID == store.selectedWorkflowID
                        ) {
                            store.selectWorkflow(workflow.workflowID)
                        }
                    }
                }
                .padding(10)
            }

            VStack(alignment: .leading, spacing: 8) {
                Text(store.statusMessage)
                    .font(.system(size: 11))
                    .foregroundStyle(AppTheme.textMuted)
                    .lineLimit(2)
                    .frame(maxWidth: .infinity, alignment: .leading)

                HStack(spacing: 8) {
                    SmallActionButton(symbol: "square.and.arrow.down", label: "Save") {
                        store.saveSelectedWorkflow()
                    }
                    SmallActionButton(symbol: "trash", label: "Delete", isDestructive: true) {
                        store.deleteSelectedWorkflow()
                    }
                    .disabled(!store.hasSelectedWorkflow || store.isRunning)
                }
            }
            .padding(12)
            .overlay(alignment: .top) {
                Rectangle().fill(AppTheme.stroke).frame(height: 1)
            }
        }
        .background(AppTheme.panel)
        .overlay(alignment: .trailing) {
            Rectangle().fill(AppTheme.stroke).frame(width: 1)
        }
    }
}

private struct WorkflowListRow: View {
    let workflow: WorkflowSummary
    let isSelected: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 10) {
                Image(systemName: isSelected ? "flowchart.fill" : "flowchart")
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(isSelected ? AppTheme.accent : AppTheme.textMuted)
                    .frame(width: 18)

                VStack(alignment: .leading, spacing: 3) {
                    Text(workflow.workflowID)
                        .font(.system(size: 12, weight: .semibold))
                        .foregroundStyle(AppTheme.text)
                        .lineLimit(1)
                    Text("\(workflow.nodeCount) nodes")
                        .font(.system(size: 10))
                        .foregroundStyle(AppTheme.textMuted)
                }

                Spacer()
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 9)
            .background(
                RoundedRectangle(cornerRadius: 8)
                    .fill(isSelected ? AppTheme.panelElev : Color.clear)
            )
            .overlay {
                RoundedRectangle(cornerRadius: 8)
                    .stroke(isSelected ? AppTheme.strokeStrong : Color.clear, lineWidth: 1)
            }
        }
        .buttonStyle(.plain)
    }
}

private struct SmallActionButton: View {
    let symbol: String
    let label: String
    var isDestructive = false
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 5) {
                Image(systemName: symbol)
                    .font(.system(size: 10, weight: .semibold))
                Text(label)
                    .font(.system(size: 11, weight: .semibold))
            }
            .foregroundStyle(isDestructive ? Color(red: 0.96, green: 0.45, blue: 0.45) : AppTheme.text)
            .padding(.horizontal, 10)
            .padding(.vertical, 6)
            .background(RoundedRectangle(cornerRadius: 7).fill(AppTheme.panelElev))
            .overlay {
                RoundedRectangle(cornerRadius: 7).stroke(AppTheme.stroke, lineWidth: 1)
            }
        }
        .buttonStyle(.plain)
    }
}

private struct RailButton: View {
    let symbol: String
    let isActive: Bool
    let action: (() -> Void)?

    init(symbol: String, isActive: Bool, action: (() -> Void)? = nil) {
        self.symbol = symbol
        self.isActive = isActive
        self.action = action
    }

    var body: some View {
        ZStack {
            if isActive {
                RoundedRectangle(cornerRadius: 9)
                    .fill(AppTheme.panelElev)
                    .frame(width: 38, height: 38)
            }
            Image(systemName: symbol)
                .font(.system(size: 15, weight: .medium))
                .foregroundStyle(isActive ? AppTheme.text : AppTheme.textMuted)
        }
        .frame(width: 40, height: 40)
        .contentShape(Rectangle())
        .onTapGesture {
            action?()
        }
    }
}

// MARK: - Top bar

private struct TopBar: View {
    @ObservedObject var store: WorkflowRunStore
    @Binding var sidebarVisible: Bool
    @Binding var inspectorVisible: Bool
    @Binding var bottomPanelVisible: Bool

    var body: some View {
        HStack(spacing: 14) {
            IconButton(symbol: "sidebar.left", isActive: sidebarVisible) {
                withAnimation(.easeInOut(duration: 0.18)) {
                    sidebarVisible.toggle()
                }
            }

            VStack(alignment: .leading, spacing: 2) {
                HStack(spacing: 6) {
                    Text("Personal")
                        .font(.system(size: 11, weight: .medium))
                        .foregroundStyle(AppTheme.textMuted)
                    Image(systemName: "chevron.right")
                        .font(.system(size: 8, weight: .bold))
                        .foregroundStyle(AppTheme.textFaint)
                    Text("Business Pipeline Health")
                        .font(.system(size: 11, weight: .medium))
                        .foregroundStyle(AppTheme.text)
                }
                HStack(spacing: 8) {
                    Text(store.runID ?? "Draft run")
                        .font(.system(size: 13, weight: .semibold))
                        .foregroundStyle(AppTheme.text)
                    Circle()
                        .fill(AppTheme.textFaint)
                        .frame(width: 3, height: 3)
                    Text(store.progressText)
                        .font(.system(size: 11))
                        .foregroundStyle(AppTheme.textMuted)
                }
            }

            Spacer()

            /*
            HStack(spacing: 0) {
                TabPill(label: "Editor", isActive: true)
                TabPill(label: "Executions", isActive: false)
            }
            .padding(3)
            .background(AppTheme.panelElev)
            .clipShape(Capsule())
            .overlay {
                Capsule().stroke(AppTheme.stroke, lineWidth: 1)
            }

            Spacer()
            */

            HStack(spacing: 8) {
                SecondaryButton(label: "Save") {
                    store.saveSelectedWorkflow()
                }
                SecondaryButton(label: "Refresh") {
                    store.refresh()
                }
                ExecuteButton(isRunning: store.isRunning) {
                    withAnimation(.easeInOut(duration: 0.18)) {
                        bottomPanelVisible = true
                    }
                    store.start()
                }
                if store.isRunning && !store.isPaused {
                    SecondaryButton(label: "Stop") {
                        store.stop()
                    }
                    .keyboardShortcut(".", modifiers: .command)
                }
                if store.isResumable {
                    SecondaryButton(label: store.isPaused ? "Next step" : "Resume") {
                        if store.isPaused {
                            store.resumeStep()
                        } else {
                            store.restartFromFailure()
                        }
                    }
                }
                if !store.isRunning {
                    IconButton(symbol: "forward.frame", isActive: false) {
                        store.startStepMode()
                    }
                }
                IconButton(symbol: "rectangle.bottomthird.inset.filled", isActive: bottomPanelVisible) {
                    withAnimation(.easeInOut(duration: 0.18)) {
                        bottomPanelVisible.toggle()
                    }
                }
                IconButton(symbol: "sidebar.right", isActive: inspectorVisible) {
                    withAnimation(.easeInOut(duration: 0.18)) {
                        inspectorVisible.toggle()
                    }
                }
            }
        }
        .padding(.horizontal, 18)
        .frame(maxWidth: .infinity)
        .background(AppTheme.topBar)
        .overlay(alignment: .bottom) {
            Rectangle().fill(AppTheme.stroke).frame(height: 1)
        }
    }
}

private struct TabPill: View {
    let label: String
    let isActive: Bool

    var body: some View {
        Text(label)
            .font(.system(size: 12, weight: .semibold))
            .foregroundStyle(isActive ? AppTheme.text : AppTheme.textMuted)
            .padding(.horizontal, 14)
            .padding(.vertical, 5)
            .background(
                Capsule().fill(isActive ? AppTheme.panel : Color.clear)
            )
    }
}

private struct SecondaryButton: View {
    let label: String
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Text(label)
                .font(.system(size: 12, weight: .semibold))
                .foregroundStyle(AppTheme.text)
                .padding(.horizontal, 14)
                .padding(.vertical, 7)
                .background(
                    Capsule().fill(AppTheme.panelElev)
                )
                .overlay {
                    Capsule().stroke(AppTheme.stroke, lineWidth: 1)
                }
        }
        .buttonStyle(.plain)
    }
}

private struct ExecuteButton: View {
    let isRunning: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 6) {
                Image(systemName: isRunning ? "hourglass" : "play.fill")
                    .font(.system(size: 11, weight: .bold))
                Text(isRunning ? "Running…" : "Execute workflow")
                    .font(.system(size: 12, weight: .semibold))
            }
            .foregroundStyle(.white)
            .padding(.horizontal, 16)
            .padding(.vertical, 8)
            .background(
                Capsule().fill(
                    LinearGradient(
                        colors: isRunning
                            ? [AppTheme.accent.opacity(0.55), AppTheme.accent.opacity(0.55)]
                            : [AppTheme.accent, Color(red: 0.95, green: 0.30, blue: 0.50)],
                        startPoint: .leading,
                        endPoint: .trailing
                    )
                )
            )
            .shadow(color: AppTheme.accent.opacity(isRunning ? 0 : 0.35), radius: 8, y: 2)
        }
        .buttonStyle(.plain)
        .disabled(isRunning)
        .keyboardShortcut("r", modifiers: [.command])
    }
}

private struct IconButton: View {
    let symbol: String
    let isActive: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Image(systemName: symbol)
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(isActive ? AppTheme.text : AppTheme.textMuted)
                .frame(width: 32, height: 30)
                .background(
                    RoundedRectangle(cornerRadius: 7)
                        .fill(isActive ? AppTheme.panelElev : Color.clear)
                )
        }
        .buttonStyle(.plain)
    }
}

// MARK: - Bottom toggle / panel

private struct BottomToggle: View {
    @Binding var visible: Bool
    let eventCount: Int

    var body: some View {
        HStack(spacing: 0) {
            Button {
                withAnimation(.easeInOut(duration: 0.18)) { visible.toggle() }
            } label: {
                HStack(spacing: 8) {
                    Image(systemName: visible ? "chevron.down" : "chevron.up")
                        .font(.system(size: 9, weight: .bold))
                    Text("Logs")
                        .font(.system(size: 11, weight: .semibold))
                    if eventCount > 0 {
                        Text("\(eventCount)")
                            .font(.system(size: 10, weight: .bold))
                            .foregroundStyle(AppTheme.accent)
                            .padding(.horizontal, 7)
                            .padding(.vertical, 1)
                            .background(Capsule().fill(AppTheme.accentMuted))
                    }
                }
                .foregroundStyle(AppTheme.textMuted)
                .padding(.horizontal, 14)
                .frame(maxHeight: .infinity)
            }
            .buttonStyle(.plain)

            Spacer()
        }
        .frame(maxWidth: .infinity)
        .background(AppTheme.panel)
        .overlay(alignment: .top) {
            Rectangle().fill(AppTheme.stroke).frame(height: 1)
        }
    }
}

private struct BottomPanel: View {
    let events: [WorkflowEventModel]
    let rawWorkflowJSON: String
    let onClose: () -> Void
    @State private var selectedTab: BottomPanelTab = .events

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 0) {
                LogTab(label: "Executions", isActive: selectedTab == .events) {
                    selectedTab = .events
                }
                LogTab(label: "JSON", isActive: selectedTab == .json) {
                    selectedTab = .json
                }
                Spacer()
                Button(action: onClose) {
                    Image(systemName: "xmark")
                        .font(.system(size: 10, weight: .semibold))
                        .foregroundStyle(AppTheme.textMuted)
                        .padding(.horizontal, 12)
                        .frame(maxHeight: .infinity)
                }
                .buttonStyle(.plain)
            }
            .frame(height: 36)
            .background(AppTheme.panel)
            .overlay(alignment: .bottom) {
                Rectangle().fill(AppTheme.stroke).frame(height: 1)
            }
            .overlay(alignment: .top) {
                Rectangle().fill(AppTheme.stroke).frame(height: 1)
            }

            if selectedTab == .events {
                EventTimelineView(events: events)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .background(AppTheme.panel)
            } else {
                ScrollView {
                    Text(rawWorkflowJSON)
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundStyle(AppTheme.textMuted)
                        .textSelection(.enabled)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(16)
                }
                .background(AppTheme.panel)
            }
        }
        .background(AppTheme.panel)
    }
}

private enum BottomPanelTab {
    case events
    case json
}

private struct LogTab: View {
    let label: String
    let isActive: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Text(label)
                .font(.system(size: 12, weight: .semibold))
                .foregroundStyle(isActive ? AppTheme.text : AppTheme.textMuted)
                .padding(.horizontal, 18)
                .frame(maxHeight: .infinity)
                .overlay(alignment: .bottom) {
                    Rectangle()
                        .fill(isActive ? AppTheme.accent : Color.clear)
                        .frame(height: 2)
                }
        }
        .buttonStyle(.plain)
    }
}
