import SwiftUI

// See docs/wiki/systems/매뉴얼-최적화-기능.md: optimization insight should
// be visible next to execution logs so users can act on the latest run.
struct OptimizationSummaryView: View {
    let report: WorkflowOptimizationReport?
    let analysis: WorkflowOptimizationAnalysis?
    let isLoading: Bool

    var body: some View {
        if isLoading {
            ProgressView("Loading optimization insight…")
                .progressViewStyle(.circular)
                .tint(AppTheme.accent)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .foregroundStyle(AppTheme.textMuted)
        } else if let report, let analysis {
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    HStack(spacing: 8) {
                        ForEach(report.sections, id: \.self) { section in
                            Text(section)
                                .font(.system(size: 10, weight: .bold))
                                .foregroundStyle(AppTheme.text)
                                .padding(.horizontal, 10)
                                .padding(.vertical, 6)
                                .background(Capsule().fill(AppTheme.panelElev))
                                .overlay {
                                    Capsule().stroke(AppTheme.stroke, lineWidth: 1)
                                }
                        }
                    }

                    OptimizationCard(title: "Main Issue", accent: AppTheme.accent) {
                        Text(report.mainIssue)
                            .font(.system(size: 16, weight: .semibold))
                            .foregroundStyle(AppTheme.text)
                            .fixedSize(horizontal: false, vertical: true)
                    }

                    OptimizationCard(title: "Measurements", accent: AppTheme.textFaint) {
                        VStack(alignment: .leading, spacing: 4) {
                            Text(report.measurementMode.capitalized)
                                .font(.system(size: 12, weight: .semibold))
                                .foregroundStyle(AppTheme.text)
                            Text(report.measurementNote)
                                .font(.system(size: 11))
                                .foregroundStyle(AppTheme.textMuted)
                                .fixedSize(horizontal: false, vertical: true)
                        }
                    }

                    if analysis.regressionPossible {
                        OptimizationCard(title: "Regression Risk", accent: AppTheme.statusColor(.failed)) {
                            VStack(alignment: .leading, spacing: 4) {
                                Text(humanizeIdentifier(analysis.regressionStepID ?? "workflow"))
                                    .font(.system(size: 12, weight: .semibold))
                                    .foregroundStyle(AppTheme.text)
                                    .textSelection(.enabled)
                                Text(analysis.regressionReason)
                                    .font(.system(size: 11))
                                    .foregroundStyle(AppTheme.textMuted)
                                    .fixedSize(horizontal: false, vertical: true)
                            }
                        }
                    }

                    OptimizationCard(title: "Recommendations", accent: AppTheme.statusColor(.succeeded)) {
                        VStack(alignment: .leading, spacing: 8) {
                            ForEach(report.recommendations, id: \.self) { recommendation in
                                HStack(alignment: .top, spacing: 8) {
                                    Image(systemName: "arrow.up.right.circle.fill")
                                        .font(.system(size: 12))
                                        .foregroundStyle(AppTheme.accent)
                                    Text(recommendation)
                                        .font(.system(size: 12))
                                        .foregroundStyle(AppTheme.text)
                                        .fixedSize(horizontal: false, vertical: true)
                                }
                            }
                        }
                    }

                    HStack(alignment: .top, spacing: 12) {
                        OptimizationListCard(title: "Token Waste", values: analysis.tokenWasteSteps.map(humanizeIdentifier))
                        OptimizationListCard(title: "Verification Gaps", values: analysis.verificationGapSteps.map(humanizeIdentifier))
                        OptimizationListCard(title: "Slow Steps", values: analysis.slowSteps.map(humanizeIdentifier))
                    }
                }
                .padding(16)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .background(AppTheme.panel)
        } else {
            VStack(spacing: 8) {
                Image(systemName: "chart.bar.doc.horizontal")
                    .font(.system(size: 26))
                    .foregroundStyle(AppTheme.textFaint)
                Text("No optimization insight yet")
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(AppTheme.textMuted)
                Text("Execute the workflow to capture optimization evidence and surface the latest bottlenecks here.")
                    .font(.system(size: 11))
                    .foregroundStyle(AppTheme.textFaint)
                    .multilineTextAlignment(.center)
                    .frame(maxWidth: 280)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background(AppTheme.panel)
        }
    }
}

private struct OptimizationCard<Content: View>: View {
    let title: String
    let accent: Color
    @ViewBuilder var content: () -> Content

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 8) {
                Circle()
                    .fill(accent)
                    .frame(width: 8, height: 8)
                Text(title)
                    .font(.system(size: 11, weight: .bold))
                    .foregroundStyle(AppTheme.textMuted)
            }

            content()
        }
        .padding(14)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(RoundedRectangle(cornerRadius: 10).fill(AppTheme.panelElev))
        .overlay {
            RoundedRectangle(cornerRadius: 10).stroke(AppTheme.stroke, lineWidth: 1)
        }
    }
}

private struct OptimizationListCard: View {
    let title: String
    let values: [String]

    var body: some View {
        OptimizationCard(title: title, accent: AppTheme.accentMuted) {
            VStack(alignment: .leading, spacing: 6) {
                if values.isEmpty {
                    Text("No strong signal")
                        .font(.system(size: 11))
                        .foregroundStyle(AppTheme.textFaint)
                } else {
                    ForEach(values, id: \.self) { value in
                        Text(value)
                            .font(.system(size: 11, weight: .semibold))
                            .foregroundStyle(AppTheme.text)
                    }
                }
            }
        }
    }
}
