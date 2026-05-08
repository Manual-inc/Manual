import SwiftUI

struct ContentView: View {
    @StateObject private var store = WorkflowRunStore()
    @State private var inspectorVisible = true
    @State private var bottomPanelVisible = true

    var body: some View {
        ZStack {
            AppTheme.canvas.ignoresSafeArea()

            HStack(spacing: 0) {
                LeftRail()
                    .frame(width: 56)

                VStack(spacing: 0) {
                    TopBar(store: store, inspectorVisible: $inspectorVisible)
                        .frame(height: 54)

                    HStack(spacing: 0) {
                        VStack(spacing: 0) {
                            WorkflowGraphView(
                                nodes: store.nodes,
                                edges: store.edges,
                                selectedNodeID: store.selectedNodeID,
                                onSelect: store.selectNode
                            )
                            .frame(maxWidth: .infinity, maxHeight: .infinity)

                            if bottomPanelVisible {
                                BottomPanel(
                                    events: store.events,
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
                        .frame(maxWidth: .infinity, maxHeight: .infinity)

                        if inspectorVisible {
                            Rectangle().fill(AppTheme.stroke).frame(width: 1)
                            NodeInspectorView(
                                node: store.selectedNode,
                                onClose: {
                                    withAnimation(.easeInOut(duration: 0.18)) {
                                        inspectorVisible = false
                                    }
                                }
                            )
                            .frame(width: 340)
                            .transition(.move(edge: .trailing).combined(with: .opacity))
                        }
                    }
                }
            }
        }
        .preferredColorScheme(.dark)
        .onReceive(NotificationCenter.default.publisher(for: .startExampleWorkflow)) { _ in
            store.start()
        }
    }
}

// MARK: - Left rail

private struct LeftRail: View {
    var body: some View {
        VStack(spacing: 6) {
            BrandLogo()
                .padding(.top, 14)
                .padding(.bottom, 14)

            RailButton(symbol: "square.grid.2x2.fill", isActive: true)
            RailButton(symbol: "clock.arrow.circlepath", isActive: false)
            RailButton(symbol: "key.fill", isActive: false)
            RailButton(symbol: "person.2.fill", isActive: false)

            Spacer()

            RailButton(symbol: "questionmark.circle", isActive: false)
            RailButton(symbol: "gearshape", isActive: false)
                .padding(.bottom, 14)
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

private struct RailButton: View {
    let symbol: String
    let isActive: Bool

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
    }
}

// MARK: - Top bar

private struct TopBar: View {
    @ObservedObject var store: WorkflowRunStore
    @Binding var inspectorVisible: Bool

    var body: some View {
        HStack(spacing: 14) {
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

            HStack(spacing: 8) {
                SecondaryButton(label: "Save")
                ExecuteButton(isRunning: store.isRunning) {
                    store.start()
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

    var body: some View {
        Button {} label: {
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
    let onClose: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 0) {
                LogTab(label: "Executions", isActive: true)
                LogTab(label: "Output", isActive: false)
                LogTab(label: "JSON", isActive: false)
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

            EventTimelineView(events: events)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .background(AppTheme.panel)
        }
        .background(AppTheme.panel)
    }
}

private struct LogTab: View {
    let label: String
    let isActive: Bool

    var body: some View {
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
}
