import SwiftUI

struct EventTimelineView: View {
    let events: [WorkflowEventModel]

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text("Run Events")
                    .font(.headline)

                Spacer()

                Text("\(events.count)")
                    .font(.caption.weight(.medium))
                    .foregroundStyle(.secondary)
            }

            if events.isEmpty {
                ContentUnavailableView(
                    "No events yet",
                    systemImage: "waveform.path.ecg",
                    description: Text("Press Start to watch this workflow run.")
                )
                .frame(maxHeight: .infinity)
            } else {
                ScrollViewReader { proxy in
                    ScrollView {
                        LazyVStack(alignment: .leading, spacing: 10) {
                            ForEach(events) { event in
                                EventRow(event: event)
                                    .id(event.id)
                            }
                        }
                        .padding(.vertical, 2)
                    }
                    .onChange(of: events.count) {
                        if let last = events.last {
                            proxy.scrollTo(last.id, anchor: .bottom)
                        }
                    }
                }
            }
        }
        .padding(14)
        .background(.regularMaterial)
        .clipShape(RoundedRectangle(cornerRadius: 8))
        .overlay {
            RoundedRectangle(cornerRadius: 8)
                .stroke(Color(nsColor: .separatorColor), lineWidth: 1)
        }
    }
}

private struct EventRow: View {
    let event: WorkflowEventModel

    var body: some View {
        HStack(alignment: .top, spacing: 10) {
            Image(systemName: symbolName)
                .foregroundStyle(event.nodeID == nil ? .purple : .blue)
                .frame(width: 18)

            VStack(alignment: .leading, spacing: 3) {
                HStack {
                    Text(event.title)
                        .font(.callout.weight(.medium))
                        .lineLimit(1)

                    Spacer()

                    Text(event.time, style: .time)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }

                Text(event.detail)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(3)
            }
        }
        .padding(10)
        .background(.background.opacity(0.65))
        .clipShape(RoundedRectangle(cornerRadius: 8))
    }

    private var symbolName: String {
        if event.nodeID == nil {
            return event.title.contains("completed") ? "flag.checkered" : "bolt.fill"
        }

        return event.title.contains("completed") ? "checkmark.circle.fill" : "play.circle.fill"
    }
}
