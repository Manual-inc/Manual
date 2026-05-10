import AppKit
import SwiftUI

struct NodeKindIcon: View {
    let kind: WorkflowNodeKind
    let symbolSize: CGFloat

    var body: some View {
        if let image = logoImage(for: kind) {
            Image(nsImage: image)
                .resizable()
                .scaledToFit()
                .padding(7)
        } else {
            Image(systemName: AppTheme.kindIcon(kind))
                .font(.system(size: symbolSize, weight: .semibold))
                .foregroundStyle(AppTheme.kindColor(kind))
        }
    }

    private func logoImage(for kind: WorkflowNodeKind) -> NSImage? {
        guard
            let resource = AppTheme.kindLogoResource(kind),
            let url = Bundle.module.url(forResource: resource.name, withExtension: resource.ext)
        else {
            return nil
        }

        return NSImage(contentsOf: url)
    }
}
