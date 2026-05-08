import SwiftUI

struct NodeInspectorView: View {
    let node: WorkflowNodeModel?
    let onClose: () -> Void

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
                    Image(systemName: AppTheme.kindIcon(node.kind))
                        .font(.system(size: 13, weight: .semibold))
                        .foregroundStyle(AppTheme.kindColor(node.kind))
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
            InspectorTab(label: "Settings", isActive: false)
            InspectorTab(label: "Docs", isActive: false)
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
