import Foundation

// See docs/wiki/systems/샌드박스-기능.md: the mac GUI edits reusable execution boundaries before a node runs.
struct SandboxPolicyModel: Identifiable, Equatable, Sendable {
    let id: String
    var name: String
    var allowRead: [String]
    var allowWrite: [String]
    var allowCommands: [String]
    var denyCommands: [String]
    var allowNetwork: [String]
    var denyNetwork: [String]
    var allowEnv: [String]
    var tmpWrite: [String]
    var cacheWrite: [String]
    var updatedAt: String
    var history: [SandboxPolicyHistoryEntry]

    init(_ object: [String: Any]) throws {
        guard let id = object["id"] as? String else {
            throw AppServerClientError.invalidResponse
        }

        self.id = id
        self.name = object["name"] as? String ?? id
        self.allowRead = Self.stringArray(object["allow_read"])
        self.allowWrite = Self.stringArray(object["allow_write"])
        self.allowCommands = Self.stringArray(object["allow_commands"])
        self.denyCommands = Self.stringArray(object["deny_commands"])
        self.allowNetwork = Self.stringArray(object["allow_network"])
        self.denyNetwork = Self.stringArray(object["deny_network"])
        self.allowEnv = Self.stringArray(object["allow_env"])
        self.tmpWrite = Self.stringArray(object["tmp_write"])
        self.cacheWrite = Self.stringArray(object["cache_write"])
        self.updatedAt = object["updated_at"] as? String ?? ""
        self.history = ((object["history"] as? [[String: Any]]) ?? []).map(SandboxPolicyHistoryEntry.init)
    }

    private static func stringArray(_ value: Any?) -> [String] {
        (value as? [Any])?.compactMap { $0 as? String } ?? []
    }
}

struct SandboxPolicyHistoryEntry: Identifiable, Equatable, Sendable {
    let id = UUID()
    let at: String
    let change: String
    let hasDiff: Bool

    init(_ object: [String: Any]) {
        self.at = object["at"] as? String ?? ""
        self.change = object["change"] as? String ?? "sandbox_updated"
        self.hasDiff = object["before"] != nil && object["after"] != nil
    }
}

struct SandboxDecisionModel: Equatable, Sendable {
    let allowed: Bool
    let approvalRequired: Bool
    let operation: String
    let target: String
    let reason: String
    let violation: Bool
    let haltNode: Bool
    let allowedTmp: [String]
    let allowedCache: [String]

    init(_ object: [String: Any]) throws {
        guard
            let allowed = object["allowed"] as? Bool,
            let operation = object["operation"] as? String,
            let target = object["target"] as? String,
            let reason = object["reason"] as? String
        else {
            throw AppServerClientError.invalidResponse
        }

        self.allowed = allowed
        self.approvalRequired = object["approval_required"] as? Bool ?? false
        self.operation = operation
        self.target = target
        self.reason = reason
        self.violation = object["violation"] as? Bool ?? !allowed
        self.haltNode = object["halt_node"] as? Bool ?? !allowed
        self.allowedTmp = (object["allowed_tmp"] as? [Any])?.compactMap { $0 as? String } ?? []
        self.allowedCache = (object["allowed_cache"] as? [Any])?.compactMap { $0 as? String } ?? []
    }
}

struct SandboxPolicyDraft: Equatable, Sendable {
    var name = "Docs Writer"
    var allowRead = "docs/**"
    var allowWrite = "docs/wiki/**\n.manual/tmp/**\n.manual/cache/**"
    var allowCommands = "scripts/**"
    var denyCommands = "scripts/deploy.sh"
    var allowNetwork = "api.example.com"
    var denyNetwork = ""
    var allowEnv = "MANUAL_*"
    var tmpWrite = ".manual/tmp/**"
    var cacheWrite = ".manual/cache/**"

    init() {}

    init(policy: SandboxPolicyModel) {
        name = policy.name
        allowRead = Self.join(policy.allowRead)
        allowWrite = Self.join(policy.allowWrite)
        allowCommands = Self.join(policy.allowCommands)
        denyCommands = Self.join(policy.denyCommands)
        allowNetwork = Self.join(policy.allowNetwork)
        denyNetwork = Self.join(policy.denyNetwork)
        allowEnv = Self.join(policy.allowEnv)
        tmpWrite = Self.join(policy.tmpWrite)
        cacheWrite = Self.join(policy.cacheWrite)
    }

    var asParameters: [String: Any] {
        [
            "name": name.trimmingCharacters(in: .whitespacesAndNewlines),
            "allow_read": Self.split(allowRead),
            "allow_write": Self.split(allowWrite),
            "allow_commands": Self.split(allowCommands),
            "deny_commands": Self.split(denyCommands),
            "allow_network": Self.split(allowNetwork),
            "deny_network": Self.split(denyNetwork),
            "allow_env": Self.split(allowEnv),
            "tmp_write": Self.split(tmpWrite),
            "cache_write": Self.split(cacheWrite),
        ]
    }

    private static func split(_ value: String) -> [String] {
        value
            .split(whereSeparator: { $0 == "\n" || $0 == "," })
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
    }

    private static func join(_ values: [String]) -> String {
        values.joined(separator: "\n")
    }
}
