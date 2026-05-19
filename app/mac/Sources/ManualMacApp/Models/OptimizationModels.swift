import Foundation

// See docs/wiki/systems/매뉴얼-최적화-기능.md: mac UI should expose
// workflow-specific optimization evidence derived from stored run history.
struct WorkflowOptimizationReport: Equatable, Sendable {
    let sections: [String]
    let mainIssue: String
    let recommendations: [String]
    let measurementMode: String
    let measurementNote: String

    init(_ result: Any) throws {
        guard
            let object = result as? [String: Any],
            let sections = object["sections"] as? [String],
            let mainIssue = object["main_issue"] as? String,
            let recommendations = object["recommendations"] as? [String]
        else {
            throw AppServerClientError.invalidResponse
        }

        self.sections = sections
        self.mainIssue = mainIssue
        self.recommendations = recommendations
        self.measurementMode = object["measurement_mode"] as? String ?? "unknown"
        self.measurementNote = object["measurement_note"] as? String ?? "Measurement provenance unavailable."
    }
}

struct WorkflowOptimizationAnalysis: Equatable, Sendable {
    let regressionPossible: Bool
    let regressionStepID: String?
    let regressionReason: String
    let tokenWasteSteps: [String]
    let verificationGapSteps: [String]
    let slowSteps: [String]
    let unstableSteps: [String]

    init(_ result: Any) throws {
        guard
            let object = result as? [String: Any],
            let regression = object["regression"] as? [String: Any],
            let bottlenecks = object["bottlenecks"] as? [String: Any]
        else {
            throw AppServerClientError.invalidResponse
        }

        self.regressionPossible = regression["possible"] as? Bool ?? false
        self.regressionStepID = regression["step_id"] as? String
        self.regressionReason = regression["reason"] as? String ?? ""
        self.tokenWasteSteps = bottlenecks["token_waste"] as? [String] ?? []
        self.verificationGapSteps = bottlenecks["verification_gaps"] as? [String] ?? []
        self.slowSteps = bottlenecks["slow_steps"] as? [String] ?? []
        self.unstableSteps = bottlenecks["unstable_tasks"] as? [String] ?? []
    }
}

struct WorkflowOptimizationHeadline: Equatable, Sendable {
    let title: String
    let detail: String
    let measurementLabel: String
    let isRegression: Bool

    init(report: WorkflowOptimizationReport?, analysis: WorkflowOptimizationAnalysis?) {
        if let report {
            self.title = analysis?.regressionPossible == true ? "Regression risk" : "Optimization insight"
            self.detail = report.mainIssue
            self.measurementLabel = report.measurementMode.capitalized
            self.isRegression = analysis?.regressionPossible == true
        } else {
            self.title = "Optimization pending"
            self.detail = "Run the workflow to capture optimization insight."
            self.measurementLabel = "Pending"
            self.isRegression = false
        }
    }
}
