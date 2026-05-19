import SwiftUI

// See docs/wiki/systems/매뉴얼-최적화-기능.md: optimization feedback should
// stay visible in the main workflow chrome, not only inside the bottom panel.
struct OptimizationPulseView: View {
    let headline: WorkflowOptimizationHeadline
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 10) {
                Image(systemName: headline.isRegression ? "exclamationmark.triangle.fill" : "waveform.path.ecg")
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(headline.isRegression ? AppTheme.statusColor(.failed) : AppTheme.accent)

                VStack(alignment: .leading, spacing: 1) {
                    Text(headline.title)
                        .font(.system(size: 11, weight: .semibold))
                        .foregroundStyle(AppTheme.text)
                        .lineLimit(1)
                    Text(headline.detail)
                        .font(.system(size: 10))
                        .foregroundStyle(AppTheme.textMuted)
                        .lineLimit(1)
                }

                Text(headline.measurementLabel)
                    .font(.system(size: 10, weight: .bold))
                    .foregroundStyle(headline.isRegression ? AppTheme.statusColor(.failed) : AppTheme.textMuted)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 4)
                    .background(Capsule().fill(AppTheme.panel))
                    .overlay {
                        Capsule().stroke(AppTheme.stroke, lineWidth: 1)
                    }
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 8)
            .background(RoundedRectangle(cornerRadius: 10).fill(AppTheme.panelElev))
            .overlay {
                RoundedRectangle(cornerRadius: 10).stroke(AppTheme.stroke, lineWidth: 1)
            }
        }
        .buttonStyle(.plain)
    }
}
