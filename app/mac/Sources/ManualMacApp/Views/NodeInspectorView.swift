import SwiftUI

struct NodeInspectorView: View {
    let node: WorkflowNodeModel?
    let onClose: () -> Void
    var store: WorkflowRunStore? = nil
    var onOverride: ((String) -> Void)? = nil

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            header
            tabBar

            ScrollView {
                if let node {
                    contentBody(for: node)
                } else {
                    emptyState
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .frame(maxHeight: .infinity)
        .background(AppTheme.panel)
    }

    // MARK: header

    private var header: some View {
        HStack(spacing: 10) {
            if let node {
                ZStack {
                    RoundedRectangle(cornerRadius: 8)
                        .fill(AppTheme.kindColor(node.kind).opacity(0.18))
                    RoundedRectangle(cornerRadius: 8)
                        .stroke(AppTheme.kindColor(node.kind).opacity(0.5), lineWidth: 1)
                    NodeKindIcon(kind: node.kind, symbolSize: 13)
                }
                .frame(width: 30, height: 30)

                VStack(alignment: .leading, spacing: 1) {
                    Text(node.title)
                        .font(.system(size: 13, weight: .semibold))
                        .foregroundStyle(AppTheme.text)
                        .lineLimit(1)
                    Text(node.kind.rawValue)
                        .font(.system(size: 11))
                        .foregroundStyle(AppTheme.textMuted)
                }
            } else {
                Text("Inspector")
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(AppTheme.text)
            }

            Spacer()

            Button(action: onClose) {
                Image(systemName: "xmark")
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(AppTheme.textMuted)
                    .frame(width: 24, height: 24)
                    .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 12)
        .overlay(alignment: .bottom) {
            Rectangle().fill(AppTheme.stroke).frame(height: 1)
        }
    }

    private var tabBar: some View {
        HStack(spacing: 0) {
            InspectorTab(label: "Parameters", isActive: true)
            /*
            InspectorTab(label: "Settings", isActive: false)
            InspectorTab(label: "Docs", isActive: false)
            */
            Spacer()
        }
        .frame(height: 36)
        .overlay(alignment: .bottom) {
            Rectangle().fill(AppTheme.stroke).frame(height: 1)
        }
    }

    // MARK: content

    private func contentBody(for node: WorkflowNodeModel) -> some View {
        VStack(alignment: .leading, spacing: 16) {
            // STEP MODE banner
            if let store, store.isPaused {
                HStack(spacing: 10) {
                    Image(systemName: "pause.circle.fill")
                        .font(.system(size: 14))
                        .foregroundStyle(Color.orange)
                    VStack(alignment: .leading, spacing: 1) {
                        Text("Paused — step mode")
                            .font(.system(size: 12, weight: .semibold))
                            .foregroundStyle(AppTheme.text)
                        Text("Tap 'Run next step' to advance")
                            .font(.system(size: 11))
                            .foregroundStyle(AppTheme.textMuted)
                    }
                    Spacer()
                    Button("Next step") {
                        store.resumeStep()
                    }
                    .buttonStyle(.plain)
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(.white)
                    .padding(.horizontal, 12)
                    .padding(.vertical, 6)
                    .background(Capsule().fill(Color.orange))
                }
                .padding(12)
                .background(RoundedRectangle(cornerRadius: 8).fill(Color.orange.opacity(0.08)))
                .overlay { RoundedRectangle(cornerRadius: 8).stroke(Color.orange.opacity(0.3), lineWidth: 1) }
            }

            InspectorRow(label: "STATUS") {
                HStack(spacing: 7) {
                    Circle()
                        .fill(AppTheme.statusColor(node.status))
                        .frame(width: 7, height: 7)
                    Text(node.status.rawValue)
                        .font(.system(size: 12, weight: .medium))
                        .foregroundStyle(AppTheme.text)
                }
            }

            InspectorRow(label: "ID") {
                Text(node.id)
                    .font(.system(size: 12, design: .monospaced))
                    .foregroundStyle(AppTheme.textMuted)
                    .textSelection(.enabled)
            }

            InspectorRow(label: "DESCRIPTION") {
                Text(node.subtitle)
                    .font(.system(size: 12))
                    .foregroundStyle(AppTheme.textMuted)
                    .fixedSize(horizontal: false, vertical: true)
            }

            InspectorRow(label: "OUTPUT") {
                Text(node.result ?? "Waiting for this node to run.")
                    .font(.system(size: 12, design: .monospaced))
                    .foregroundStyle(node.result == nil ? AppTheme.textFaint : AppTheme.text)
                    .padding(12)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(
                        RoundedRectangle(cornerRadius: 8).fill(AppTheme.canvas)
                    )
                    .overlay {
                        RoundedRectangle(cornerRadius: 8).stroke(AppTheme.stroke, lineWidth: 1)
                    }
                    .textSelection(.enabled)
            }

            // ACTIONS
            if let store {
                InspectorRow(label: "ACTIONS") {
                    VStack(spacing: 6) {
                        // Run / Restart
                        if node.status != .running {
                            HStack(spacing: 6) {
                                if node.status == .failed {
                                    ActionButton(label: "Restart from here", symbol: "arrow.clockwise") {
                                        store.restartFromFailure()
                                    }
                                } else {
                                    ActionButton(label: "Run node", symbol: "play.fill") {
                                        store.runNode(node.id)
                                    }
                                }
                                ActionButton(label: "Override input…", symbol: "pencil") {
                                    onOverride?(node.id)
                                }
                            }
                        }
                        // Stop
                        if node.status == .running && store.isRunning {
                            ActionButton(label: "Stop", symbol: "stop.fill") {
                                store.stop()
                            }
                        }
                        // Step mode resume
                        if store.isPaused && (node.status == .paused || store.selectedNodeID == node.id) {
                            ActionButton(label: "Run next step", symbol: "forward.fill", accent: true) {
                                store.resumeStep()
                            }
                        }
                    }
                }
            }

            // PREVIOUS OUTPUT
            if let previousResult = node.previousResult, previousResult != node.result {
                InspectorRow(label: "PREVIOUS OUTPUT") {
                    VStack(alignment: .leading, spacing: 8) {
                        // Old result
                        VStack(alignment: .leading, spacing: 4) {
                            Text("BEFORE")
                                .font(.system(size: 9, weight: .bold))
                                .tracking(0.5)
                                .foregroundStyle(AppTheme.textFaint)
                            Text(previousResult)
                                .font(.system(size: 11, design: .monospaced))
                                .foregroundStyle(AppTheme.textMuted)
                                .padding(10)
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .background(RoundedRectangle(cornerRadius: 6).fill(AppTheme.canvas))
                                .overlay { RoundedRectangle(cornerRadius: 6).stroke(AppTheme.stroke, lineWidth: 1) }
                                .textSelection(.enabled)
                        }
                        // New result
                        if let result = node.result {
                            VStack(alignment: .leading, spacing: 4) {
                                Text("AFTER")
                                    .font(.system(size: 9, weight: .bold))
                                    .tracking(0.5)
                                    .foregroundStyle(Color.green.opacity(0.7))
                                Text(result)
                                    .font(.system(size: 11, design: .monospaced))
                                    .foregroundStyle(AppTheme.text)
                                    .padding(10)
                                    .frame(maxWidth: .infinity, alignment: .leading)
                                    .background(RoundedRectangle(cornerRadius: 6).fill(AppTheme.canvas))
                                    .overlay { RoundedRectangle(cornerRadius: 6).stroke(Color.green.opacity(0.3), lineWidth: 1) }
                                    .textSelection(.enabled)
                            }
                        }
                    }
                }
            }
        }
        .padding(16)
        .frame(maxWidth: .infinity, alignment: .topLeading)
    }

    private var emptyState: some View {
        VStack(spacing: 10) {
            Image(systemName: "cursorarrow.click.2")
                .font(.system(size: 30))
                .foregroundStyle(AppTheme.textFaint)
            Text("No node selected")
                .font(.system(size: 13, weight: .semibold))
                .foregroundStyle(AppTheme.textMuted)
            Text("Click any node on the canvas to inspect its parameters and execution output.")
                .font(.system(size: 11))
                .foregroundStyle(AppTheme.textFaint)
                .multilineTextAlignment(.center)
                .frame(maxWidth: 220)
        }
        .frame(maxWidth: .infinity)
        .padding(.top, 60)
        .padding(.horizontal, 24)
    }
}

private struct InspectorTab: View {
    let label: String
    let isActive: Bool

    var body: some View {
        Text(label)
            .font(.system(size: 12, weight: .semibold))
            .foregroundStyle(isActive ? AppTheme.text : AppTheme.textMuted)
            .padding(.horizontal, 14)
            .frame(maxHeight: .infinity)
            .overlay(alignment: .bottom) {
                Rectangle().fill(isActive ? AppTheme.accent : Color.clear).frame(height: 2)
            }
    }
}

private struct InspectorRow<Content: View>: View {
    let label: String
    @ViewBuilder var content: () -> Content

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(label)
                .font(.system(size: 10, weight: .bold))
                .tracking(0.7)
                .foregroundStyle(AppTheme.textFaint)
            content()
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

private struct ActionButton: View {
    let label: String
    let symbol: String
    var accent: Bool = false
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 5) {
                Image(systemName: symbol)
                    .font(.system(size: 10, weight: .bold))
                Text(label)
                    .font(.system(size: 11, weight: .semibold))
            }
            .foregroundStyle(accent ? .white : AppTheme.text)
            .padding(.horizontal, 12)
            .padding(.vertical, 6)
            .frame(maxWidth: .infinity)
            .background(
                RoundedRectangle(cornerRadius: 7)
                    .fill(accent ? AppTheme.accent : AppTheme.panelElev)
            )
            .overlay {
                RoundedRectangle(cornerRadius: 7)
                    .stroke(accent ? Color.clear : AppTheme.stroke, lineWidth: 1)
            }
        }
        .buttonStyle(.plain)
    }
}
