import Testing
@testable import ManualMacApp

@Suite("Bottom Panel State")
struct BottomPanelStateTests {
    @Test func showOptimization_makesPanelVisible_andSelectsOptimizationTab() {
        var state = WorkflowPanelState(
            isBottomPanelVisible: false,
            selectedTab: .events
        )

        state.showOptimization()

        #expect(state.isBottomPanelVisible)
        #expect(state.selectedTab == .optimization)
    }
}
