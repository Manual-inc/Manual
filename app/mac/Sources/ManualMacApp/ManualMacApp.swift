import SwiftUI

@main
struct ManualMacApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
                .frame(minWidth: 720, minHeight: 480)
        }
    }
}

private struct ContentView: View {
    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "sparkles")
                .font(.system(size: 48, weight: .semibold))

            Text("Manual Mac")
                .font(.largeTitle.bold())

            Text("SwiftUI app initialized for macOS.")
                .foregroundStyle(.secondary)
        }
        .padding(40)
    }
}

