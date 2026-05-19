// See docs/wiki/systems/매뉴얼-최적화-기능.md: optimization should be
// discoverable from the main workflow chrome, not buried in a hidden panel.
enum BottomPanelTab: String, Equatable, Sendable {
    case events
    case json
    case output
    case optimization
}

struct WorkflowPanelState: Equatable, Sendable {
    var isBottomPanelVisible: Bool
    var selectedTab: BottomPanelTab

    mutating func showOptimization() {
        isBottomPanelVisible = true
        selectedTab = .optimization
    }

    mutating func showOutput() {
        isBottomPanelVisible = true
        selectedTab = .output
    }
}
