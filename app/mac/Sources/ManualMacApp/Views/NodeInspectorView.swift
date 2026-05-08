import SwiftUI

struct NodeInspectorView: View {
    let node: WorkflowNodeModel?

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            if let node {
                HStack {
                    Label(node.title, systemImage: node.status.symbolName)
                        .font(.headline)

                    Spacer()

                    Text(node.status.rawValue)
                        .font(.caption.weight(.medium))
                        .padding(.horizontal, 8)
                        .padding(.vertical, 4)
                        .background(statusColor(for: node).opacity(0.14))
                        .foregroundStyle(statusColor(for: node))
                        .clipShape(Capsule())
                }

                Text(node.subtitle)
                    .foregroundStyle(.secondary)

                Text(node.result ?? "Waiting for this node to run.")
                    .font(.callout)
                    .lineLimit(3)
            } else {
                Text("No node selected")
                    .font(.headline)
                Text("Select a node in the graph or sidebar.")
                    .foregroundStyle(.secondary)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .padding(14)
        .background(.regularMaterial)
        .clipShape(RoundedRectangle(cornerRadius: 8))
        .overlay {
            RoundedRectangle(cornerRadius: 8)
                .stroke(Color(nsColor: .separatorColor), lineWidth: 1)
        }
    }

    private func statusColor(for node: WorkflowNodeModel) -> Color {
        switch node.status {
        case .idle: .secondary
        case .running: .blue
        case .succeeded: .green
        case .failed: .red
        }
    }
}
