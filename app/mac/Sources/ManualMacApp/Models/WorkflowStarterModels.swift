import Foundation

// See docs/wiki/features/workflow-starters.md: mac onboarding should be able to
// create the same first-success starter workflows without sending users back to CLI docs.
enum WorkflowStarterError: Error, LocalizedError {
    case unsupportedAgent(String)
    case noAvailableAgent
    case notGitRepository(String)
    case gitCommandFailed(String)

    var errorDescription: String? {
        switch self {
        case let .unsupportedAgent(agent):
            "Unsupported starter agent: \(agent)"
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

enum WorkflowStarterDefinition {
    static func suggestedWorkflowID(repositoryRootPath: String) -> String {
        let name = URL(fileURLWithPath: repositoryRootPath, isDirectory: true)
            .lastPathComponent
        let normalized = name
            .lowercased()
            .map { character -> Character in
                character.isLetter || character.isNumber ? character : "-"
            }
        let collapsed = String(normalized)
            .replacingOccurrences(of: "-+", with: "-", options: .regularExpression)
            .trimmingCharacters(in: CharacterSet(charactersIn: "-"))
        return "starter-\(collapsed.isEmpty ? "repo" : collapsed)-review"
    }

    static func preferredAgent(from agents: [AppServerAgentAvailability]) -> String? {
        for candidate in ["codex", "claude", "pi"] {
            if agents.contains(where: { $0.name == candidate && $0.available }) {
                return candidate
            }
        }
        return nil
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

    private static func codeReviewPrompt() -> String {
        """
        Review the repository changes described in Input.collect_diff.stdout.
        Focus on correctness bugs, regressions, risky assumptions, and missing tests.
        The input includes file summaries and a bounded patch preview.
        If the diff is truncated or seems insufficient, say that explicitly and focus on the highest-risk observations you can support.
        Keep the answer concise and actionable.
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
}
