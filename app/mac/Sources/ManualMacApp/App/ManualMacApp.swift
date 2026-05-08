import AppKit
import SwiftUI

@main
struct ManualMacApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate

    var body: some Scene {
        WindowGroup {
            ContentView()
                .frame(minWidth: 1240, minHeight: 760)
                .preferredColorScheme(.dark)
        }
        .commands {
            CommandMenu("Workflow") {
                Button("Execute Workflow") {
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
        NSApp.appearance = NSAppearance(named: .darkAqua)
    }
}

extension Notification.Name {
    static let startExampleWorkflow = Notification.Name("ManualMacStartExampleWorkflow")
}
