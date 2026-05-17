import Foundation

// See docs/wiki/architecture/manual-app-architecture.md for why app commands and views share typed notifications.
public extension Notification.Name {
    static let startExampleWorkflow = Notification.Name("ManualMacStartExampleWorkflow")
}
