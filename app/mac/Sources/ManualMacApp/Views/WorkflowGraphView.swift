import SwiftUI

struct WorkflowGraphView: View {
    let nodes: [WorkflowNodeModel]
    let edges: [WorkflowEdgeModel]
    let selectedNodeID: String?
    let onSelect: (String) -> Void

    private let nodeSize = CGSize(width: 180, height: 88)

    var body: some View {
        GeometryReader { proxy in
            ZStack {
                Canvas { context, size in
                    for edge in edges {
                        guard
                            let from = nodes.first(where: { $0.id == edge.from }),
                            let to = nodes.first(where: { $0.id == edge.to })
                        else {
                            continue
                        }

                        let start = point(for: from, in: size)
                        let end = point(for: to, in: size)
                        var path = Path()
                        path.move(to: CGPoint(x: start.x + nodeSize.width / 2, y: start.y))
                        path.addCurve(
                            to: CGPoint(x: end.x - nodeSize.width / 2, y: end.y),
                            control1: CGPoint(x: start.x + 110, y: start.y),
                            control2: CGPoint(x: end.x - 110, y: end.y)
                        )

                        let active = from.status == .succeeded || to.status == .running || to.status == .succeeded
                        context.stroke(
                            path,
                            with: .color(active ? .blue : Color(nsColor: .separatorColor)),
                            lineWidth: active ? 2.5 : 1.5
                        )
                    }
                }

                ForEach(nodes) { node in
                    WorkflowNodeCard(node: node, isSelected: selectedNodeID == node.id)
                        .frame(width: nodeSize.width, height: nodeSize.height)
                        .position(point(for: node, in: proxy.size))
                        .onTapGesture {
                            onSelect(node.id)
                        }
                }
            }
            .background(.background)
            .clipShape(RoundedRectangle(cornerRadius: 8))
            .overlay {
                RoundedRectangle(cornerRadius: 8)
                    .stroke(Color(nsColor: .separatorColor), lineWidth: 1)
            }
        }
    }

    private func point(for node: WorkflowNodeModel, in size: CGSize) -> CGPoint {
        CGPoint(
            x: max(nodeSize.width / 2 + 16, min(size.width - nodeSize.width / 2 - 16, size.width * node.position.x)),
            y: max(nodeSize.height / 2 + 16, min(size.height - nodeSize.height / 2 - 16, size.height * node.position.y))
        )
    }
}

private struct WorkflowNodeCard: View {
    let node: WorkflowNodeModel
    let isSelected: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 8) {
                Image(systemName: node.status.symbolName)
                    .foregroundStyle(statusColor)

                Text(node.kind.rawValue)
                    .font(.caption.weight(.medium))
                    .foregroundStyle(.secondary)

                Spacer()
            }

            Text(node.title)
                .font(.headline)
                .lineLimit(1)

            Text(node.result ?? node.subtitle)
                .font(.caption)
                .foregroundStyle(.secondary)
                .lineLimit(2)
        }
        .padding(12)
        .background(.regularMaterial)
        .clipShape(RoundedRectangle(cornerRadius: 8))
        .overlay {
            RoundedRectangle(cornerRadius: 8)
                .stroke(isSelected ? Color.accentColor : Color(nsColor: .separatorColor), lineWidth: isSelected ? 2 : 1)
        }
        .shadow(color: .black.opacity(isSelected ? 0.12 : 0.06), radius: isSelected ? 10 : 4, y: 2)
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
