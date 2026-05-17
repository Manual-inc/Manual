import SwiftUI

struct SandboxPolicyPanel: View {
    @ObservedObject var store: WorkflowRunStore

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            header

            ScrollView {
                VStack(alignment: .leading, spacing: 14) {
                    sandboxList
                    policyEditor
                    assignmentSection
                    probeSection
                    historySection
                }
                .padding(14)
            }
        }
        .background(AppTheme.panel)
        .overlay(alignment: .trailing) {
            Rectangle().fill(AppTheme.stroke).frame(width: 1)
        }
    }

    private var header: some View {
        HStack(spacing: 10) {
            Image(systemName: "lock.shield.fill")
                .font(.system(size: 13, weight: .semibold))
                .foregroundStyle(AppTheme.accent)
                .frame(width: 28, height: 28)
                .background(RoundedRectangle(cornerRadius: 7).fill(AppTheme.accentMuted))

            VStack(alignment: .leading, spacing: 2) {
                Text("Sandboxes")
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(AppTheme.text)
                Text(store.currentSandboxBackend.isEmpty ? "Policy backend" : store.currentSandboxBackend)
                    .font(.system(size: 11))
                    .foregroundStyle(AppTheme.textMuted)
            }

            Spacer()

            Button {
                store.refreshSandboxes()
            } label: {
                Image(systemName: "arrow.clockwise")
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(AppTheme.textMuted)
                    .frame(width: 26, height: 24)
            }
            .buttonStyle(.plain)
        }
        .padding(.horizontal, 14)
        .frame(height: 50)
        .overlay(alignment: .bottom) {
            Rectangle().fill(AppTheme.stroke).frame(height: 1)
        }
    }

    private var sandboxList: some View {
        VStack(alignment: .leading, spacing: 8) {
            SectionLabel("TYPES")

            ForEach(store.sandboxes) { sandbox in
                Button {
                    store.selectSandbox(sandbox.id)
                } label: {
                    HStack(spacing: 9) {
                        Image(systemName: sandbox.id == store.selectedSandboxID ? "shield.lefthalf.filled" : "shield")
                            .font(.system(size: 12, weight: .semibold))
                            .foregroundStyle(sandbox.id == store.selectedSandboxID ? AppTheme.accent : AppTheme.textMuted)
                            .frame(width: 18)

                        VStack(alignment: .leading, spacing: 2) {
                            Text(sandbox.name)
                                .font(.system(size: 12, weight: .semibold))
                                .foregroundStyle(AppTheme.text)
                                .lineLimit(1)
                            Text("\(sandbox.allowWrite.count) write, \(sandbox.allowCommands.count) command")
                                .font(.system(size: 10))
                                .foregroundStyle(AppTheme.textMuted)
                        }

                        Spacer()
                    }
                    .padding(.horizontal, 10)
                    .padding(.vertical, 8)
                    .background(
                        RoundedRectangle(cornerRadius: 8)
                            .fill(sandbox.id == store.selectedSandboxID ? AppTheme.panelElev : Color.clear)
                    )
                    .overlay {
                        RoundedRectangle(cornerRadius: 8)
                            .stroke(sandbox.id == store.selectedSandboxID ? AppTheme.strokeStrong : AppTheme.stroke, lineWidth: 1)
                    }
                }
                .buttonStyle(.plain)
            }

            HStack(spacing: 8) {
                SandboxActionButton(symbol: "plus", label: "New") {
                    store.createSandbox()
                }
                SandboxActionButton(symbol: "checkmark", label: "Save") {
                    store.saveSelectedSandbox()
                }
            }
        }
    }

    private var policyEditor: some View {
        VStack(alignment: .leading, spacing: 10) {
            SectionLabel("POLICY")
            SandboxTextField("Name", text: draftBinding(\.name))
            SandboxTextField("File read allow paths", text: draftBinding(\.allowRead), lines: 2)
            SandboxTextField("File write allow paths", text: draftBinding(\.allowWrite), lines: 3)
            SandboxTextField("Executable commands", text: draftBinding(\.allowCommands), lines: 2)
            SandboxTextField("Denied commands", text: draftBinding(\.denyCommands), lines: 2)
            SandboxTextField("Allowed network hosts", text: draftBinding(\.allowNetwork), lines: 2)
            SandboxTextField("Denied network hosts", text: draftBinding(\.denyNetwork), lines: 2)
            SandboxTextField("Environment scope", text: draftBinding(\.allowEnv), lines: 2)

            HStack(spacing: 8) {
                SandboxTextField("Temp writes", text: draftBinding(\.tmpWrite), lines: 2)
                SandboxTextField("Cache writes", text: draftBinding(\.cacheWrite), lines: 2)
            }
        }
    }

    private var assignmentSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            SectionLabel("NODE ASSIGNMENT")
            HStack(spacing: 8) {
                VStack(alignment: .leading, spacing: 2) {
                    Text(store.selectedNode?.title ?? "No node selected")
                        .font(.system(size: 12, weight: .semibold))
                        .foregroundStyle(AppTheme.text)
                        .lineLimit(1)
                    Text(store.selectedNodeSandboxName)
                        .font(.system(size: 10))
                        .foregroundStyle(AppTheme.textMuted)
                        .lineLimit(1)
                }
                Spacer()
                Button {
                    store.assignSelectedSandboxToSelectedNode()
                } label: {
                    Image(systemName: "link")
                        .font(.system(size: 12, weight: .semibold))
                        .foregroundStyle(AppTheme.text)
                        .frame(width: 30, height: 28)
                        .background(RoundedRectangle(cornerRadius: 7).fill(AppTheme.panelElev))
                }
                .buttonStyle(.plain)
                .disabled(store.selectedNode == nil || store.selectedSandboxID == nil)
            }
            .padding(10)
            .background(RoundedRectangle(cornerRadius: 8).fill(AppTheme.canvas.opacity(0.65)))
            .overlay {
                RoundedRectangle(cornerRadius: 8).stroke(AppTheme.stroke, lineWidth: 1)
            }
        }
    }

    private var probeSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            SectionLabel("POLICY PROBE")

            HStack(spacing: 8) {
                Menu {
                    ForEach(["read_file", "write_file", "execute", "network", "read_env"], id: \.self) { operation in
                        Button(operation) {
                            store.updateSandboxProbe(operation: operation)
                        }
                    }
                } label: {
                    Text(store.sandboxProbeOperation)
                        .font(.system(size: 11, weight: .semibold, design: .monospaced))
                        .foregroundStyle(AppTheme.text)
                        .frame(width: 92, height: 28)
                        .background(RoundedRectangle(cornerRadius: 7).fill(AppTheme.panelElev))
                }
                .buttonStyle(.plain)

                TextField(
                    "target",
                    text: Binding(
                        get: { store.sandboxProbeTarget },
                        set: { store.updateSandboxProbe(target: $0) }
                    )
                )
                .font(.system(size: 11, design: .monospaced))
                .textFieldStyle(.plain)
                .foregroundStyle(AppTheme.text)
                .padding(.horizontal, 9)
                .frame(height: 28)
                .background(RoundedRectangle(cornerRadius: 7).fill(AppTheme.panelElev))

                Button {
                    store.evaluateSelectedSandbox()
                } label: {
                    Image(systemName: "play.fill")
                        .font(.system(size: 10, weight: .bold))
                        .foregroundStyle(.white)
                        .frame(width: 28, height: 28)
                        .background(RoundedRectangle(cornerRadius: 7).fill(AppTheme.accent))
                }
                .buttonStyle(.plain)
            }

            if let decision = store.sandboxDecision {
                HStack(alignment: .top, spacing: 8) {
                    Image(systemName: decision.allowed ? "checkmark.circle.fill" : "xmark.octagon.fill")
                        .foregroundStyle(decision.allowed ? AppTheme.statusColor(.succeeded) : AppTheme.statusColor(.failed))
                    VStack(alignment: .leading, spacing: 3) {
                        Text(decision.allowed ? "Allowed without approval" : "Blocked and halted")
                            .font(.system(size: 12, weight: .semibold))
                            .foregroundStyle(AppTheme.text)
                        Text(decision.reason)
                            .font(.system(size: 11))
                            .foregroundStyle(AppTheme.textMuted)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                    Spacer()
                }
                .padding(10)
                .background(RoundedRectangle(cornerRadius: 8).fill(AppTheme.canvas.opacity(0.65)))
                .overlay {
                    RoundedRectangle(cornerRadius: 8).stroke(AppTheme.stroke, lineWidth: 1)
                }
            }
        }
    }

    private var historySection: some View {
        VStack(alignment: .leading, spacing: 8) {
            SectionLabel("HISTORY")

            if let sandbox = store.selectedSandbox, !sandbox.history.isEmpty {
                ForEach(sandbox.history.prefix(5)) { item in
                    HStack(spacing: 8) {
                        Image(systemName: item.hasDiff ? "clock.arrow.circlepath" : "sparkle")
                            .font(.system(size: 11, weight: .semibold))
                            .foregroundStyle(AppTheme.textMuted)
                            .frame(width: 16)
                        VStack(alignment: .leading, spacing: 2) {
                            Text(item.change)
                                .font(.system(size: 11, weight: .semibold))
                                .foregroundStyle(AppTheme.text)
                            Text(item.at)
                                .font(.system(size: 10))
                                .foregroundStyle(AppTheme.textMuted)
                        }
                        Spacer()
                    }
                }
            } else {
                Text("No sandbox history yet.")
                    .font(.system(size: 11))
                    .foregroundStyle(AppTheme.textFaint)
            }
        }
    }

    private func draftBinding(_ keyPath: WritableKeyPath<SandboxPolicyDraft, String>) -> Binding<String> {
        Binding(
            get: { store.sandboxDraft[keyPath: keyPath] },
            set: { value in
                store.updateSandboxDraft { draft in
                    draft[keyPath: keyPath] = value
                }
            }
        )
    }
}

private struct SectionLabel: View {
    let title: String

    init(_ title: String) {
        self.title = title
    }

    var body: some View {
        Text(title)
            .font(.system(size: 10, weight: .bold))
            .tracking(0.7)
            .foregroundStyle(AppTheme.textFaint)
    }
}

private struct SandboxTextField: View {
    let label: String
    @Binding var text: String
    var lines: Int

    init(_ label: String, text: Binding<String>, lines: Int = 1) {
        self.label = label
        self._text = text
        self.lines = lines
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            Text(label)
                .font(.system(size: 10, weight: .semibold))
                .foregroundStyle(AppTheme.textMuted)
            TextField(label, text: $text, axis: .vertical)
                .font(.system(size: 11, design: .monospaced))
                .textFieldStyle(.plain)
                .foregroundStyle(AppTheme.text)
                .lineLimit(lines, reservesSpace: true)
                .padding(9)
                .background(RoundedRectangle(cornerRadius: 7).fill(AppTheme.panelElev))
                .overlay {
                    RoundedRectangle(cornerRadius: 7).stroke(AppTheme.stroke, lineWidth: 1)
                }
        }
    }
}

private struct SandboxActionButton: View {
    let symbol: String
    let label: String
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 5) {
                Image(systemName: symbol)
                    .font(.system(size: 10, weight: .semibold))
                Text(label)
                    .font(.system(size: 11, weight: .semibold))
            }
            .foregroundStyle(AppTheme.text)
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
