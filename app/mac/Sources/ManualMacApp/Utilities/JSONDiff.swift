import Foundation

enum DiffOp {
    case same, added, removed
}

struct DiffLine {
    let op: DiffOp
    let text: String
}

enum JSONDiff {
    static func diff(old: String?, new: String?) -> [DiffLine] {
        let oldLines = prettyLines(old)
        let newLines = prettyLines(new)
        return lcs(old: oldLines, new: newLines)
    }

    static func prettyJSON(_ value: Any?) -> String? {
        guard let value else { return nil }
        guard JSONSerialization.isValidJSONObject(value) || value is String || value is NSNumber else {
            return String(describing: value)
        }
        if let str = value as? String { return str }
        guard let data = try? JSONSerialization.data(withJSONObject: value, options: [.prettyPrinted, .sortedKeys]),
              let string = String(data: data, encoding: .utf8) else { return nil }
        return string
    }

    private static func prettyLines(_ text: String?) -> [String] {
        guard let text, !text.isEmpty else { return [] }
        return text.components(separatedBy: "\n")
    }

    private static func lcs(old: [String], new: [String]) -> [DiffLine] {
        let m = old.count, n = new.count
        var dp = Array(repeating: Array(repeating: 0, count: n + 1), count: m + 1)

        for i in 1...max(m, 1) where i <= m {
            for j in 1...max(n, 1) where j <= n {
                dp[i][j] = old[i-1] == new[j-1] ? dp[i-1][j-1] + 1 : max(dp[i-1][j], dp[i][j-1])
            }
        }

        var result: [DiffLine] = []
        var i = m, j = n
        while i > 0 || j > 0 {
            if i > 0 && j > 0 && old[i-1] == new[j-1] {
                result.append(DiffLine(op: .same, text: old[i-1]))
                i -= 1; j -= 1
            } else if j > 0 && (i == 0 || dp[i][j-1] >= dp[i-1][j]) {
                result.append(DiffLine(op: .added, text: new[j-1]))
                j -= 1
            } else {
                result.append(DiffLine(op: .removed, text: old[i-1]))
                i -= 1
            }
        }
        return result.reversed()
    }
}
