import SwiftUI

struct WorkflowGraphView: View {
    let nodes: [WorkflowNodeModel]
    let edges: [WorkflowEdgeModel]
    let selectedNodeID: String?
    let onSelect: (String) -> Void

    @State private var zoom: CGFloat = 1.0
    @State private var startZoom: CGFloat = 1.0
    @State private var pan: CGSize = .zero
    @State private var dragStart: CGSize = .zero
    @State private var phase: CGFloat = 0

    private let nodeSize = CGSize(width: 230, height: 96)

    var body: some View {
        GeometryReader { proxy in
            let canvasSize = CGSize(
                width: max(1200, proxy.size.width * 1.4),
                height: max(700, proxy.size.height * 1.4)
            )

            ZStack {
                AppTheme.canvas

                ZStack {
                    DotGridBackground()
                    edgeLayer(canvasSize: canvasSize)
                    nodeLayer(canvasSize: canvasSize)
                }
                .frame(width: canvasSize.width, height: canvasSize.height)
                .scaleEffect(zoom, anchor: .center)
                .offset(pan)
                .contentShape(Rectangle())

                VStack {
                    HStack {
                        Spacer()
                        zoomControls
                            .padding(14)
                    }
                    Spacer()
                }
            }
            .clipped()
            .gesture(
                DragGesture(minimumDistance: 6)
                    .onChanged { value in
                        pan = CGSize(
                            width: dragStart.width + value.translation.width,
                            height: dragStart.height + value.translation.height
                        )
                    }
                    .onEnded { _ in
                        dragStart = pan
                    }
            )
            .simultaneousGesture(
                MagnifyGesture()
                    .onChanged { value in
                        zoom = clampZoom(startZoom * value.magnification)
                    }
                    .onEnded { _ in
                        startZoom = zoom
                    }
            )
            .onAppear {
                withAnimation(.linear(duration: 1.2).repeatForever(autoreverses: false)) {
                    phase = -16
                }
            }
        }
    }

    private func clampZoom(_ value: CGFloat) -> CGFloat {
        max(0.4, min(2.0, value))
    }

    private func nodePoint(for node: WorkflowNodeModel, in size: CGSize) -> CGPoint {
        CGPoint(
            x: size.width * node.position.x,
            y: size.height * node.position.y
        )
    }

    // MARK: edges

    private func edgeLayer(canvasSize: CGSize) -> some View {
        Canvas { ctx, _ in
            for edge in edges {
                guard
                    let from = nodes.first(where: { $0.id == edge.from }),
                    let to   = nodes.first(where: { $0.id == edge.to })
                else { continue }

                let fromCenter = nodePoint(for: from, in: canvasSize)
                let toCenter   = nodePoint(for: to, in: canvasSize)

                let start = CGPoint(x: fromCenter.x + nodeSize.width / 2, y: fromCenter.y)
                let end   = CGPoint(x: toCenter.x   - nodeSize.width / 2, y: toCenter.y)

                let dx = max(60, abs(end.x - start.x) * 0.45)
                let c1 = CGPoint(x: start.x + dx, y: start.y)
                let c2 = CGPoint(x: end.x - dx, y: end.y)

                var path = Path()
                path.move(to: start)
                path.addCurve(to: end, control1: c1, control2: c2)

                let active = isActiveEdge(from: from, to: to)

                if active {
                    ctx.stroke(
                        path,
                        with: .color(AppTheme.edgeActive),
                        style: StrokeStyle(
                            lineWidth: 2.4,
                            lineCap: .round,
                            dash: [6, 7],
                            dashPhase: phase
                        )
                    )
                    let arrow = arrowHead(at: end, towards: c2, color: AppTheme.edgeActive)
                    ctx.fill(arrow.path, with: .color(arrow.color))
                } else {
                    ctx.stroke(
                        path,
                        with: .color(AppTheme.edge),
                        style: StrokeStyle(lineWidth: 1.6, lineCap: .round)
                    )
                    let arrow = arrowHead(at: end, towards: c2, color: AppTheme.edge)
                    ctx.fill(arrow.path, with: .color(arrow.color))
                }
            }
        }
    }

    private func isActiveEdge(from: WorkflowNodeModel, to: WorkflowNodeModel) -> Bool {
        if to.status == .running { return true }
        if from.status == .succeeded && (to.status == .running || to.status == .succeeded) {
            return true
        }
        return false
    }

    private func arrowHead(at point: CGPoint, towards control: CGPoint, color: Color) -> (path: Path, color: Color) {
        let angle = atan2(point.y - control.y, point.x - control.x)
        let size: CGFloat = 6
        var path = Path()
        path.move(to: point)
        path.addLine(to: CGPoint(
            x: point.x - size * cos(angle - .pi / 7),
            y: point.y - size * sin(angle - .pi / 7)
        ))
        path.addLine(to: CGPoint(
            x: point.x - size * cos(angle + .pi / 7),
            y: point.y - size * sin(angle + .pi / 7)
        ))
        path.closeSubpath()
        return (path, color)
    }

    // MARK: nodes

    private func nodeLayer(canvasSize: CGSize) -> some View {
        ZStack {
            ForEach(nodes) { node in
                WorkflowNodeCard(
                    node: node,
                    isSelected: selectedNodeID == node.id,
                    size: nodeSize
                )
                .position(nodePoint(for: node, in: canvasSize))
                .onTapGesture { onSelect(node.id) }
            }
        }
    }

    // MARK: zoom controls

    private var zoomControls: some View {
        HStack(spacing: 0) {
            zoomButton(symbol: "plus") { zoom = clampZoom(zoom + 0.1); startZoom = zoom }
            divider
            zoomButton(symbol: "minus") { zoom = clampZoom(zoom - 0.1); startZoom = zoom }
            divider
            zoomButton(symbol: "viewfinder") {
                withAnimation(.spring(response: 0.4, dampingFraction: 0.85)) {
                    zoom = 1.0
                    pan = .zero
                    dragStart = .zero
                    startZoom = 1.0
                }
            }
        }
        .background(AppTheme.panelElev)
        .clipShape(RoundedRectangle(cornerRadius: 9))
        .overlay {
            RoundedRectangle(cornerRadius: 9).stroke(AppTheme.stroke, lineWidth: 1)
        }
        .shadow(color: .black.opacity(0.30), radius: 8, y: 2)
    }

    private var divider: some View {
        Rectangle()
            .fill(AppTheme.stroke)
            .frame(width: 1, height: 18)
    }

    private func zoomButton(symbol: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Image(systemName: symbol)
                .font(.system(size: 11, weight: .semibold))
                .foregroundStyle(AppTheme.text)
                .frame(width: 32, height: 30)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }
}

// MARK: - Dot grid background

private struct DotGridBackground: View {
    var body: some View {
        Canvas { ctx, size in
            let spacing: CGFloat = 22
            let radius: CGFloat = 0.9
            var x: CGFloat = spacing
            while x < size.width {
                var y: CGFloat = spacing
                while y < size.height {
                    let rect = CGRect(
                        x: x - radius,
                        y: y - radius,
                        width: radius * 2,
                        height: radius * 2
                    )
                    ctx.fill(Path(ellipseIn: rect), with: .color(AppTheme.canvasGrid))
                    y += spacing
                }
                x += spacing
            }
        }
    }
}

// MARK: - Node card

private struct WorkflowNodeCard: View {
    let node: WorkflowNodeModel
    let isSelected: Bool
    let size: CGSize

    var body: some View {
        ZStack {
            HStack(alignment: .center, spacing: 12) {
                ZStack {
                    RoundedRectangle(cornerRadius: 11)
                        .fill(AppTheme.kindColor(node.kind).opacity(0.18))
                    RoundedRectangle(cornerRadius: 11)
                        .stroke(AppTheme.kindColor(node.kind).opacity(0.55), lineWidth: 1)
                    NodeKindIcon(kind: node.kind, symbolSize: 18)
                }
                .frame(width: 46, height: 46)

                VStack(alignment: .leading, spacing: 4) {
                    Text(node.title)
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(AppTheme.text)
                        .lineLimit(1)

                    Text(node.kind.rawValue.uppercased())
                        .font(.system(size: 9, weight: .bold))
                        .tracking(0.7)
                        .foregroundStyle(AppTheme.textMuted)
                }

                Spacer(minLength: 0)

                StatusBadge(status: node.status)
            }
            .padding(.horizontal, 14)
            .padding(.vertical, 10)
            .frame(width: size.width, height: size.height)
            .background(
                RoundedRectangle(cornerRadius: 12)
                    .fill(AppTheme.nodeCard)
            )
            .overlay {
                RoundedRectangle(cornerRadius: 12)
                    .stroke(
                        isSelected ? AppTheme.accent : AppTheme.nodeStroke,
                        lineWidth: isSelected ? 2 : 1
                    )
            }
            .shadow(
                color: .black.opacity(isSelected ? 0.45 : 0.28),
                radius: isSelected ? 14 : 8,
                y: 4
            )
            .overlay {
                if isSelected {
                    RoundedRectangle(cornerRadius: 12)
                        .stroke(AppTheme.accent.opacity(0.25), lineWidth: 6)
                        .blur(radius: 4)
                }
            }

            HStack {
                Port(color: AppTheme.kindColor(node.kind), filled: true)
                    .offset(x: -6)
                Spacer()
                Port(color: AppTheme.kindColor(node.kind), filled: false)
                    .offset(x: 6)
            }
            .frame(width: size.width)
        }
        .frame(width: size.width, height: size.height)
        .animation(.easeInOut(duration: 0.15), value: isSelected)
        .animation(.easeInOut(duration: 0.2), value: node.status)
    }
}

private struct StatusBadge: View {
    let status: WorkflowNodeStatus

    var body: some View {
        Group {
            switch status {
            case .idle:
                Circle()
                    .stroke(AppTheme.textFaint, lineWidth: 1)
                    .frame(width: 8, height: 8)
            case .running:
                ZStack {
                    Circle()
                        .fill(AppTheme.statusColor(status).opacity(0.25))
                        .frame(width: 18, height: 18)
                    Circle()
                        .fill(AppTheme.statusColor(status))
                        .frame(width: 8, height: 8)
                }
            case .succeeded:
                Image(systemName: "checkmark.circle.fill")
                    .font(.system(size: 14))
                    .foregroundStyle(AppTheme.statusColor(status))
            case .failed:
                Image(systemName: "xmark.octagon.fill")
                    .font(.system(size: 14))
                    .foregroundStyle(AppTheme.statusColor(status))
            }
        }
    }
}

private struct Port: View {
    let color: Color
    let filled: Bool

    var body: some View {
        ZStack {
            Circle()
                .fill(filled ? color : AppTheme.canvas)
                .frame(width: 12, height: 12)
            Circle()
                .stroke(color.opacity(0.9), lineWidth: 1.4)
                .frame(width: 12, height: 12)
        }
    }
}
