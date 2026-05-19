import Foundation

final class ManualWorld {
    // See docs/wiki/systems/기능-계약-테스트.md: scenario state keeps app-server evidence beside UI-facing assertions.
    let appServer = AppServerScenarioDriver()

    // MARK: 매뉴얼

    enum ManualStatus { case draft, active, archived, deleted, running }
    enum EvolutionMode { case suggest, assistedEdit, autoImprove }

    struct Sandbox {
        var name: String
        var readPaths: [String] = []
        var writePaths: [String] = []
        var allowedCommands: [String] = []
        var allowedHosts: [String] = []
        var deniedCommands: [String] = []
        var deniedHosts: [String] = []
        var environmentScope: [String] = []
        var tempDirs: [String] = []
        var cacheDirs: [String] = []
        var history: [String] = []
    }

    struct WorkflowNode {
        let id: String
        var kind: String // "agent" | "script" | "context" | ...
        var sandboxName: String?
        var skills: [String] = []
        var skillCandidates: [String] = []
        var inputSchema: [String: Bool] = [:] // field -> required
        var outputSchema: [String: String] = [:] // field -> format
        var validationCriteria: [String] = []
        var taskKind: String?
        var deterministic: Bool?
        var inferenceDepth: String?
        var failureCost: String?
        var verifiability: String?
        var inputSize: String?
        var reuseLikelihood: String?
        var usesHighEndLLM: Bool = false
        var highEndLLMReason: String?
        var tokenBudget: Int?
        var importance: String?
        var testCases: [NodeTestCase] = []
    }

    struct NodeTestCase {
        let input: String
        let expectedOutput: String
        let criteria: String
    }

    struct Manual {
        var id: String
        var name: String
        var purpose: String = ""
        var description: String = ""
        var status: ManualStatus = .draft
        var createdAt: Date = .init()
        var updatedAt: Date = .init()
        var version: Int = 1
        var versionHistory: [String] = ["v1"]
        var defaultAgent: String?
        var modelSettings: String = "default"
        var nodes: [WorkflowNode] = []
        var sandboxNames: [String] = []
        var tokenBudgets: [String: Int] = [:]
        var verificationCriteria: [String] = []
        var measureTokens: Bool = false
        var measureTime: Bool = false
        var verificationStandard: String?
        var suggestImprovements: Bool = false
        var evolutionMode: EvolutionMode?
        var tags: [String] = []
        var changeHistory: [String] = []
        var executionRecords: [ExecutionRecord] = []
    }

    struct ExecutionRecord {
        var runID: String
        var manualID: String
        var stages: [StageRun]
        var totalTokens: Int = 0
        var totalDurationMs: Int = 0
        var success: Bool = true
        var retries: Int = 0
        var modelCalls: [String] = []
        var toolCalls: [String] = []
        var contextSummaries: [String] = []
        var executionMode: String = "auto"
        var baseline: Bool = false
        var parentRunID: String?
        var verificationPassRate: Double = 1.0
        var cost: Double = 0
    }

    struct StageRun {
        var nodeID: String
        var status: String // pending|running|succeeded|failed|skipped|aborted
        var input: String = ""
        var output: String = ""
        var log: String = ""
        var durationMs: Int = 0
        var tokens: Int = 0
        var model: String = ""
        var skillsUsed: [String] = []
        var observedSkills: [String] = []
        var validations: [String: String] = [:] // criterion -> pass|fail|unknown
        var evidence: [String: String] = [:]
        var verified: Bool = true
    }

    // MARK: 상태

    var installedAgents: Set<String> = ["claude", "codex", "pi"]
    var detectedAgents: Set<String> = ["claude", "codex", "pi"]
    var sandboxes: [String: Sandbox] = [:]
    var manuals: [String: Manual] = [:]
    var currentManualID: String?
    var currentNodeID: String?
    var currentSandboxName: String?
    var currentRunID: String?
    var currentWorkflowID: String?
    var lastResponse: [String: Any]?
    var latestNodeList: [String: Any]?
    var latestSchema: [String: Any]?
    var nodeRunID: String?
    var nodeEvents: [String: Any]?
    var nodeTestCaseID: String?
    var macAppUI: MacAppUIDriver?
    var workflowRunIDsBeforeUIAction: Set<String> = []

    var lastError: String?
    var lastBlockedReason: String?
    var lastBlockedTarget: String?
    var lastSaveSucceeded: Bool = true
    var pendingInputs: [String: String] = [:]
    var providedInputs: [String: String] = [:]
    var inputDiff: String?
    var executionModeOffered: Set<String> = []
    var manualActivationBlocked: Bool = false
    var manualActivationReason: String?
    var rollbackAvailable: Bool = false
    var improvementCandidates: [String] = []
    var pendingImprovement: String?
    var appliedImprovements: [String] = []
    var lowRiskTargets: Set<String> = ["파일 목록 캐싱", "로그 정리", "요약 입력 생성"]

    // Sandbox runtime
    var executionPlatform: String = "macOS"
    var sandboxWrapped: Bool = false
    var sandboxBackend: String?
    var lastNodeExecutionStopped: Bool = false
    var blockedAccessHistory: [String] = []
    var policyViolations: [String] = []

    // Storybook
    var nodeRegistry: [String: WorkflowNode] = [:]
    var lastNodeRunResult: StageRun?
    var lastNodeTestRun: [String: String] = [:]

    // Skill verification
    var skillVerificationResults: [String: String] = [:] // skill -> verified|unverified|mismatch
    var agentSkillSupport: [String: String] = [
        "claude": "skill-flag",
        "codex": "instruction-block",
        "pi": "unknown",
    ]

    // Optimization
    var optimizationReports: [String: String] = [:]
    var bottlenecks: [String] = []
    var preprocessingProposals: [String] = []
    var modelChangeProposals: [String] = []
    var costRegression: Bool = false
    var modelComparison: [String: Double] = [:]
    var comparisonReport: String?

    // Self-evolution
    var baselineRun: ExecutionRecord?
    var suggestedChanges: [String] = []
    var verificationDecreasing: Bool = false
    var autoApplyBlocked: Bool = false
    var manualEvolutionHistory: [String: [String]] = [:]

    // MARK: 헬퍼

    func newManualID() -> String {
        "manual-\(manuals.count + 1)"
    }

    func currentManual() -> Manual? {
        guard let id = currentManualID else { return nil }
        return manuals[id]
    }

    func updateCurrentManual(_ mutate: (inout Manual) -> Void) {
        guard let id = currentManualID, var manual = manuals[id] else { return }
        mutate(&manual)
        manual.updatedAt = Date()
        manuals[id] = manual
    }

    func currentNode() -> WorkflowNode? {
        guard
            let manualID = currentManualID,
            let manual = manuals[manualID],
            let nodeID = currentNodeID
        else { return nil }
        return manual.nodes.first { $0.id == nodeID }
    }

    func updateCurrentNode(_ mutate: (inout WorkflowNode) -> Void) {
        guard
            let manualID = currentManualID,
            var manual = manuals[manualID],
            let nodeID = currentNodeID,
            let index = manual.nodes.firstIndex(where: { $0.id == nodeID })
        else { return }
        mutate(&manual.nodes[index])
        manual.updatedAt = Date()
        manuals[manualID] = manual
    }
}
