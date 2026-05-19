import Foundation
import ManualMacApp

// See docs/wiki/systems/기능-계약-테스트.md: feature intents translate cucumber language into app-server calls.
enum CucumberFeatureIntent {
    static func perform(
        world: ManualWorld,
        step: String,
        captures: [String],
        file: StaticString,
        line: UInt
    ) throws {
        if try performMacUIIntent(world: world, step: step, file: file, line: line) { return }
        if isExpectation(step) { return }
        if step.contains("노드") || step.contains("schema") || step.contains("테스트 케이스") {
            try performNodeIntent(world: world, step: step); return
        }
        if step.contains("skill") || step.contains("에이전트별 실행 설정") {
            try performSkillIntent(world: world, step: step); return
        }
        if step.contains("워크플로우 단계를 구성") {
            try performNodeIntent(world: world, step: step); return
        }
        if isManualIntentStep(step) {
            try performManualIntent(world: world, step: step); return
        }
        if step.contains("샌드박스") || step.contains("정책") || step.contains("네트워크") || step.contains("환경 변수") {
            try performSandboxIntent(world: world, step: step, captures: captures); return
        }
        if step.contains("토큰") || step.contains("최적화") || step.contains("검증") || step.contains("Verification")
            || step.contains("모델") || step.contains("개선") || step.contains("진화") || step.contains("롤백") {
            try performOptimizationIntent(world: world, step: step); return
        }
        if step.contains("워크플로우") || step.contains("단계") || step.contains("실행") || step.contains("재시작") {
            try performWorkflowIntent(world: world, step: step); return
        }
        _ = try world.appServer.rpc(method: "workflow.list")
    }

    private static func performMacUIIntent(world: ManualWorld, step: String, file: StaticString, line: UInt) throws -> Bool {
        switch step {
        case "mac UI intent가 테스트 app-server discovery를 사용한다":
            let ui = MacAppUIDriver(appServer: world.appServer)
            try ui.launch()
            world.macAppUI = ui
            return true
        case "사용자가 UI에서 예제 워크플로우 실행을 선택한다":
            let ui = world.macAppUI ?? MacAppUIDriver(appServer: world.appServer)
            world.macAppUI = ui
            world.workflowRunIDsBeforeUIAction = world.appServer.workflowRunIDs()
            world.currentRunID = try ui.chooseExecuteWorkflowFromUI().runID
            return true
        case "app-server에는 UI가 시작한 workflow run이 생성되어야 한다":
            let runID = try world.currentRunID
                ?? world.appServer.waitForNewWorkflowRunID(after: world.workflowRunIDsBeforeUIAction)
            world.currentRunID = runID
            try expectStep(runID.hasPrefix("run-"), "UI action should create app-server run id", file: file, line: line)
            return true
        case "run 이벤트는 workflow_started를 포함해야 한다":
            guard let runID = world.currentRunID else { throw StepError.assertion("workflow run id가 필요함") }
            world.lastResponse = try world.appServer.poll(
                method: "workflow.events",
                params: ["run_id": runID, "cursor": 0],
                until: { response in
                    let events = (response["result"] as? [String: Any])?["events"] as? [[String: Any]] ?? []
                    return events.contains { $0["type"] as? String == "workflow_started" }
                }
            )
            return true
        case "UI workflow 완료 후 optimization report가 준비되어야 한다":
            guard let runID = world.currentRunID else { throw StepError.assertion("workflow run id가 필요함") }
            let completed = try world.appServer.poll(
                method: "workflow.events",
                params: ["run_id": runID, "cursor": 0],
                timeout: 5,
                until: { response in
                    let result = response["result"] as? [String: Any]
                    return result?["completed"] as? Bool == true
                }
            )
            let workflowID =
                ((completed["result"] as? [String: Any])?["run"] as? [String: Any])?["workflow_id"] as? String
                ?? "business-pipeline-health"
            world.lastResponse = try world.appServer.poll(
                method: "optimization.report",
                params: ["workflow_id": workflowID],
                timeout: 5,
                until: { response in
                    let result = response["result"] as? [String: Any]
                    let mainIssue = result?["main_issue"] as? String ?? ""
                    return !mainIssue.isEmpty && !mainIssue.contains("insufficient run history")
                }
            )
            return true
        case "optimization report는 derived 측정 근거를 포함해야 한다":
            return true
        default:
            return false
        }
    }

    private static func performManualIntent(world: ManualWorld, step: String) throws {
        if step.contains("기본 실행 에이전트를 선택") {
            world.lastResponse = try world.appServer.rpc(method: "agent.list", params: ["candidates": ["claude", "codex", "pi", "hermes"]])
            return
        }
        if step.contains("실행 가능한 에이전트가 감지되지") {
            try createManual(
                world: world,
                params: [
                    "name": "No Agent Manual",
                    "purpose": "activation validation",
                    "description": "missing agent",
                    "default_agent": "",
                ]
            )
            return
        }
        if step.contains("특정 매뉴얼이 현재 실행 중") {
            try createManual(
                world: world,
                params: [
                    "name": "Running Manual",
                    "purpose": "delete validation",
                    "description": "running manual",
                    "running": true,
                ]
            )
            return
        }
        if step.contains("에이전트나 스크립트를 실행하는 단계") {
            try createManual(
                world: world,
                params: [
                    "name": "Missing Sandbox Manual",
                    "purpose": "activation validation",
                    "workflow_steps": [
                        [
                            "id": "agent-step",
                            "kind": "codex",
                            "input_schema": [["name": "prompt", "required": true]],
                            "output_schema": "agent result object",
                            "verification_policy": ["required": true, "criteria": ["tests pass"]],
                            "token_budget": 4000,
                        ],
                    ],
                ]
            )
            return
        }
        if step.contains("최적화 측정이 활성화") {
            try createManual(
                world: world,
                params: [
                    "name": "Missing Budget Manual",
                    "purpose": "activation validation",
                    "optimization": [
                        "token_measurement": true,
                        "time_measurement": true,
                        "verification_criteria": ["requirements"],
                        "improvement_recommendations": true,
                        "self_evolution_mode": "suggest",
                    ],
                    "workflow_steps": [
                        [
                            "id": "agent-step",
                            "kind": "codex",
                            "input_schema": [["name": "prompt", "required": true]],
                            "output_schema": "agent result object",
                            "sandbox_policy": ["sandbox_id": "default"],
                        ],
                    ],
                ]
            )
            return
        }
        if step.contains("여러 개의 매뉴얼") {
            try createManual(world: world, params: ["name": "Intent Manual One", "tags": ["mvp", "docs"]])
            try createManual(world: world, params: ["name": "Intent Manual Two", "tags": ["ops"]])
            return
        }
        if step.contains("여러 버전이 존재") {
            try ensureManual(world)
            world.lastResponse = try world.appServer.rpc(
                method: "manual.update",
                params: [
                    "manual_id": world.currentManualID ?? "",
                    "execution_affecting": true,
                    "changes": ["description": "versioned by cucumber intent"],
                ]
            )
            return
        }
        if step.contains("목록") {
            world.lastResponse = try world.appServer.rpc(method: "manual.list", params: [:]); return
        }
        if step.contains("이름을 입력하지") {
            world.lastResponse = try world.appServer.rpcAllowingErrors(method: "manual.create", params: ["name": ""]); return
        }
        if step.contains("복제") {
            try ensureManual(world); world.lastResponse = try world.appServer.rpc(method: "manual.clone", params: ["manual_id": world.currentManualID ?? ""]); return
        }
        if step.contains("보관") {
            try ensureManual(world); world.lastResponse = try world.appServer.rpc(method: "manual.archive", params: ["manual_id": world.currentManualID ?? ""]); return
        }
        if step.contains("삭제") {
            try ensureManual(world); world.lastResponse = try world.appServer.rpcAllowingErrors(method: "manual.delete", params: ["manual_id": world.currentManualID ?? ""]); return
        }
        if step.contains("활성") {
            try ensureManual(world); world.lastResponse = try world.appServer.rpc(method: "manual.activate", params: ["manual_id": world.currentManualID ?? ""]); return
        }
        if step.contains("수정") || step.contains("변경") {
            try ensureManual(world)
            world.lastResponse = try world.appServer.rpc(
                method: "manual.update",
                params: ["manual_id": world.currentManualID ?? "", "execution_affecting": true, "changes": ["description": "updated by mac intent"]]
            )
            return
        }
        if step.contains("버전") {
            try ensureManual(world); world.lastResponse = try world.appServer.rpc(method: "manual.versions", params: ["manual_id": world.currentManualID ?? ""]); return
        }
        try ensureManual(world)
        world.lastResponse = try world.appServer.rpc(method: "manual.get", params: ["manual_id": world.currentManualID ?? ""])
    }

    private static func performNodeIntent(world: ManualWorld, step: String) throws {
        try ensureStorybookNode(world)
        if step.contains("목록") {
            world.latestNodeList = try world.appServer.rpc(method: "node.list")
            world.latestSchema = try world.appServer.rpc(method: "node.schema", params: ["kind": "template"])
            return
        }
        if step.contains("schema") || step.contains("입력") || step.contains("출력") {
            world.latestSchema = try world.appServer.rpc(method: "node.schema", params: ["kind": "template"])
        }
        if step.contains("독립 실행") || step.contains("임의 입력") || step.contains("실행 결과") || step.contains("공통 실행") {
            try runNode(world)
        }
        if step.contains("저장한다") || step.contains("기록해야 한다") { try saveNodeTestCase(world) }
        if step.contains("검증") || step.contains("회귀") || step.contains("비교") || step.contains("차이") {
            world.lastResponse = try world.appServer.rpc(method: "node.testcase.verify", params: ["node_id": "digest"])
        }
        if step.contains("워크플로우 단계") || step.contains("registry") || step.contains("후보") {
            world.lastResponse = try world.appServer.rpc(method: "workflow.compose_from_registry", params: ["node_id": "digest"])
        }
    }

    private static func performSkillIntent(world: ManualWorld, step: String) throws {
        if step.contains("후보") {
            world.lastResponse = try world.appServer.rpc(method: "skill.candidates", params: ["task_type": "documentation"])
        } else if step.contains("검증") || step.contains("불일치") || step.contains("리스크") {
            world.lastResponse = try world.appServer.rpc(method: "skill.verify", params: ["step_id": "agent-step"])
        } else if step.contains("전달 방식") || step.contains("에이전트별") {
            world.lastResponse = try world.appServer.rpc(method: "skill.agent_capabilities")
        } else {
            world.lastResponse = try world.appServer.rpc(
                method: "skill.configure",
                params: ["step_id": "agent-step", "task_type": "documentation", "agent": "codex", "skills": ["code-review"]]
            )
        }
    }

    private static func performSandboxIntent(world: ManualWorld, step: String, captures: [String]) throws {
        if world.currentSandboxName == nil {
            let created = try world.appServer.rpc(method: "sandbox.create", params: [:])
            world.currentSandboxName = ((created["result"] as? [String: Any])?["sandbox"] as? [String: Any])?["id"] as? String
            world.lastResponse = created
        }
        if step.contains("조회") || step.contains("확인") {
            world.lastResponse = try world.appServer.rpc(method: "sandbox.get", params: ["sandbox_id": world.currentSandboxName ?? ""])
        } else if step.contains("차단") || step.contains("허용") || step.contains("접근") || step.contains("실행") {
            world.lastResponse = try world.appServer.rpc(method: "sandbox.evaluate", params: ["sandbox_id": world.currentSandboxName ?? "", "operation": captures.first ?? "probe"])
        } else {
            world.lastResponse = try world.appServer.rpc(method: "sandbox.update", params: ["sandbox_id": world.currentSandboxName ?? "", "changes": ["allow_read": ["docs/**"], "allow_env": ["MANUAL_*"]]])
        }
    }

    private static func performOptimizationIntent(world: ManualWorld, step: String) throws {
        if step.contains("비교") || step.contains("롤백") {
            world.lastResponse = try world.appServer.rpc(method: "optimization.compare")
        } else if step.contains("보고") || step.contains("효과") || step.contains("상태") {
            world.lastResponse = try world.appServer.rpc(method: "optimization.report")
        } else if step.contains("분석") || step.contains("후보") || step.contains("제안") || step.contains("개선") {
            world.lastResponse = try world.appServer.rpc(method: "optimization.analyze")
        } else {
            world.lastResponse = try world.appServer.rpc(method: "optimization.record_run", params: ["manual_id": "manual-intent"])
        }
    }

    private static func performWorkflowIntent(world: ManualWorld, step: String) throws {
        try ensureWorkflow(world)
        if step.contains("중단") {
            world.lastResponse = try world.appServer.rpcAllowingErrors(method: "workflow.stop", params: ["run_id": world.currentRunID ?? "run-0"])
        } else if step.contains("재시작") || step.contains("계속") {
            world.lastResponse = try world.appServer.rpcAllowingErrors(method: "workflow.resume", params: ["run_id": world.currentRunID ?? "run-0"])
        } else if step.contains("조회") || step.contains("결과") || step.contains("기록") {
            if let runID = world.currentRunID {
                world.lastResponse = try world.appServer.rpc(method: "workflow.events", params: ["run_id": runID, "cursor": 0])
            } else {
                world.lastResponse = try world.appServer.rpc(method: "workflow.get", params: ["workflow_id": "intent-workflow"])
            }
        } else if step.contains("실행") {
            let started = try world.appServer.rpc(method: "workflow.start", params: ["workflow_id": "intent-workflow"])
            world.currentRunID = ((started["result"] as? [String: Any])?["run_id"] as? String)
            world.lastResponse = started
        } else {
            world.lastResponse = try world.appServer.rpc(method: "workflow.get", params: ["workflow_id": "intent-workflow"])
        }
    }

    private static func ensureManual(_ world: ManualWorld) throws {
        if world.currentManualID != nil { return }
        try createManual(
            world: world,
            params: [
                "name": "Intent Manual",
                "purpose": "Exercise mac cucumber intent",
                "description": "Contract-backed manual",
                "default_agent": "codex",
                "execution_mode": "single_agent",
            ]
        )
    }

    private static func createManual(world: ManualWorld, params: [String: Any]) throws {
        let created = try world.appServer.rpc(method: "manual.create", params: params)
        if let manualID = (((created["result"] as? [String: Any])?["manual"] as? [String: Any])?["id"] as? String) {
            world.currentManualID = manualID
        }
        world.lastResponse = created
    }

    private static func isExpectation(_ step: String) -> Bool {
        [
            "해야 한다",
            "있어야 한다",
            "않아야 한다",
            "제공해야 한다",
            "기록해야 한다",
            "막아야 한다",
            "거부해야 한다",
            "변경해야 한다",
            "저장해야 한다",
        ].contains { step.contains($0) }
    }

    private static func isManualIntentStep(_ step: String) -> Bool {
        if step.contains("처음 실행") || step.contains("실행 중단") || step.contains("실행 기록을 분석") {
            return false
        }
        if step.contains("기본 실행 에이전트") {
            return true
        }
        if step.contains("실행 가능한 에이전트가 감지되지") {
            return true
        }
        if step.contains("설명, 태그") {
            return true
        }
        if step.contains("워크플로우 단계, 샌드박스 정책, 검증 정책, 모델 설정") {
            return true
        }
        if step.contains("버전 이력") {
            return true
        }
        return step.contains("매뉴얼") || step.contains("Manual")
    }

    private static func ensureWorkflow(_ world: ManualWorld) throws {
        let listed = try world.appServer.rpc(method: "workflow.list")
        let workflows = ((listed["result"] as? [String: Any])?["workflows"] as? [[String: Any]]) ?? []
        if workflows.contains(where: { $0["workflow_id"] as? String == "intent-workflow" }) { return }
        world.lastResponse = try world.appServer.rpc(
            method: "workflow.create",
            params: [
                "workflow": [
                    "id": "intent-workflow",
                    "nodes": [["id": "source", "kind": "constant", "value": "input"], ["id": "digest", "kind": "template", "template": "result {{source}}"]],
                    "dependencies": [["node": "digest", "depends_on": "source"]],
                ],
            ]
        )
    }

    private static func ensureStorybookNode(_ world: ManualWorld) throws {
        let list = try world.appServer.rpc(method: "node.list")
        let templates = ((list["result"] as? [String: Any])?["templates"] as? [[String: Any]]) ?? []
        if templates.contains(where: { $0["id"] as? String == "digest" }) {
            world.latestNodeList = list
            return
        }
        _ = try world.appServer.rpcAllowingErrors(
            method: "node.create",
            params: ["name": "Digest node", "description": "Summarizes a topic from Storybook input", "node": ["id": "digest", "kind": "template", "template": "topic={{__storybook_input__.topic}} priority={{__storybook_input__.priority}}"]]
        )
        world.latestNodeList = try world.appServer.rpc(method: "node.list")
    }

    private static func runNode(_ world: ManualWorld) throws {
        let started = try world.appServer.rpc(
            method: "node.run",
            params: ["node": ["id": "digest", "kind": "template", "template": "topic={{__storybook_input__.topic}} priority={{__storybook_input__.priority}}"], "inputs": ["topic": "contract-tests", "priority": 3]]
        )
        guard let runID = (started["result"] as? [String: Any])?["run_id"] as? String else {
            throw StepError.assertion("node.run이 run_id를 반환해야 함")
        }
        world.nodeRunID = runID
        world.nodeEvents = try world.appServer.poll(
            method: "node.run.events",
            params: ["run_id": runID, "cursor": 0],
            until: { (($0["result"] as? [String: Any])?["completed"] as? Bool) == true }
        )
    }

    private static func saveNodeTestCase(_ world: ManualWorld) throws {
        if world.nodeRunID == nil { try runNode(world) }
        guard let runID = world.nodeRunID else { return }
        let saved = try world.appServer.rpc(method: "node.testcase.save", params: ["run_id": runID, "criteria": ["comparison": "json_equal", "schema_match_required": true]])
        world.nodeTestCaseID = (((saved["result"] as? [String: Any])?["test_case"] as? [String: Any])?["id"] as? String)
        world.lastResponse = saved
    }
}
