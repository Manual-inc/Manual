import SwiftUI

struct NodeInputOverrideSheet: View {
    let nodeID: String
    let nodeTitle: String
    let onRun: ([String: Any]) -> Void
    let onCancel: () -> Void

    @State private var jsonText: String = "{}"
    @State private var parseError: String? = nil

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            // Header
            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text("Override Input")
                        .font(.system(size: 15, weight: .semibold))
                        .foregroundStyle(AppTheme.text)
                    Text(nodeTitle)
                        .font(.system(size: 12))
                        .foregroundStyle(AppTheme.textMuted)
                }
                Spacer()
                Button(action: onCancel) {
                    Image(systemName: "xmark")
                        .font(.system(size: 11, weight: .semibold))
                        .foregroundStyle(AppTheme.textMuted)
                        .frame(width: 24, height: 24)
                        .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
            }
            .padding(.horizontal, 20)
            .padding(.top, 20)
            .padding(.bottom, 16)

            Divider()

            // JSON editor
            VStack(alignment: .leading, spacing: 8) {
                Text("INPUT JSON")
                    .font(.system(size: 10, weight: .bold))
                    .tracking(0.7)
                    .foregroundStyle(AppTheme.textFaint)
                    .padding(.horizontal, 20)
                    .padding(.top, 16)

                TextEditor(text: $jsonText)
                    .font(.system(size: 12, design: .monospaced))
                    .foregroundStyle(AppTheme.text)
                    .scrollContentBackground(.hidden)
                    .background(AppTheme.canvas)
                    .frame(height: 180)
                    .overlay {
                        RoundedRectangle(cornerRadius: 8)
                            .stroke(parseError != nil ? Color.red.opacity(0.6) : AppTheme.stroke, lineWidth: 1)
                    }
                    .padding(.horizontal, 20)
                    .onChange(of: jsonText) { _, _ in parseError = nil }

                if let parseError {
                    Text(parseError)
                        .font(.system(size: 11))
                        .foregroundStyle(Color.red)
                        .padding(.horizontal, 20)
                }
            }

            Spacer(minLength: 16)

            // Actions
            HStack(spacing: 10) {
                Spacer()
                Button("Cancel", action: onCancel)
                    .buttonStyle(.plain)
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(AppTheme.textMuted)
                    .padding(.horizontal, 14)
                    .padding(.vertical, 7)
                    .background(Capsule().fill(AppTheme.panelElev))
                    .overlay { Capsule().stroke(AppTheme.stroke, lineWidth: 1) }

                Button("Run with Overrides") {
                    runWithOverrides()
                }
                .buttonStyle(.plain)
                .font(.system(size: 12, weight: .semibold))
                .foregroundStyle(.white)
                .padding(.horizontal, 16)
                .padding(.vertical, 8)
                .background(
                    Capsule().fill(AppTheme.accent)
                )
            }
            .padding(.horizontal, 20)
            .padding(.bottom, 20)
        }
        .frame(width: 420)
        .background(AppTheme.panel)
    }

    private func runWithOverrides() {
        let trimmed = jsonText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let data = trimmed.data(using: .utf8),
              let parsed = try? JSONSerialization.jsonObject(with: data),
              let dict = parsed as? [String: Any]
        else {
            parseError = "Invalid JSON — must be an object { … }"
            return
        }
        onRun(dict)
    }
}
