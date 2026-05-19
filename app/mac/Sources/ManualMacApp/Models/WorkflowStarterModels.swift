import Foundation

// See docs/wiki/features/workflow-starters.md: mac onboarding should be able to
// create the same first-success starter workflows without sending users back to CLI docs.
enum WorkflowStarterError: Error, LocalizedError {
    case unsupportedAgent(String)
    case unsupportedPreset(String)
    case noAvailableAgent
    case notGitRepository(String)
    case gitCommandFailed(String)

    var errorDescription: String? {
        switch self {
        case let .unsupportedAgent(agent):
            "Unsupported starter agent: \(agent)"
        case let .unsupportedPreset(preset):
            "Unsupported starter preset: \(preset)"
        case .noAvailableAgent:
            "No supported local agent is available. Install codex, claude, or pi first."
        case let .notGitRepository(path):
            "Code review starter requires a git repository: \(path)"
        case let .gitCommandFailed(message):
            message
        }
    }
}

struct AppServerAgentAvailability: Equatable, Sendable {
    let name: String
    let available: Bool
    let path: String?

    init(name: String, available: Bool, path: String?) {
        self.name = name
        self.available = available
        self.path = path
    }

    init(_ object: [String: Any]) throws {
        guard
            let name = object["name"] as? String,
            let available = object["available"] as? Bool
        else {
            throw AppServerClientError.invalidResponse
        }

        self.name = name
        self.available = available
        self.path = object["path"] as? String
    }
}

struct WorkflowStarterPreset: Equatable, Sendable {
    let id: String
    let title: String
    let summary: String
    let bestWhen: String
    let expectedOutcome: String
    let workflowIDSuffix: String
}

struct WorkflowStarterRecommendation: Equatable, Sendable {
    let preset: WorkflowStarterPreset
    let reason: String
}

struct WorkflowStarterRecommendationPreview: Equatable, Sendable {
    let preset: WorkflowStarterPreset
    let reason: String
    let changedFilesHint: String
    let expectedOutcome: String
}

struct WorkflowStarterRecentEntry: Codable, Equatable, Sendable {
    let presetID: String
    let repositoryRootPath: String
    let workflowID: String
    let recommendationReason: String?
    let outcomeLabel: String?
    let outcomeText: String?

    init(
        presetID: String,
        repositoryRootPath: String,
        workflowID: String,
        recommendationReason: String? = nil,
        outcomeLabel: String? = nil,
        outcomeText: String? = nil
    ) {
        self.presetID = presetID
        self.repositoryRootPath = repositoryRootPath
        self.workflowID = workflowID
        self.recommendationReason = recommendationReason
        self.outcomeLabel = outcomeLabel
        self.outcomeText = outcomeText
    }
}

enum WorkflowStarterDefinition {
    static let availablePresets: [WorkflowStarterPreset] = [
        WorkflowStarterPreset(
            id: "code-review",
            title: "Code Review Starter",
            summary: "Review repository changes for correctness bugs, regressions, risky assumptions, and missing tests.",
            bestWhen: "You want a correctness-focused review before trusting the change.",
            expectedOutcome: "You get a concise review of bugs, regressions, risky assumptions, and missing tests.",
            workflowIDSuffix: "review"
        ),
        WorkflowStarterPreset(
            id: "change-summary",
            title: "Change Summary Starter",
            summary: "summarize the repository changes into a concise update covering what changed, why it matters, and what to verify next.",
            bestWhen: "The diff is mostly docs, markdown, or reader-facing content updates.",
            expectedOutcome: "You get a short human-readable change update with follow-up verification guidance.",
            workflowIDSuffix: "summary"
        ),
        WorkflowStarterPreset(
            id: "test-plan",
            title: "Test Plan Starter",
            summary: "outline the highest-value automated and manual checks for the repository changes before you run them.",
            bestWhen: "Code changed but matching tests or verification steps did not.",
            expectedOutcome: "You get a focused test plan covering the highest-value automated and manual checks.",
            workflowIDSuffix: "test-plan"
        ),
    ]

    static func recommendedStarterSelectionSummary() -> String {
        // See docs/wiki/features/workflow-starters.md: before launch, users
        // should understand both why Manual recommends a starter and what they will get.
        "Docs-only changes -> Change Summary. Code without matching tests -> Test Plan. Otherwise -> Code Review."
    }

    static func suggestedWorkflowID(repositoryRootPath: String, presetID: String = "code-review") -> String {
        let name = repositoryDisplayName(repositoryRootPath: repositoryRootPath)
        let normalized = name
            .lowercased()
            .map { character -> Character in
                character.isLetter || character.isNumber ? character : "-"
            }
        let collapsed = String(normalized)
            .replacingOccurrences(of: "-+", with: "-", options: .regularExpression)
            .trimmingCharacters(in: CharacterSet(charactersIn: "-"))
        let suffix = availablePresets.first(where: { $0.id == presetID })?.workflowIDSuffix ?? "review"
        return "starter-\(collapsed.isEmpty ? "repo" : collapsed)-\(suffix)"
    }

    static func repositoryDisplayName(repositoryRootPath: String) -> String {
        URL(fileURLWithPath: repositoryRootPath, isDirectory: true).lastPathComponent
    }

    static func recentEntries(from json: String) -> [WorkflowStarterRecentEntry] {
        guard let data = json.data(using: .utf8) else { return [] }
        return (try? JSONDecoder().decode([WorkflowStarterRecentEntry].self, from: data)) ?? []
    }

    static func encodeRecentEntries(_ entries: [WorkflowStarterRecentEntry]) -> String {
        guard let data = try? JSONEncoder().encode(entries),
              let string = String(data: data, encoding: .utf8)
        else {
            return "[]"
        }
        return string
    }

    static func updatedRecentEntries(
        _ entries: [WorkflowStarterRecentEntry],
        with newEntry: WorkflowStarterRecentEntry,
        limit: Int = 5
    ) -> [WorkflowStarterRecentEntry] {
        var updated = entries.filter {
            !($0.presetID == newEntry.presetID && $0.repositoryRootPath == newEntry.repositoryRootPath)
        }
        updated.insert(newEntry, at: 0)
        return Array(updated.prefix(limit))
    }

    static func mergedRecentEntries(
        local: [WorkflowStarterRecentEntry],
        shared: [WorkflowStarterRecentEntry],
        limit: Int = 5
    ) -> [WorkflowStarterRecentEntry] {
        // See docs/wiki/features/workflow-starters.md: starter history should
        // follow the user across surfaces instead of feeling device-local.
        var merged = shared

        for entry in local where !merged.contains(where: {
            $0.presetID == entry.presetID && $0.repositoryRootPath == entry.repositoryRootPath
        }) {
            merged.append(entry)
        }

        return Array(merged.prefix(limit))
    }

    static func preferredAgent(from agents: [AppServerAgentAvailability]) -> String? {
        for candidate in ["codex", "claude", "pi"] {
            if agents.contains(where: { $0.name == candidate && $0.available }) {
                return candidate
            }
        }
        return nil
    }

    static func recommendedPreset(forChangedFiles changedFiles: [String]) -> WorkflowStarterRecommendation {
        if changedFiles.isEmpty {
            return WorkflowStarterRecommendation(
                preset: preset(id: "code-review"),
                reason: "No active diff was detected, so code-review is the safest general default."
            )
        }

        let docsLike = changedFiles.filter(isDocsLikePath)
        if docsLike.count == changedFiles.count {
            return WorkflowStarterRecommendation(
                preset: preset(id: "change-summary"),
                reason: "Detected mostly documentation or markdown changes."
            )
        }

        let codeLike = changedFiles.filter(isCodeLikePath)
        let testLike = changedFiles.filter(isTestLikePath)
        if !codeLike.isEmpty && testLike.isEmpty {
            return WorkflowStarterRecommendation(
                preset: preset(id: "test-plan"),
                reason: "Detected code changes without matching test updates."
            )
        }

        return WorkflowStarterRecommendation(
            preset: preset(id: "code-review"),
            reason: "Detected implementation changes that benefit from a correctness and regression review."
        )
    }

    static func recommendedPreset(repositoryRootPath: String) throws -> WorkflowStarterRecommendation {
        recommendedPreset(forChangedFiles: try changedFiles(repositoryRootPath: repositoryRootPath))
    }

    static func recommendedStarterPreview(forChangedFiles changedFiles: [String]) -> WorkflowStarterRecommendationPreview {
        let recommendation = recommendedPreset(forChangedFiles: changedFiles)
        return WorkflowStarterRecommendationPreview(
            preset: recommendation.preset,
            reason: recommendation.reason,
            changedFilesHint: changedFilesHint(forChangedFiles: changedFiles),
            expectedOutcome: recommendation.preset.expectedOutcome
        )
    }

    static func recommendedStarterPreview(repositoryRootPath: String) throws -> WorkflowStarterRecommendationPreview {
        // See docs/wiki/features/workflow-starters.md: the quick-start surface
        // should explain the next recommended starter before the user launches it.
        return recommendedStarterPreview(
            forChangedFiles: try changedFiles(repositoryRootPath: repositoryRootPath)
        )
    }

    static func codeReviewWorkflow(
        workflowID: String,
        repositoryRootPath: String,
        agent: String,
        model: String? = nil
    ) throws -> [String: Any] {
        guard ["codex", "claude", "pi"].contains(agent) else {
            throw WorkflowStarterError.unsupportedAgent(agent)
        }

        var reviewNode: [String: Any] = [
            "id": "review",
            "kind": agent,
            "prompt": codeReviewPrompt(),
            "cwd": repositoryRootPath,
        ]
        if let model, !model.isEmpty {
            reviewNode["model"] = model
        }

        return [
            "id": workflowID,
            "nodes": [
                [
                    "id": "collect_diff",
                    "kind": "script",
                    "script": codeReviewScript(repositoryRootPath: repositoryRootPath),
                    "sandbox_policy": codeReviewSandbox(repositoryRootPath: repositoryRootPath),
                ],
                reviewNode,
            ],
            "dependencies": [
                [
                    "node": "review",
                    "depends_on": "collect_diff",
                ],
            ],
        ]
    }

    static func changeSummaryWorkflow(
        workflowID: String,
        repositoryRootPath: String,
        agent: String,
        model: String? = nil
    ) throws -> [String: Any] {
        guard ["codex", "claude", "pi"].contains(agent) else {
            throw WorkflowStarterError.unsupportedAgent(agent)
        }

        var summaryNode: [String: Any] = [
            "id": "summary",
            "kind": agent,
            "prompt": changeSummaryPrompt(),
            "cwd": repositoryRootPath,
        ]
        if let model, !model.isEmpty {
            summaryNode["model"] = model
        }

        return [
            "id": workflowID,
            "nodes": [
                [
                    "id": "collect_diff",
                    "kind": "script",
                    "script": codeReviewScript(repositoryRootPath: repositoryRootPath),
                    "sandbox_policy": codeReviewSandbox(repositoryRootPath: repositoryRootPath),
                ],
                summaryNode,
            ],
            "dependencies": [
                [
                    "node": "summary",
                    "depends_on": "collect_diff",
                ],
            ],
        ]
    }

    static func testPlanWorkflow(
        workflowID: String,
        repositoryRootPath: String,
        agent: String,
        model: String? = nil
    ) throws -> [String: Any] {
        guard ["codex", "claude", "pi"].contains(agent) else {
            throw WorkflowStarterError.unsupportedAgent(agent)
        }

        var testPlanNode: [String: Any] = [
            "id": "test_plan",
            "kind": agent,
            "prompt": testPlanPrompt(),
            "cwd": repositoryRootPath,
        ]
        if let model, !model.isEmpty {
            testPlanNode["model"] = model
        }

        return [
            "id": workflowID,
            "nodes": [
                [
                    "id": "collect_diff",
                    "kind": "script",
                    "script": codeReviewScript(repositoryRootPath: repositoryRootPath),
                    "sandbox_policy": codeReviewSandbox(repositoryRootPath: repositoryRootPath),
                ],
                testPlanNode,
            ],
            "dependencies": [
                [
                    "node": "test_plan",
                    "depends_on": "collect_diff",
                ],
            ],
        ]
    }

    static func resolveRepositoryRootPath(from selectedPath: String) throws -> String {
        let candidate = URL(fileURLWithPath: selectedPath, isDirectory: true)
            .standardizedFileURL
            .path
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
        process.arguments = ["git", "-C", candidate, "rev-parse", "--show-toplevel"]

        let output = Pipe()
        let errors = Pipe()
        process.standardOutput = output
        process.standardError = errors

        try process.run()
        process.waitUntilExit()

        guard process.terminationStatus == 0 else {
            let message = String(data: errors.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8)?
                .trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
            if message.isEmpty {
                throw WorkflowStarterError.notGitRepository(candidate)
            }
            throw WorkflowStarterError.gitCommandFailed(message)
        }

        let data = output.fileHandleForReading.readDataToEndOfFile()
        let root = String(data: data, encoding: .utf8)?
            .trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        guard !root.isEmpty else {
            throw WorkflowStarterError.notGitRepository(candidate)
        }
        return root
    }

    static func changedFiles(repositoryRootPath: String) throws -> [String] {
        var files = collectChangedFiles(arguments: ["diff", "--name-only", "--", "."], repositoryRootPath: repositoryRootPath)
        files.append(contentsOf: collectChangedFiles(arguments: ["diff", "--cached", "--name-only", "--", "."], repositoryRootPath: repositoryRootPath))

        if files.isEmpty {
            files = collectChangedFiles(arguments: ["diff", "--name-only", "HEAD~1", "--", "."], repositoryRootPath: repositoryRootPath)
        }
        if files.isEmpty {
            files = collectChangedFiles(arguments: ["show", "--pretty=", "--name-only", "HEAD", "--", "."], repositoryRootPath: repositoryRootPath)
        }

        return Array(Set(files)).sorted()
    }

    private static func codeReviewPrompt() -> String {
        """
        Review the repository changes described in Input.collect_diff.stdout.
        Focus on correctness bugs, regressions, risky assumptions, and missing tests.
        The input includes file summaries and a bounded patch preview.
        If the diff is truncated or seems insufficient, say that explicitly and focus on the highest-risk observations you can support.
        Keep the answer concise and actionable.
        """
    }

    private static func changeSummaryPrompt() -> String {
        """
        Summarize the repository changes described in Input.collect_diff.stdout.
        Write a concise human update covering what changed, why it matters, and what to verify next.
        The input includes file summaries and a bounded patch preview.
        If the diff is truncated or seems insufficient, say that explicitly and avoid pretending to know more than the evidence supports.
        """
    }

    private static func testPlanPrompt() -> String {
        """
        Outline the highest-value automated and manual checks for the repository changes described in Input.collect_diff.stdout.
        Focus on regression risks, missing verification, and the smallest set of checks that would increase confidence.
        The input includes file summaries and a bounded patch preview.
        If the diff is truncated or seems insufficient, say that explicitly and avoid pretending to know more than the evidence supports.
        """
    }

    private static func codeReviewScript(repositoryRootPath: String) -> String {
        let repo = shellQuote(repositoryRootPath)
        return """
        set -eu
        cd \(repo)
        PATCH_LIMIT=220
        print_limited_git_output() {
          "$@" | {
            count=0
            while IFS= read -r line; do
              printf '%s\n' "$line"
              count=$((count + 1))
              if [ "$count" -ge "$PATCH_LIMIT" ]; then
                printf '\n--- PATCH TRUNCATED AFTER %s LINES ---\n' "$PATCH_LIMIT"
                break
              fi
            done
          }
        }
        if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
          echo "starter code-review requires a git repository" >&2
          exit 1
        fi
        if ! git diff --quiet -- . || ! git diff --cached --quiet -- .; then
          if ! git diff --quiet -- .; then
            printf '%s\n' '--- FILE SUMMARY ---'
            git --no-pager diff --stat -- . || true
            printf '\n%s\n' '--- PATCH (first 220 lines) ---'
            print_limited_git_output git --no-pager diff --unified=3 -- .
          fi
          if ! git diff --cached --quiet -- .; then
            printf '\n%s\n' '--- STAGED FILE SUMMARY ---'
            git --no-pager diff --cached --stat -- . || true
            printf '\n%s\n' '--- STAGED PATCH (first 220 lines) ---'
            print_limited_git_output git --no-pager diff --cached --unified=3 -- .
          fi
        elif git rev-parse --verify HEAD~1 >/dev/null 2>&1; then
          printf '%s\n' '--- FILE SUMMARY ---'
          git --no-pager diff --stat HEAD~1 -- .
          printf '\n%s\n' '--- PATCH (first 220 lines) ---'
          print_limited_git_output git --no-pager diff --unified=3 HEAD~1 -- .
        elif git rev-parse --verify HEAD >/dev/null 2>&1; then
          printf '%s\n' '--- FILE SUMMARY ---'
          git --no-pager show --stat --format=medium HEAD -- .
          printf '\n%s\n' '--- PATCH (first 220 lines) ---'
          print_limited_git_output git --no-pager show --patch --format=medium HEAD -- .
        else
          echo "No commits or working tree changes available to review."
        fi
        """
    }

    private static func codeReviewSandbox(repositoryRootPath: String) -> [String: Any] {
        let gitPath = findCommandInPath("git") ?? "/usr/bin/git"
        return [
            "scope_root": repositoryRootPath,
            "allow_read": [repositoryRootPath],
            "allow_write": [],
            "allow_commands": ["/bin/sh", "/bin/bash", gitPath],
            "allow_network": [],
            "deny_network": ["*"],
            "tmp_write": [],
            "cache_write": [],
        ]
    }

    private static func shellQuote(_ value: String) -> String {
        "'\(value.replacingOccurrences(of: "'", with: "'\"'\"'"))'"
    }

    private static func findCommandInPath(_ command: String) -> String? {
        guard let path = ProcessInfo.processInfo.environment["PATH"] else { return nil }
        for directory in path.split(separator: ":") {
            let candidate = String(directory) + "/" + command
            if FileManager.default.isExecutableFile(atPath: candidate) {
                return candidate
            }
        }
        return nil
    }

    private static func preset(id: String) -> WorkflowStarterPreset {
        availablePresets.first(where: { $0.id == id }) ?? availablePresets[0]
    }

    static func changedFilesHint(forChangedFiles changedFiles: [String]) -> String {
        let unique = Array(Set(changedFiles)).sorted()
        guard !unique.isEmpty else {
            return "Changed files: No active diff detected."
        }

        let preview = unique.prefix(2).joined(separator: ", ")
        let overflow = unique.count - min(unique.count, 2)
        if overflow > 0 {
            return "Changed files: \(preview) (+\(overflow) more)"
        }
        return "Changed files: \(preview)"
    }

    private static func collectChangedFiles(arguments: [String], repositoryRootPath: String) -> [String] {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
        process.arguments = ["git", "-C", repositoryRootPath] + arguments
        let output = Pipe()
        process.standardOutput = output
        process.standardError = nil

        do {
            try process.run()
        } catch {
            return []
        }
        process.waitUntilExit()
        guard process.terminationStatus == 0 else { return [] }
        let data = output.fileHandleForReading.readDataToEndOfFile()
        let text = String(data: data, encoding: .utf8) ?? ""
        return text
            .split(separator: "\n")
            .map { String($0).trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
    }

    private static func isDocsLikePath(_ path: String) -> Bool {
        let lowercase = path.lowercased()
        return lowercase.hasPrefix("docs/")
            || lowercase.hasSuffix(".md")
            || lowercase.hasSuffix(".mdx")
            || lowercase.hasSuffix(".txt")
            || lowercase.hasSuffix(".rst")
            || lowercase.contains("readme")
            || lowercase.contains("changelog")
    }

    private static func isTestLikePath(_ path: String) -> Bool {
        let lowercase = path.lowercased()
        return lowercase.contains("/test")
            || lowercase.contains("/tests")
            || lowercase.contains("_test.")
            || lowercase.contains(".test.")
            || lowercase.contains(".spec.")
            || lowercase.hasSuffix(".feature")
    }

    private static func isCodeLikePath(_ path: String) -> Bool {
        let lowercase = path.lowercased()
        return [".rs", ".swift", ".cs", ".ts", ".tsx", ".js", ".jsx", ".py", ".java", ".kt", ".go", ".rb", ".php"]
            .contains { lowercase.hasSuffix($0) }
    }
}
