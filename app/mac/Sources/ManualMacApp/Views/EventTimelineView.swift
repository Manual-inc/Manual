import SwiftUI

struct EventTimelineView: View {
    let events: [WorkflowEventModel]

    var body: some View {
        if events.isEmpty {
            emptyState
        } else {
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(spacing: 0) {
                        ForEach(events) { event in
                            EventRow(event: event)
                                .id(event.id)
                            Rectangle()
                                .fill(AppTheme.stroke)
                                .frame(height: 1)
                        }
                    }
                }
                .onChange(of: events.count) {
                    if let last = events.last {
                        withAnimation(.easeOut(duration: 0.2)) {
                            proxy.scrollTo(last.id, anchor: .bottom)
                        }
                    }
                }
            }
        }
    }

    private var emptyState: some View {
        VStack(spacing: 8) {
            Image(systemName: "waveform.path")
                .font(.system(size: 26))
                .foregroundStyle(AppTheme.textFaint)
            Text("No execution events yet")
                .font(.system(size: 13, weight: .semibold))
                .foregroundStyle(AppTheme.textMuted)
            Text("Hit Execute workflow to stream realtime events here.")
                .font(.system(size: 11))
                .foregroundStyle(AppTheme.textFaint)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

private struct EventRow: View {
    let event: WorkflowEventModel

    var body: some View {
        HStack(alignment: .center, spacing: 12) {
            Text(event.time, style: .time)
                .font(.system(size: 11, design: .monospaced))
                .foregroundStyle(AppTheme.textFaint)
                .frame(width: 78, alignment: .leading)

            Image(systemName: symbolName)
                .font(.system(size: 12, weight: .semibold))
                .foregroundStyle(symbolColor)
                .frame(width: 16)

            Text(event.title)
                .font(.system(size: 12, weight: .semibold))
                .foregroundStyle(AppTheme.text)
                .frame(width: 140, alignment: .leading)

            Text(event.detail)
                .font(.system(size: 12, design: .monospaced))
                .foregroundStyle(AppTheme.textMuted)
                .lineLimit(1)
                .truncationMode(.tail)
                .frame(maxWidth: .infinity, alignment: .leading)
                .textSelection(.enabled)
        }
        .padding(.horizontal, 18)
        .padding(.vertical, 9)
        .frame(maxWidth: .infinity)
    }

    private var symbolName: String {
        if event.nodeID == nil {
            return event.title.contains("completed") ? "flag.checkered" : "bolt.fill"
        }
        return event.title.contains("completed") ? "checkmark.circle.fill" : "play.circle.fill"
    }

    private var symbolColor: Color {
        if event.nodeID == nil {
            return AppTheme.accent
        }
        if event.title.contains("completed") {
            return Color(red: 0.42, green: 0.85, blue: 0.50)
        }
        return Color(red: 0.40, green: 0.65, blue: 1.00)
    }
}
