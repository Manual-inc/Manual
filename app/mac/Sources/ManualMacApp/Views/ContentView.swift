import AppKit
import SwiftUI

public struct ContentView: View {
    @StateObject private var store = WorkflowRunStore()
    @SceneStorage("ManualMac.sidebarVisible") private var sidebarVisible = true
    @SceneStorage("ManualMac.sandboxPanelVisible") private var sandboxPanelVisible = true
    @SceneStorage("ManualMac.inspectorVisible") private var inspectorVisible = false
    @SceneStorage("ManualMac.bottomPanelVisible") private var bottomPanelVisible = false
    @SceneStorage("ManualMac.bottomPanelTab") private var bottomPanelTabRawValue = BottomPanelTab.events.rawValue
    @SceneStorage("ManualMac.lastStarterRepositoryPath") private var lastStarterRepositoryPath = ""
    @SceneStorage("ManualMac.recentStarterHistoryJSON") private var recentStarterHistoryJSON = "[]"

    public init() {}

    public var body: some View {
        ZStack {
            AppTheme.canvas.ignoresSafeArea()

            HStack(spacing: 0) {
                LeftRail(sidebarVisible: $sidebarVisible, sandboxPanelVisible: $sandboxPanelVisible)
                    .frame(width: 56)

                if sidebarVisible {
                    WorkflowSidebar(
                        store: store,
                        onCreateRecommendedStarter: {
                            guard let repositoryURL = pickWorkflowStarterRepository(message: "Pick a git repository and Manual will choose the best starter.") else { return }
                            if let repositoryRootPath = try? WorkflowStarterDefinition.resolveRepositoryRootPath(from: repositoryURL.path),
                               let recommendation = try? WorkflowStarterDefinition.recommendedPreset(repositoryRootPath: repositoryRootPath)
                            {
                                lastStarterRepositoryPath = repositoryRootPath
                                rememberRecentStarter(
                                    presetID: recommendation.preset.id,
                                    repositoryRootPath: repositoryRootPath,
                                    recommendationReason: recommendation.reason
                                )
                            } else {
                                lastStarterRepositoryPath = repositoryURL.path
                            }
                            presentOutputPanel()
                            store.createAndRunRecommendedStarter(selectedPath: repositoryURL.path)
                        },
                        onRerunRecommendedStarter: {
                            guard let repositoryRootPath = effectiveLastStarterRepositoryPath else { return }
                            lastStarterRepositoryPath = repositoryRootPath
                            presentOutputPanel()
                            store.createAndRunRecommendedStarter(selectedPath: repositoryRootPath)
                        },
                        lastStarterRepositoryPath: effectiveLastStarterRepositoryPath,
                        lastStarterRecommendation: effectiveLastStarterRecommendation,
                        recentStarters: mergedStarterHistory,
                        onRerunRecentStarter: { entry in
                            lastStarterRepositoryPath = entry.repositoryRootPath
                            presentOutputPanel()
                            store.createAndRunStarter(
                                presetID: entry.presetID,
                                selectedPath: entry.repositoryRootPath
                            )
                        },
                        onCreateStarter: { preset in
                            guard let repositoryURL = pickWorkflowStarterRepository(for: preset) else { return }
                            if let repositoryRootPath = try? WorkflowStarterDefinition.resolveRepositoryRootPath(from: repositoryURL.path) {
                                lastStarterRepositoryPath = repositoryRootPath
                                rememberRecentStarter(
                                    presetID: preset.id,
                                    repositoryRootPath: repositoryRootPath,
                                    recommendationReason: nil
                                )
                            } else {
                                lastStarterRepositoryPath = repositoryURL.path
                            }
                            presentOutputPanel()
                            store.createAndRunStarter(presetID: preset.id, selectedPath: repositoryURL.path)
                        }
                    )
                        .frame(width: 260)
                        .transition(.move(edge: .leading).combined(with: .opacity))
                }

                if sandboxPanelVisible {
                    SandboxPolicyPanel(store: store)
                        .frame(width: 360)
                        .transition(.move(edge: .leading).combined(with: .opacity))
                }

                VStack(spacing: 0) {
                    TopBar(
                        store: store,
                        sidebarVisible: $sidebarVisible,
                        inspectorVisible: $inspectorVisible,
                        bottomPanelVisible: $bottomPanelVisible,
                        showOptimization: presentOptimizationPanel
                    )
                        .frame(height: 54)

                    HStack(spacing: 0) {
                        ZStack(alignment: .bottom) {
                            WorkflowGraphView(
                                nodes: store.nodes,
                                edges: store.edges,
                                selectedNodeID: store.selectedNodeID,
                                onSelect: { nodeID in
                                    store.selectNode(nodeID)
                                    withAnimation(.easeInOut(duration: 0.18)) {
                                        inspectorVisible = true
                                    }
                                }
                            )
                            .frame(maxWidth: .infinity, maxHeight: .infinity)

                            VStack(spacing: 0) {
                                if bottomPanelVisible {
                                    BottomPanel(
                                        events: store.events,
                                        nodes: store.nodes,
                                        workflowID: store.selectedWorkflowID,
                                        runID: store.runID,
                                        rawWorkflowJSON: store.rawWorkflowJSON,
                                        optimizationReport: store.optimizationReport,
                                        optimizationAnalysis: store.optimizationAnalysis,
                                        optimizationLoading: store.optimizationLoading,
                                        selectedTab: selectedBottomPanelTabBinding,
                                        onClose: {
                                            withAnimation(.easeInOut(duration: 0.18)) {
                                                bottomPanelVisible = false
                                            }
                                        }
                                    )
                                    .frame(height: 220)
                                    .transition(.move(edge: .bottom).combined(with: .opacity))
                                }

                                BottomToggle(visible: $bottomPanelVisible, eventCount: store.events.count)
                                    .frame(height: 30)
                            }
                            .background(AppTheme.panel.opacity(0.98))
                            .shadow(color: .black.opacity(bottomPanelVisible ? 0.35 : 0), radius: 16, y: -4)
                        }
                        .frame(maxWidth: .infinity, maxHeight: .infinity)

                        if inspectorVisible {
                            Rectangle().fill(AppTheme.stroke).frame(width: 1)
                            NodeInspectorView(
                                store: store,
                                onClose: {
                                    withAnimation(.easeInOut(duration: 0.18)) {
                                        inspectorVisible = false
                                    }
                                }
                            )
                            .frame(width: 340)
                            .transition(.move(edge: .trailing).combined(with: .opacity))
                        }
                    }
                }
            }
        }
        .preferredColorScheme(.dark)
        .onAppear {
            store.bootstrap()
        }
        .onReceive(NotificationCenter.default.publisher(for: .startExampleWorkflow)) { _ in
            withAnimation(.easeInOut(duration: 0.18)) {
                bottomPanelVisible = true
            }
            store.start()
        }
    }

    private var selectedBottomPanelTab: BottomPanelTab {
        BottomPanelTab(rawValue: bottomPanelTabRawValue) ?? .events
    }

    private var selectedBottomPanelTabBinding: Binding<BottomPanelTab> {
        Binding(
            get: { selectedBottomPanelTab },
            set: { bottomPanelTabRawValue = $0.rawValue }
        )
    }

    private var recentStarterHistory: [WorkflowStarterRecentEntry] {
        WorkflowStarterDefinition.recentEntries(from: recentStarterHistoryJSON)
    }

    private var mergedStarterHistory: [WorkflowStarterRecentEntry] {
        WorkflowStarterDefinition.mergedRecentEntries(
            local: recentStarterHistory,
            shared: store.recentSharedStarters
        )
    }

    private var effectiveLastStarterRepositoryPath: String? {
        if !lastStarterRepositoryPath.isEmpty {
            return lastStarterRepositoryPath
        }
        return mergedStarterHistory.first?.repositoryRootPath
    }

    private var effectiveLastStarterRecommendation: WorkflowStarterRecommendationPreview? {
        // See docs/wiki/features/workflow-starters.md: the last-repository
        // rerun affordance should preview the next recommended starter first.
        guard let repositoryRootPath = effectiveLastStarterRepositoryPath else {
            return nil
        }
        return try? WorkflowStarterDefinition.recommendedStarterPreview(
            repositoryRootPath: repositoryRootPath
        )
    }

    private func presentOptimizationPanel() {
        var panelState = WorkflowPanelState(
            isBottomPanelVisible: bottomPanelVisible,
            selectedTab: selectedBottomPanelTab
        )
        panelState.showOptimization()
        withAnimation(.easeInOut(duration: 0.18)) {
            bottomPanelVisible = panelState.isBottomPanelVisible
            bottomPanelTabRawValue = panelState.selectedTab.rawValue
        }
    }

    private func pickWorkflowStarterRepository(for preset: WorkflowStarterPreset) -> URL? {
        pickWorkflowStarterRepository(message: "Pick a git repository for \(preset.title).")
    }

    private func pickWorkflowStarterRepository(message: String) -> URL? {
        // See docs/wiki/features/workflow-starters.md: mac onboarding should let
        // users pick a real repository before creating the starter workflow.
        let panel = NSOpenPanel()
        panel.title = "Choose a Repository"
        panel.message = message
        panel.prompt = "Use Repository"
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = false
        panel.canCreateDirectories = false
        return panel.runModal() == .OK ? panel.url : nil
    }

    private func presentOutputPanel() {
        var panelState = WorkflowPanelState(
            isBottomPanelVisible: bottomPanelVisible,
            selectedTab: selectedBottomPanelTab
        )
        panelState.showOutput()
        withAnimation(.easeInOut(duration: 0.18)) {
            bottomPanelVisible = panelState.isBottomPanelVisible
            bottomPanelTabRawValue = panelState.selectedTab.rawValue
        }
    }

    private func rememberRecentStarter(
        presetID: String,
        repositoryRootPath: String,
        recommendationReason: String?
    ) {
        let entry = WorkflowStarterRecentEntry(
            presetID: presetID,
            repositoryRootPath: repositoryRootPath,
            workflowID: WorkflowStarterDefinition.suggestedWorkflowID(
                repositoryRootPath: repositoryRootPath,
                presetID: presetID
            ),
            recommendationReason: recommendationReason,
            outcomeLabel: nil,
            outcomeText: nil
        )
        let updated = WorkflowStarterDefinition.updatedRecentEntries(recentStarterHistory, with: entry)
        recentStarterHistoryJSON = WorkflowStarterDefinition.encodeRecentEntries(updated)
    }
}

// MARK: - Left rail

private struct LeftRail: View {
    @Binding var sidebarVisible: Bool
    @Binding var sandboxPanelVisible: Bool

    var body: some View {
        VStack(spacing: 6) {
            BrandLogo()
                .padding(.top, 14)
                .padding(.bottom, 14)

            RailButton(symbol: "square.grid.2x2.fill", isActive: sidebarVisible) {
                withAnimation(.easeInOut(duration: 0.18)) {
                    sidebarVisible.toggle()
                }
            }
            RailButton(symbol: "lock.shield.fill", isActive: sandboxPanelVisible) {
                withAnimation(.easeInOut(duration: 0.18)) {
                    sandboxPanelVisible.toggle()
                }
            }
            /*
            RailButton(symbol: "clock.arrow.circlepath", isActive: false)
            RailButton(symbol: "key.fill", isActive: false)
            RailButton(symbol: "person.2.fill", isActive: false)
            */

            Spacer()

            /*
            RailButton(symbol: "questionmark.circle", isActive: false)
            RailButton(symbol: "gearshape", isActive: false)
                .padding(.bottom, 14)
            */
        }
        .frame(maxHeight: .infinity)
        .background(AppTheme.rail)
        .overlay(alignment: .trailing) {
            Rectangle().fill(AppTheme.stroke).frame(width: 1)
        }
    }
}

private struct BrandLogo: View {
    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 9)
                .fill(
                    LinearGradient(
                        colors: [AppTheme.accent, Color(red: 0.95, green: 0.30, blue: 0.50)],
                        startPoint: .topLeading,
                        endPoint: .bottomTrailing
                    )
                )
                .frame(width: 34, height: 34)
                .shadow(color: AppTheme.accent.opacity(0.4), radius: 8, y: 2)
            Text("M")
                .font(.system(size: 17, weight: .black, design: .rounded))
                .foregroundStyle(.white)
        }
    }
}

private struct WorkflowSidebar: View {
    @ObservedObject var store: WorkflowRunStore
    let onCreateRecommendedStarter: () -> Void
    let onRerunRecommendedStarter: () -> Void
    let lastStarterRepositoryPath: String?
    let lastStarterRecommendation: WorkflowStarterRecommendationPreview?
    let recentStarters: [WorkflowStarterRecentEntry]
    let onRerunRecentStarter: (WorkflowStarterRecentEntry) -> Void
    let onCreateStarter: (WorkflowStarterPreset) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 8) {
                Text("Workflows")
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(AppTheme.text)
                Spacer()
                Button {
                    store.refresh()
                } label: {
                    Image(systemName: "arrow.clockwise")
                        .font(.system(size: 11, weight: .semibold))
                        .foregroundStyle(AppTheme.textMuted)
                        .frame(width: 26, height: 24)
                }
                .buttonStyle(.plain)
                .disabled(store.isLoading)
            }
            .padding(.horizontal, 14)
            .frame(height: 44)
            .overlay(alignment: .bottom) {
                Rectangle().fill(AppTheme.stroke).frame(height: 1)
            }

            QuickStartCard(
                presets: WorkflowStarterDefinition.availablePresets,
                onCreateRecommendedStarter: onCreateRecommendedStarter,
                onRerunRecommendedStarter: onRerunRecommendedStarter,
                lastStarterRepositoryPath: lastStarterRepositoryPath,
                lastStarterRecommendation: lastStarterRecommendation,
                recentStarters: recentStarters,
                onRerunRecentStarter: onRerunRecentStarter,
                action: onCreateStarter
            )
                .padding(10)

            ScrollView {
                LazyVStack(spacing: 6) {
                    ForEach(store.workflows) { workflow in
                        WorkflowListRow(
                            workflow: workflow,
                            isSelected: workflow.workflowID == store.selectedWorkflowID
                        ) {
                            store.selectWorkflow(workflow.workflowID)
                        }
                    }
                }
                .padding(10)
            }

            VStack(alignment: .leading, spacing: 8) {
                Text(store.statusMessage)
                    .font(.system(size: 11))
                    .foregroundStyle(AppTheme.textMuted)
                    .lineLimit(2)
                    .frame(maxWidth: .infinity, alignment: .leading)

                HStack(spacing: 8) {
                    SmallActionButton(symbol: "square.and.arrow.down", label: "Save") {
                        store.saveSelectedWorkflow()
                    }
                    SmallActionButton(symbol: "trash", label: "Delete", isDestructive: true) {
                        store.deleteSelectedWorkflow()
                    }
                    .disabled(!store.hasSelectedWorkflow || store.isRunning)
                }
            }
            .padding(12)
            .overlay(alignment: .top) {
                Rectangle().fill(AppTheme.stroke).frame(height: 1)
            }
        }
        .background(AppTheme.panel)
        .overlay(alignment: .trailing) {
            Rectangle().fill(AppTheme.stroke).frame(width: 1)
        }
    }
}

private struct QuickStartCard: View {
    let presets: [WorkflowStarterPreset]
    let onCreateRecommendedStarter: () -> Void
    let onRerunRecommendedStarter: () -> Void
    let lastStarterRepositoryPath: String?
    let lastStarterRecommendation: WorkflowStarterRecommendationPreview?
    let recentStarters: [WorkflowStarterRecentEntry]
    let onRerunRecentStarter: (WorkflowStarterRecentEntry) -> Void
    let action: (WorkflowStarterPreset) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("Quick Start")
                .font(.system(size: 11, weight: .semibold))
                .foregroundStyle(AppTheme.textMuted)
            Text("Choose a starter workflow for a real git repository.")
                .font(.system(size: 12, weight: .semibold))
                .foregroundStyle(AppTheme.text)
                .fixedSize(horizontal: false, vertical: true)
            Text("Manual will build a runnable starter workflow, launch it, and stream events plus optimization into the bottom panel.")
                .font(.system(size: 11))
                .foregroundStyle(AppTheme.textMuted)
                .fixedSize(horizontal: false, vertical: true)

            if let lastStarterRepositoryPath {
                VStack(alignment: .leading, spacing: 6) {
                    Text("Last repository")
                        .font(.system(size: 10, weight: .bold))
                        .foregroundStyle(AppTheme.textFaint)
                    Text(WorkflowStarterDefinition.repositoryDisplayName(repositoryRootPath: lastStarterRepositoryPath))
                        .font(.system(size: 11, weight: .semibold))
                        .foregroundStyle(AppTheme.text)
                        .lineLimit(1)

                    if let lastStarterRecommendation {
                        Text("Recommended now: \(lastStarterRecommendation.preset.title)")
                            .font(.system(size: 10, weight: .semibold))
                            .foregroundStyle(AppTheme.text)
                            .fixedSize(horizontal: false, vertical: true)
                        Text("Why: \(lastStarterRecommendation.reason)")
                            .font(.system(size: 10))
                            .foregroundStyle(AppTheme.textMuted)
                            .fixedSize(horizontal: false, vertical: true)
                        Text(lastStarterRecommendation.changedFilesHint)
                            .font(.system(size: 10))
                            .foregroundStyle(AppTheme.textMuted)
                            .fixedSize(horizontal: false, vertical: true)
                        Text(lastStarterRecommendation.expectedOutcome)
                            .font(.system(size: 10))
                            .foregroundStyle(AppTheme.textFaint)
                            .fixedSize(horizontal: false, vertical: true)
                    }

                    Button(action: onRerunRecommendedStarter) {
                        HStack(spacing: 6) {
                            Image(systemName: "arrow.clockwise")
                                .font(.system(size: 10, weight: .bold))
                            Text("Run Recommended Again")
                                .font(.system(size: 11, weight: .semibold))
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .foregroundStyle(AppTheme.text)
                        .padding(.horizontal, 12)
                        .padding(.vertical, 9)
                        .background(
                            RoundedRectangle(cornerRadius: 9)
                                .fill(AppTheme.panel)
                        )
                        .overlay {
                            RoundedRectangle(cornerRadius: 9)
                                .stroke(AppTheme.strokeStrong, lineWidth: 1)
                        }
                    }
                    .buttonStyle(.plain)
                    .accessibilityIdentifier("rerun-recommended-starter-button")
                }
            }

            if !recentStarters.isEmpty {
                VStack(alignment: .leading, spacing: 6) {
                    Text("Recent starters")
                        .font(.system(size: 10, weight: .bold))
                        .foregroundStyle(AppTheme.textFaint)

                    ForEach(recentStarters.prefix(3), id: \.workflowID) { entry in
                        VStack(alignment: .leading, spacing: 6) {
                            Button(action: { onRerunRecentStarter(entry) }) {
                                VStack(alignment: .leading, spacing: 4) {
                                    Text("\(title(for: entry.presetID)) · \(WorkflowStarterDefinition.repositoryDisplayName(repositoryRootPath: entry.repositoryRootPath))")
                                        .font(.system(size: 11, weight: .semibold))
                                    if let recommendationReason = entry.recommendationReason {
                                        Text("Why it fit: \(recommendationReason)")
                                            .font(.system(size: 10))
                                            .foregroundStyle(AppTheme.textMuted)
                                            .fixedSize(horizontal: false, vertical: true)
                                    }
                                    Text(expectedOutcome(for: entry.presetID))
                                        .font(.system(size: 10))
                                        .foregroundStyle(AppTheme.textFaint)
                                        .fixedSize(horizontal: false, vertical: true)
                                    if let outcomeText = entry.outcomeText {
                                        Text("Last result: \(starterOutcomePreviewText(outcomeText))")
                                            .font(.system(size: 10))
                                            .foregroundStyle(AppTheme.textMuted)
                                            .lineLimit(2)
                                            .fixedSize(horizontal: false, vertical: true)
                                    }
                                    Text("Run \(entry.workflowID) again")
                                        .font(.system(size: 10))
                                        .foregroundStyle(AppTheme.textMuted)
                                }
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .foregroundStyle(AppTheme.text)
                                .padding(.horizontal, 12)
                                .padding(.vertical, 9)
                                .background(
                                    RoundedRectangle(cornerRadius: 9)
                                        .fill(AppTheme.panel)
                                )
                                .overlay {
                                    RoundedRectangle(cornerRadius: 9)
                                        .stroke(AppTheme.stroke, lineWidth: 1)
                                }
                            }
                            .buttonStyle(.plain)
                            .accessibilityIdentifier("rerun-starter-\(entry.presetID)-button")

                            if let shareText = starterOutcomeShareText(from: entry) {
                                Button {
                                    let pasteboard = NSPasteboard.general
                                    pasteboard.clearContents()
                                    pasteboard.setString(shareText, forType: .string)
                                } label: {
                                    HStack(spacing: 6) {
                                        Image(systemName: "doc.on.doc")
                                            .font(.system(size: 10, weight: .semibold))
                                        Text("Copy Summary")
                                            .font(.system(size: 10, weight: .semibold))
                                    }
                                    .foregroundStyle(AppTheme.text)
                                    .padding(.horizontal, 10)
                                    .padding(.vertical, 6)
                                    .background(Capsule().fill(AppTheme.panel))
                                    .overlay {
                                        Capsule().stroke(AppTheme.stroke, lineWidth: 1)
                                    }
                                    .frame(maxWidth: .infinity, alignment: .leading)
                                }
                                .buttonStyle(.plain)
                                .accessibilityIdentifier("copy-starter-summary-\(entry.workflowID)-button")
                            }
                        }
                    }
                }
            }

            Button(action: onCreateRecommendedStarter) {
                VStack(alignment: .leading, spacing: 5) {
                    HStack(spacing: 6) {
                        Image(systemName: "wand.and.stars")
                            .font(.system(size: 10, weight: .bold))
                        Text("Recommended Starter…")
                            .font(.system(size: 11, weight: .semibold))
                    }
                    Text("Manual will inspect changed files and choose the best-fit starter for this repository.")
                        .font(.system(size: 10))
                        .foregroundStyle(AppTheme.textMuted)
                        .fixedSize(horizontal: false, vertical: true)
                    Text(WorkflowStarterDefinition.recommendedStarterSelectionSummary())
                        .font(.system(size: 10))
                        .foregroundStyle(AppTheme.textFaint)
                        .fixedSize(horizontal: false, vertical: true)
                    Text("You get a ready-to-run review, summary, or test plan in the Output panel.")
                        .font(.system(size: 10))
                        .foregroundStyle(AppTheme.textMuted)
                        .fixedSize(horizontal: false, vertical: true)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .foregroundStyle(AppTheme.text)
                .padding(.horizontal, 12)
                .padding(.vertical, 9)
                .background(
                    RoundedRectangle(cornerRadius: 9)
                        .fill(AppTheme.panel)
                )
                .overlay {
                    RoundedRectangle(cornerRadius: 9)
                        .stroke(AppTheme.strokeStrong, lineWidth: 1)
                }
            }
            .buttonStyle(.plain)
            .accessibilityIdentifier("create-recommended-starter-button")

            ForEach(presets, id: \.id) { preset in
                Button(action: { action(preset) }) {
                    VStack(alignment: .leading, spacing: 6) {
                        HStack(spacing: 6) {
                            Image(systemName: "sparkles.rectangle.stack.fill")
                                .font(.system(size: 10, weight: .bold))
                            Text("\(preset.title)…")
                                .font(.system(size: 11, weight: .semibold))
                        }
                        Text(preset.summary)
                            .font(.system(size: 10))
                            .foregroundStyle(.white.opacity(0.82))
                            .fixedSize(horizontal: false, vertical: true)
                        Text("Best when: \(preset.bestWhen)")
                            .font(.system(size: 10))
                            .foregroundStyle(.white.opacity(0.72))
                            .fixedSize(horizontal: false, vertical: true)
                        Text(preset.expectedOutcome)
                            .font(.system(size: 10))
                            .foregroundStyle(.white.opacity(0.9))
                            .fixedSize(horizontal: false, vertical: true)
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .foregroundStyle(.white)
                    .padding(.horizontal, 12)
                    .padding(.vertical, 10)
                    .background(
                        RoundedRectangle(cornerRadius: 9)
                            .fill(
                                LinearGradient(
                                    colors: [AppTheme.accent, Color(red: 0.95, green: 0.30, blue: 0.50)],
                                    startPoint: .leading,
                                    endPoint: .trailing
                                )
                            )
                    )
                }
                .buttonStyle(.plain)
                // See docs/wiki/features/workflow-starters.md for why the app needs
                // stable automation hooks for the first-success starter actions.
                .accessibilityIdentifier("create-starter-\(preset.id)-button")
            }
        }
        .padding(12)
        .background(
            RoundedRectangle(cornerRadius: 12)
                .fill(AppTheme.panelElev)
        )
        .overlay {
            RoundedRectangle(cornerRadius: 12)
                .stroke(AppTheme.stroke, lineWidth: 1)
        }
    }

    private func title(for presetID: String) -> String {
        presets.first(where: { $0.id == presetID })?.title ?? presetID
    }

    private func expectedOutcome(for presetID: String) -> String {
        presets.first(where: { $0.id == presetID })?.expectedOutcome ?? presetID
    }
}

private struct WorkflowListRow: View {
    let workflow: WorkflowSummary
    let isSelected: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 10) {
                Image(systemName: isSelected ? "flowchart.fill" : "flowchart")
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(isSelected ? AppTheme.accent : AppTheme.textMuted)
                    .frame(width: 18)

                VStack(alignment: .leading, spacing: 3) {
                    Text(workflow.workflowID)
                        .font(.system(size: 12, weight: .semibold))
                        .foregroundStyle(AppTheme.text)
                        .lineLimit(1)
                    Text("\(workflow.nodeCount) nodes")
                        .font(.system(size: 10))
                        .foregroundStyle(AppTheme.textMuted)
                }

                Spacer()
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 9)
            .background(
                RoundedRectangle(cornerRadius: 8)
                    .fill(isSelected ? AppTheme.panelElev : Color.clear)
            )
            .overlay {
                RoundedRectangle(cornerRadius: 8)
                    .stroke(isSelected ? AppTheme.strokeStrong : Color.clear, lineWidth: 1)
            }
        }
        .buttonStyle(.plain)
    }
}

private struct SmallActionButton: View {
    let symbol: String
    let label: String
    var isDestructive = false
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 5) {
                Image(systemName: symbol)
                    .font(.system(size: 10, weight: .semibold))
                Text(label)
                    .font(.system(size: 11, weight: .semibold))
            }
            .foregroundStyle(isDestructive ? Color(red: 0.96, green: 0.45, blue: 0.45) : AppTheme.text)
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

private struct RailButton: View {
    let symbol: String
    let isActive: Bool
    let action: (() -> Void)?

    init(symbol: String, isActive: Bool, action: (() -> Void)? = nil) {
        self.symbol = symbol
        self.isActive = isActive
        self.action = action
    }

    var body: some View {
        ZStack {
            if isActive {
                RoundedRectangle(cornerRadius: 9)
                    .fill(AppTheme.panelElev)
                    .frame(width: 38, height: 38)
            }
            Image(systemName: symbol)
                .font(.system(size: 15, weight: .medium))
                .foregroundStyle(isActive ? AppTheme.text : AppTheme.textMuted)
        }
        .frame(width: 40, height: 40)
        .contentShape(Rectangle())
        .onTapGesture {
            action?()
        }
    }
}

// MARK: - Top bar

private struct TopBar: View {
    @ObservedObject var store: WorkflowRunStore
    @Binding var sidebarVisible: Bool
    @Binding var inspectorVisible: Bool
    @Binding var bottomPanelVisible: Bool
    let showOptimization: () -> Void

    var body: some View {
        HStack(spacing: 14) {
            IconButton(symbol: "sidebar.left", isActive: sidebarVisible) {
                withAnimation(.easeInOut(duration: 0.18)) {
                    sidebarVisible.toggle()
                }
            }

            VStack(alignment: .leading, spacing: 2) {
                HStack(spacing: 6) {
                    Text("Personal")
                        .font(.system(size: 11, weight: .medium))
                        .foregroundStyle(AppTheme.textMuted)
                    Image(systemName: "chevron.right")
                        .font(.system(size: 8, weight: .bold))
                        .foregroundStyle(AppTheme.textFaint)
                    Text(store.selectedWorkflowTitle)
                        .font(.system(size: 11, weight: .medium))
                        .foregroundStyle(AppTheme.text)
                }
                HStack(spacing: 8) {
                    Text(store.runID ?? "Draft run")
                        .font(.system(size: 13, weight: .semibold))
                        .foregroundStyle(AppTheme.text)
                    Circle()
                        .fill(AppTheme.textFaint)
                        .frame(width: 3, height: 3)
                    Text(store.progressText)
                        .font(.system(size: 11))
                        .foregroundStyle(AppTheme.textMuted)
                }
            }

            OptimizationPulseView(
                headline: WorkflowOptimizationHeadline(
                    report: store.optimizationReport,
                    analysis: store.optimizationAnalysis
                ),
                action: showOptimization
            )
            .frame(maxWidth: 320)

            Spacer()

            /*
            HStack(spacing: 0) {
                TabPill(label: "Editor", isActive: true)
                TabPill(label: "Executions", isActive: false)
            }
            .padding(3)
            .background(AppTheme.panelElev)
            .clipShape(Capsule())
            .overlay {
                Capsule().stroke(AppTheme.stroke, lineWidth: 1)
            }

            Spacer()
            */

            HStack(spacing: 8) {
                SecondaryButton(label: "Save") {
                    store.saveSelectedWorkflow()
                }
                SecondaryButton(label: "Refresh") {
                    store.refresh()
                }
                ExecuteButton(isRunning: store.isRunning) {
                    withAnimation(.easeInOut(duration: 0.18)) {
                        bottomPanelVisible = true
                    }
                    store.start()
                }
                IconButton(symbol: "rectangle.bottomthird.inset.filled", isActive: bottomPanelVisible) {
                    withAnimation(.easeInOut(duration: 0.18)) {
                        bottomPanelVisible.toggle()
                    }
                }
                IconButton(symbol: "sidebar.right", isActive: inspectorVisible) {
                    withAnimation(.easeInOut(duration: 0.18)) {
                        inspectorVisible.toggle()
                    }
                }
            }
        }
        .padding(.horizontal, 18)
        .frame(maxWidth: .infinity)
        .background(AppTheme.topBar)
        .overlay(alignment: .bottom) {
            Rectangle().fill(AppTheme.stroke).frame(height: 1)
        }
    }
}

private struct TabPill: View {
    let label: String
    let isActive: Bool

    var body: some View {
        Text(label)
            .font(.system(size: 12, weight: .semibold))
            .foregroundStyle(isActive ? AppTheme.text : AppTheme.textMuted)
            .padding(.horizontal, 14)
            .padding(.vertical, 5)
            .background(
                Capsule().fill(isActive ? AppTheme.panel : Color.clear)
            )
    }
}

private struct SecondaryButton: View {
    let label: String
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Text(label)
                .font(.system(size: 12, weight: .semibold))
                .foregroundStyle(AppTheme.text)
                .padding(.horizontal, 14)
                .padding(.vertical, 7)
                .background(
                    Capsule().fill(AppTheme.panelElev)
                )
                .overlay {
                    Capsule().stroke(AppTheme.stroke, lineWidth: 1)
                }
        }
        .buttonStyle(.plain)
    }
}

private struct ExecuteButton: View {
    let isRunning: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 6) {
                Image(systemName: isRunning ? "hourglass" : "play.fill")
                    .font(.system(size: 11, weight: .bold))
                Text(isRunning ? "Running…" : "Execute workflow")
                    .font(.system(size: 12, weight: .semibold))
            }
            .foregroundStyle(.white)
            .padding(.horizontal, 16)
            .padding(.vertical, 8)
            .background(
                Capsule().fill(
                    LinearGradient(
                        colors: isRunning
                            ? [AppTheme.accent.opacity(0.55), AppTheme.accent.opacity(0.55)]
                            : [AppTheme.accent, Color(red: 0.95, green: 0.30, blue: 0.50)],
                        startPoint: .leading,
                        endPoint: .trailing
                    )
                )
            )
            .shadow(color: AppTheme.accent.opacity(isRunning ? 0 : 0.35), radius: 8, y: 2)
        }
        .buttonStyle(.plain)
        .disabled(isRunning)
        .keyboardShortcut("r", modifiers: [.command])
        // See docs/wiki/architecture/manual-app-architecture.md for why mac UI controls need stable automation hooks.
        .accessibilityIdentifier("execute-workflow-button")
        .accessibilityLabel(isRunning ? "Running workflow" : "Execute workflow")
    }
}

private struct IconButton: View {
    let symbol: String
    let isActive: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Image(systemName: symbol)
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(isActive ? AppTheme.text : AppTheme.textMuted)
                .frame(width: 32, height: 30)
                .background(
                    RoundedRectangle(cornerRadius: 7)
                        .fill(isActive ? AppTheme.panelElev : Color.clear)
                )
        }
        .buttonStyle(.plain)
    }
}

// MARK: - Bottom toggle / panel

private struct BottomToggle: View {
    @Binding var visible: Bool
    let eventCount: Int

    var body: some View {
        HStack(spacing: 0) {
            Button {
                withAnimation(.easeInOut(duration: 0.18)) { visible.toggle() }
            } label: {
                HStack(spacing: 8) {
                    Image(systemName: visible ? "chevron.down" : "chevron.up")
                        .font(.system(size: 9, weight: .bold))
                    Text("Logs")
                        .font(.system(size: 11, weight: .semibold))
                    if eventCount > 0 {
                        Text("\(eventCount)")
                            .font(.system(size: 10, weight: .bold))
                            .foregroundStyle(AppTheme.accent)
                            .padding(.horizontal, 7)
                            .padding(.vertical, 1)
                            .background(Capsule().fill(AppTheme.accentMuted))
                    }
                }
                .foregroundStyle(AppTheme.textMuted)
                .padding(.horizontal, 14)
                .frame(maxHeight: .infinity)
            }
            .buttonStyle(.plain)

            Spacer()
        }
        .frame(maxWidth: .infinity)
        .background(AppTheme.panel)
        .overlay(alignment: .top) {
            Rectangle().fill(AppTheme.stroke).frame(height: 1)
        }
    }
}

private struct BottomPanel: View {
    let events: [WorkflowEventModel]
    let nodes: [WorkflowNodeModel]
    let workflowID: String?
    let runID: String?
    let rawWorkflowJSON: String
    let optimizationReport: WorkflowOptimizationReport?
    let optimizationAnalysis: WorkflowOptimizationAnalysis?
    let optimizationLoading: Bool
    @Binding var selectedTab: BottomPanelTab
    let onClose: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 0) {
                LogTab(label: "Executions", isActive: selectedTab == .events) {
                    selectedTab = .events
                }
                LogTab(label: "JSON", isActive: selectedTab == .json) {
                    selectedTab = .json
                }
                LogTab(label: "Output", isActive: selectedTab == .output) {
                    selectedTab = .output
                }
                LogTab(label: "Optimization", isActive: selectedTab == .optimization) {
                    selectedTab = .optimization
                }
                Spacer()
                Button(action: onClose) {
                    Image(systemName: "xmark")
                        .font(.system(size: 10, weight: .semibold))
                        .foregroundStyle(AppTheme.textMuted)
                        .padding(.horizontal, 12)
                        .frame(maxHeight: .infinity)
                }
                .buttonStyle(.plain)
            }
            .frame(height: 36)
            .background(AppTheme.panel)
            .overlay(alignment: .bottom) {
                Rectangle().fill(AppTheme.stroke).frame(height: 1)
            }
            .overlay(alignment: .top) {
                Rectangle().fill(AppTheme.stroke).frame(height: 1)
            }

            if selectedTab == .events {
                EventTimelineView(events: events)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .background(AppTheme.panel)
            } else if selectedTab == .json {
                ScrollView {
                    Text(rawWorkflowJSON)
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundStyle(AppTheme.textMuted)
                        .textSelection(.enabled)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(16)
                }
                .background(AppTheme.panel)
            } else if selectedTab == .output {
                WorkflowOutputsView(
                    workflowID: workflowID,
                    runID: runID,
                    nodes: nodes
                )
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .background(AppTheme.panel)
            } else {
                OptimizationSummaryView(
                    report: optimizationReport,
                    analysis: optimizationAnalysis,
                    isLoading: optimizationLoading
                )
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .background(AppTheme.panel)
    }
}

private struct WorkflowOutputsView: View {
    let workflowID: String?
    let runID: String?
    let nodes: [WorkflowNodeModel]

    private var completedOutputs: [WorkflowNodeModel] {
        nodes.filter { $0.result != nil }
    }

    private var starterSummary: StarterOutcomeSummary? {
        starterOutcomeSummary(
            workflowID: workflowID,
            runID: runID,
            nodes: nodes
        )
    }

    var body: some View {
        if completedOutputs.isEmpty {
            VStack(spacing: 8) {
                Image(systemName: "text.append")
                    .font(.system(size: 26))
                    .foregroundStyle(AppTheme.textFaint)
                Text("No node outputs yet")
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(AppTheme.textMuted)
                Text("Starter review results and script outputs will appear here as soon as nodes finish.")
                    .font(.system(size: 11))
                    .foregroundStyle(AppTheme.textFaint)
                    .multilineTextAlignment(.center)
                    .frame(maxWidth: 320)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else {
            ScrollView {
                VStack(alignment: .leading, spacing: 12) {
                    if let starterSummary {
                        VStack(alignment: .leading, spacing: 10) {
                            HStack(spacing: 8) {
                                Text("Starter Outcome")
                                    .font(.system(size: 12, weight: .semibold))
                                    .foregroundStyle(AppTheme.text)
                                Spacer()
                                Button {
                                    let pasteboard = NSPasteboard.general
                                    pasteboard.clearContents()
                                    pasteboard.setString(starterOutcomeShareText(starterSummary), forType: .string)
                                } label: {
                                    HStack(spacing: 6) {
                                        Image(systemName: "doc.on.doc")
                                            .font(.system(size: 10, weight: .semibold))
                                        Text("Copy Summary")
                                            .font(.system(size: 10, weight: .semibold))
                                    }
                                    .foregroundStyle(AppTheme.text)
                                    .padding(.horizontal, 10)
                                    .padding(.vertical, 6)
                                    .background(Capsule().fill(AppTheme.panel))
                                    .overlay {
                                        Capsule().stroke(AppTheme.stroke, lineWidth: 1)
                                    }
                                }
                                .buttonStyle(.plain)
                            }

                            Text(starterSummary.label)
                                .font(.system(size: 11, weight: .semibold))
                                .foregroundStyle(AppTheme.textMuted)
                            Text(starterSummary.text)
                                .font(.system(size: 11, design: .monospaced))
                                .foregroundStyle(AppTheme.text)
                                .textSelection(.enabled)
                                .frame(maxWidth: .infinity, alignment: .leading)
                            Text("Reusable command: \(starterSummary.rerunCommand)")
                                .font(.system(size: 10, design: .monospaced))
                                .foregroundStyle(AppTheme.textFaint)
                                .textSelection(.enabled)
                        }
                        .padding(16)
                        .background(
                            RoundedRectangle(cornerRadius: 14).fill(AppTheme.panelElev)
                        )
                        .overlay {
                            RoundedRectangle(cornerRadius: 14)
                                .stroke(AppTheme.strokeStrong, lineWidth: 1)
                        }
                    }

                    ForEach(completedOutputs) { node in
                        VStack(alignment: .leading, spacing: 8) {
                            HStack(spacing: 8) {
                                Text(node.title)
                                    .font(.system(size: 12, weight: .semibold))
                                    .foregroundStyle(AppTheme.text)
                                Spacer()
                                Text(node.kind.rawValue)
                                    .font(.system(size: 10, weight: .semibold))
                                    .foregroundStyle(AppTheme.textMuted)
                            }

                            Text(node.result ?? "")
                                .font(.system(size: 11, design: .monospaced))
                                .foregroundStyle(AppTheme.text)
                                .textSelection(.enabled)
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .padding(12)
                                .background(
                                    RoundedRectangle(cornerRadius: 10).fill(AppTheme.canvas)
                                )
                                .overlay {
                                    RoundedRectangle(cornerRadius: 10)
                                        .stroke(AppTheme.stroke, lineWidth: 1)
                                }
                        }
                        .padding(16)
                        .background(
                            RoundedRectangle(cornerRadius: 14).fill(AppTheme.panelElev)
                        )
                        .overlay {
                            RoundedRectangle(cornerRadius: 14)
                                .stroke(AppTheme.stroke, lineWidth: 1)
                        }
                    }
                }
                .padding(16)
            }
        }
    }
}

private struct LogTab: View {
    let label: String
    let isActive: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Text(label)
                .font(.system(size: 12, weight: .semibold))
                .foregroundStyle(isActive ? AppTheme.text : AppTheme.textMuted)
                .padding(.horizontal, 18)
                .frame(maxHeight: .infinity)
                .overlay(alignment: .bottom) {
                    Rectangle()
                        .fill(isActive ? AppTheme.accent : Color.clear)
                        .frame(height: 2)
                }
        }
        .buttonStyle(.plain)
    }
}
