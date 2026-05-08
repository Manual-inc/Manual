import AppKit
import SwiftUI

@main
struct ManualMacApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate

    var body: some Scene {
        WindowGroup {
            ContentView()
                .frame(minWidth: 1100, minHeight: 720)
        }
        .commands {
            CommandMenu("Workflow") {
                Button("Start Example Workflow") {
                    NotificationCenter.default.post(name: .startExampleWorkflow, object: nil)
                }
                .keyboardShortcut("r", modifiers: [.command])
            }
        }
    }
}

final class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.regular)
        NSApp.activate(ignoringOtherApps: true)
    }
}

extension Notification.Name {
    static let startExampleWorkflow = Notification.Name("ManualMacStartExampleWorkflow")
}
