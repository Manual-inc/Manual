import Foundation

final class StepRegistry {
    typealias Handler = (ManualWorld, [String], String, StaticString, UInt) throws -> Void

    struct Entry {
        let pattern: NSRegularExpression
        let handler: Handler
        let source: String
    }

    nonisolated(unsafe) static let shared = StepRegistry()

    private(set) var entries: [Entry] = []
    private var registered = false
    private let lock = NSLock()

    func ensureRegistered() {
        lock.lock()
        defer { lock.unlock() }
        guard !registered else { return }
        registered = true
        IntentDrivenSteps.register(in: self)
    }

    func step(_ pattern: String, _ handler: @escaping Handler) {
        let anchored = "^\(pattern)$"
        do {
            let regex = try NSRegularExpression(pattern: anchored)
            entries.append(Entry(pattern: regex, handler: handler, source: pattern))
        } catch {
            preconditionFailure("Invalid step pattern: \(pattern) — \(error)")
        }
    }

    func match(_ text: String) -> (Entry, [String])? {
        let range = NSRange(text.startIndex..<text.endIndex, in: text)
        for entry in entries {
            guard let result = entry.pattern.firstMatch(in: text, range: range) else { continue }
            var captures: [String] = []
            for i in 1..<result.numberOfRanges {
                let captureRange = result.range(at: i)
                if captureRange.location == NSNotFound {
                    captures.append("")
                } else if let swiftRange = Range(captureRange, in: text) {
                    captures.append(String(text[swiftRange]))
                }
            }
            return (entry, captures)
        }
        return nil
    }
}

enum StepError: Error, CustomStringConvertible {
    case undefined(String)
    case assertion(String)

    var description: String {
        switch self {
        case let .undefined(text): "정의되지 않은 단계: \(text)"
        case let .assertion(message): message
        }
    }
}

func expectStep(_ condition: @autoclosure () -> Bool, _ message: String, file: StaticString, line: UInt) throws {
    if !condition() {
        throw StepError.assertion(message)
    }
}
