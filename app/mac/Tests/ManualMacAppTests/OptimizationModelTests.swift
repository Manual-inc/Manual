import Testing
@testable import ManualMacApp

@Suite("Optimization Models")
struct OptimizationModelTests {
    @Test func parsesOptimizationReportAndAnalysis() throws {
        let report = try WorkflowOptimizationReport([
            "sections": ["Token Usage", "Verification", "Time"],
            "main_issue": "implementation step used most tokens",
            "recommendations": ["preprocess file discovery", "add verification checklist"],
            "measurement_mode": "derived",
            "measurement_note": "Estimated from workflow events.",
        ])
        let analysis = try WorkflowOptimizationAnalysis([
            "regression": [
                "possible": true,
                "step_id": "implement",
                "reason": "tokens and time increased while success rate fell",
            ],
            "bottlenecks": [
                "token_waste": ["implement"],
                "verification_gaps": ["review"],
                "slow_steps": ["implement"],
                "unstable_tasks": ["implement"],
            ],
        ])

        #expect(report.sections == ["Token Usage", "Verification", "Time"])
        #expect(report.mainIssue == "implementation step used most tokens")
        #expect(report.recommendations.count == 2)
        #expect(report.measurementMode == "derived")
        #expect(report.measurementNote == "Estimated from workflow events.")
        #expect(analysis.regressionPossible)
        #expect(analysis.regressionStepID == "implement")
        #expect(analysis.tokenWasteSteps.first == "implement")
        #expect(analysis.verificationGapSteps.first == "review")
    }

    @Test func optimizationHeadline_prioritizes_regression_and_provenance() throws {
        let report = try WorkflowOptimizationReport([
            "sections": ["Token Usage", "Verification", "Time"],
            "main_issue": "implementation step used most tokens",
            "recommendations": ["preprocess file discovery"],
            "measurement_mode": "derived",
            "measurement_note": "Estimated from workflow events.",
        ])
        let analysis = try WorkflowOptimizationAnalysis([
            "regression": [
                "possible": true,
                "step_id": "implement",
                "reason": "tokens and time increased while success rate fell",
            ],
            "bottlenecks": [
                "token_waste": ["implement"],
                "verification_gaps": ["review"],
                "slow_steps": ["implement"],
                "unstable_tasks": ["implement"],
            ],
        ])

        let headline = WorkflowOptimizationHeadline(report: report, analysis: analysis)

        #expect(headline.title == "Regression risk")
        #expect(headline.detail == "implementation step used most tokens")
        #expect(headline.measurementLabel == "Derived")
        #expect(headline.isRegression)
    }

    @Test func workflow_title_from_identifier_is_human_readable() {
        #expect(humanizeIdentifier("demo-optimization") == "Demo Optimization")
        #expect(humanizeIdentifier("business-pipeline-health") == "Business Pipeline Health")
        #expect(humanizeIdentifier("digest") == "Digest")
    }
}
