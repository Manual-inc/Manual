import SwiftUI

enum AppTheme {
    static let canvas        = Color(red: 0.16, green: 0.17, blue: 0.21)
    static let canvasGrid    = Color.white.opacity(0.06)
    static let panel         = Color(red: 0.13, green: 0.14, blue: 0.17)
    static let panelElev     = Color(red: 0.18, green: 0.19, blue: 0.23)
    static let rail          = Color(red: 0.10, green: 0.11, blue: 0.13)
    static let topBar        = Color(red: 0.13, green: 0.14, blue: 0.17)

    static let nodeCard      = Color(red: 0.21, green: 0.22, blue: 0.27)
    static let nodeStroke    = Color.white.opacity(0.10)

    static let stroke        = Color.white.opacity(0.08)
    static let strokeStrong  = Color.white.opacity(0.16)

    static let text          = Color.white.opacity(0.92)
    static let textMuted     = Color.white.opacity(0.55)
    static let textFaint     = Color.white.opacity(0.32)

    static let accent        = Color(red: 1.00, green: 0.43, blue: 0.36)
    static let accentMuted   = Color(red: 1.00, green: 0.43, blue: 0.36).opacity(0.18)

    static let edge          = Color.white.opacity(0.20)
    static let edgeActive    = Color(red: 1.00, green: 0.43, blue: 0.36)

    static func statusColor(_ status: WorkflowNodeStatus) -> Color {
        switch status {
        case .idle:      return Color.white.opacity(0.45)
        case .running:   return Color(red: 0.40, green: 0.65, blue: 1.00)
        case .succeeded: return Color(red: 0.42, green: 0.85, blue: 0.50)
        case .failed:    return Color(red: 0.96, green: 0.45, blue: 0.45)
        }
    }

    static func kindColor(_ kind: WorkflowNodeKind) -> Color {
        switch kind {
        case .context: return Color(red: 0.50, green: 0.62, blue: 1.00)
        case .script:  return Color(red: 1.00, green: 0.55, blue: 0.30)
        case .agent:   return Color(red: 0.93, green: 0.39, blue: 0.71)
        case .digest:  return Color(red: 0.42, green: 0.85, blue: 0.50)
        }
    }

    static func kindIcon(_ kind: WorkflowNodeKind) -> String {
        switch kind {
        case .context: return "doc.text"
        case .script:  return "chevron.left.slash.chevron.right"
        case .agent:   return "sparkles"
        case .digest:  return "tray.full"
        }
    }
}
