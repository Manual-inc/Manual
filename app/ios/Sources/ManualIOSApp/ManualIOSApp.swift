import SwiftUI

public struct ManualIOSRootView: View {
    public init() {}

    public var body: some View {
        NavigationStack {
            VStack(spacing: 16) {
                Image(systemName: "iphone.gen3")
                    .font(.system(size: 52, weight: .semibold))

                Text("Manual iOS")
                    .font(.largeTitle.bold())

                Text("SwiftUI module initialized for iOS.")
                    .foregroundStyle(.secondary)
            }
            .padding(32)
            .navigationTitle("Manual")
        }
    }
}

