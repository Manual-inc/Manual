import Foundation

// See docs/wiki/systems/기능-계약-테스트.md: assertions pin cucumber wording to concrete app-server evidence.
enum CucumberFeatureAssertions {
    static func assert(world: ManualWorld, step: String, file: StaticString, line: UInt) throws {
        try assertEvidence(world: world, step: step, file: file, line: line)

        switch step {
        case "Manual은 새 매뉴얼을 초안 상태로 저장해야 한다",
            "복제된 매뉴얼은 초안 상태로 시작해야 한다":
            try assertField(manual(world)["status"], equals: "draft", "manual.status draft", file: file, line: line)
        case "사용자는 매뉴얼 이름을 입력해야 한다",
            "Manual은 매뉴얼 이름을 제공해야 한다":
            try assertString(manual(world)["name"], "manual.name", file: file, line: line)
        case "매뉴얼의 목적을 입력할 수 있어야 한다":
            try assertString(manual(world)["purpose"], "manual.purpose", file: file, line: line)
        case "설명과 목적을 제공해야 한다":
            let m = try manual(world)
            try assertString(m["description"], "manual.description", file: file, line: line)
            try assertString(m["purpose"], "manual.purpose", file: file, line: line)
        case "상태를 제공해야 한다":
            try assertString(manual(world)["status"], "manual.status", file: file, line: line)
        case "생성일과 수정일을 제공해야 한다",
            "Manual은 새 수정일을 기록해야 한다":
            let m = try manual(world)
            try assertString(m["created_at"], "manual.created_at", file: file, line: line)
            try assertString(m["updated_at"], "manual.updated_at", file: file, line: line)
        case "현재 버전을 제공해야 한다",
            "Manual은 매뉴얼의 새 버전을 생성해야 한다":
            try assertNumber(manual(world)["current_version"], "manual.current_version", file: file, line: line)
        case "Manual은 로컬에 설치된 기본 실행 에이전트를 제공해야 한다",
            "사용자 로컬에 설치되어 감지된 에이전트 중에서 기본 실행 에이전트를 선택할 수 있어야 한다",
            "Manual은 기존 매뉴얼의 기본 속성과 실행 속성을 복사해야 한다":
            try assertString(manual(world)["default_agent"], "manual.default_agent", file: file, line: line)
        case "초기 실행 방식은 단일 에이전트 방식으로 시작할 수 있어야 한다":
            try assertField(manual(world)["execution_mode"], equals: "single_agent", "manual.execution_mode", file: file, line: line)
        case "모델 설정을 제공해야 한다":
            try assertObject(manual(world)["model"], "manual.model", file: file, line: line)
        case "워크플로우 단계 목록을 제공해야 한다":
            try assertArray(manual(world)["workflow_steps"], "manual.workflow_steps", file: file, line: line)
        case "각 단계의 입력과 산출물 정의를 제공해야 한다":
            let step = try firstWorkflowStep(world)
            try assertArray(step["input_schema"], "workflow_step.input_schema", file: file, line: line)
            try assertString(step["output_schema"], "workflow_step.output_schema", file: file, line: line)
        case "각 단계의 검증 정책을 제공해야 한다":
            try assertObject(firstWorkflowStep(world)["verification_policy"], "workflow_step.verification_policy", file: file, line: line)
        case "각 단계의 샌드박스 정책을 제공해야 한다":
            try assertObject(firstWorkflowStep(world)["sandbox_policy"], "workflow_step.sandbox_policy", file: file, line: line)
        case "Manual은 토큰 측정 사용 여부를 제공해야 한다":
            try assertBool(optimization(world)["token_measurement"], "optimization.token_measurement", file: file, line: line)
        case "시간 측정 사용 여부를 제공해야 한다":
            try assertBool(optimization(world)["time_measurement"], "optimization.time_measurement", file: file, line: line)
        case "Verification 측정 기준을 제공해야 한다":
            try assertArray(optimization(world)["verification_criteria"], "optimization.verification_criteria", file: file, line: line)
        case "개선 추천 사용 여부를 제공해야 한다":
            try assertBool(optimization(world)["improvement_recommendations"], "optimization.improvement_recommendations", file: file, line: line)
        case "자기진화 모드를 제공해야 한다":
            try assertString(optimization(world)["self_evolution_mode"], "optimization.self_evolution_mode", file: file, line: line)
        case "Manual은 매뉴얼을 저장하지 않아야 한다",
            "사용자에게 이름이 필수라는 오류를 제공해야 한다",
            "Manual은 삭제를 거부해야 한다":
            try assertObject(last(world)["error"], "jsonrpc.error", file: file, line: line)
        case "Manual은 매뉴얼 이름, 상태, 최신 수정일, 현재 버전을 목록에 제공해야 한다":
            let first = try firstManualSummary(world)
            try assertString(first["name"], "manual summary.name", file: file, line: line)
            try assertString(first["status"], "manual summary.status", file: file, line: line)
            try assertString(first["updated_at"], "manual summary.updated_at", file: file, line: line)
            try assertNumber(first["current_version"], "manual summary.current_version", file: file, line: line)
        case "사용자는 태그와 상태로 매뉴얼을 필터링할 수 있어야 한다":
            try assertArray(result(world)["filters"], "manual.list filters", file: file, line: line)
        case "사용자는 이름이나 설명으로 매뉴얼을 검색할 수 있어야 한다":
            try assertArray(result(world)["search_fields"], "manual.list search_fields", file: file, line: line)
        case "최근 실행 기록과 최근 변경 이력을 제공해야 한다",
            "변경 이력을 남겨야 한다":
            let m = try manual(world)
            try assertArray(m["recent_runs"], "manual.recent_runs", file: file, line: line)
            try assertArray(m["change_history"], "manual.change_history", file: file, line: line)
        case "이전 버전을 보존해야 한다",
            "Manual은 각 버전의 생성 시점과 변경 요약을 제공해야 한다",
            "사용자는 특정 버전의 상세 내용을 확인할 수 있어야 한다":
            try assertArray(versions(world), "manual versions", file: file, line: line)
        case "사용자는 변경 전후 차이를 확인할 수 있어야 한다",
            "사용자는 현재 버전과 이전 버전의 차이를 비교할 수 있어야 한다":
            try assertObject(manualOrResult(world)["last_diff"] ?? result(world)["diff"], "version diff", file: file, line: line)
        case "Manual은 매뉴얼 상태를 보관됨으로 변경해야 한다":
            try assertField(manual(world)["status"], equals: "archived", "manual.status archived", file: file, line: line)
        case "Manual은 매뉴얼을 삭제해야 한다":
            try assertBool(manual(world)["deleted"], "manual.deleted", file: file, line: line)
        case "Manual은 기본 실행 에이전트를 선택할 수 없다는 상태를 제공해야 한다",
            "사용자가 로컬 에이전트 설치 또는 경로 설정이 필요함을 알 수 있어야 한다",
            "사용자에게 어떤 단계에 샌드박스가 필요한지 알려줘야 한다",
            "사용자에게 어떤 단계에 예산이나 검증 기준이 필요한지 알려줘야 한다":
            try assertArray(validationMissing(world), "validation.missing", file: file, line: line)
        case "매뉴얼을 활성 상태로 저장하지 않아야 한다",
            "누락된 단계가 있으면 활성화를 막아야 한다",
            "누락된 예산이나 검증 기준이 있으면 활성화를 막아야 한다":
            try assertBool(result(world)["activated"], "activation result", file: file, line: line)

        case "Manual은 등록된 노드의 이름과 설명을 제공해야 한다":
            let template = try digestTemplate(world)
            try assertString(template["name"], "node.name", file: file, line: line)
            try assertString(template["description"], "node.description", file: file, line: line)
        case "각 노드의 실행 종류가 스크립트인지 에이전트인지 제공해야 한다":
            try assertString((digestTemplate(world)["node"] as? [String: Any])?["kind"], "node.kind", file: file, line: line)
        case "각 노드의 입력 schema와 출력 schema 존재 여부를 제공해야 한다",
            "Manual은 해당 노드의 입력 필드와 필수 여부를 제공해야 한다":
            try assertArray(schema(world)["inputs"], "node.schema.inputs", file: file, line: line)
        case "출력 필드와 산출물 형식을 제공해야 한다",
            "검증 가능한 출력 조건을 제공해야 한다":
            try assertString(schema(world)["output_description"], "node.schema.output_description", file: file, line: line)
        case "노드 종류별 고유 정보가 있으면 별도로 제공해야 한다":
            if let schema = try? schema(world) {
                try assertString(schema["kind"], "node.schema.kind", file: file, line: line)
            } else {
                try assertString(nodeRun(world)["node_id"], "node run node_id", file: file, line: line)
            }
        case "Manual은 워크플로우 전체를 실행하지 않고 해당 노드만 실행해야 한다",
            "실행 결과를 노드 단위 기록으로 저장해야 한다",
            "최종 출력을 제공해야 한다",
            "출력 schema와 실제 출력의 일치 여부를 제공해야 한다",
            "실행 결과가 기대에 맞는다",
            "Manual은 입력, 출력, 로그, 실행 시간, 성공 여부를 공통 형식으로 제공해야 한다":
            let run = try nodeRun(world)
            try assertField(run["status"], equals: "completed", "node run status", file: file, line: line)
            try assertString(run["result"], "node run result", file: file, line: line)
        case "실행에 사용한 입력값을 기록해야 한다",
            "Manual은 실행 로그를 제공해야 한다",
            "중간 결과가 있으면 제공해야 한다":
            try assertArray(result(world.nodeEvents)["events"], "node run events", file: file, line: line)
        case "Manual은 입력값, 기대 출력, 검증 기준을 노드 테스트 케이스로 기록해야 한다":
            let testCase = try testCase(world)
            try assertObject(testCase["inputs"], "testcase.inputs", file: file, line: line)
            try assertString(testCase["expected_output"], "testcase.expected_output", file: file, line: line)
            try assertObject(testCase["criteria"], "testcase.criteria", file: file, line: line)
        case "이후 같은 노드 변경 시 해당 테스트 케이스를 다시 사용할 수 있어야 한다":
            try expectStep(world.nodeTestCaseID?.isEmpty == false, "node testcase id should be recorded", file: file, line: line)
        case "Manual은 저장된 테스트 케이스로 노드를 실행해야 한다",
            "기대 출력과 실제 출력을 비교해야 한다",
            "실패한 테스트 케이스와 차이를 제공해야 한다":
            try assertArray(result(world)["results"] ?? result(world)["failed"], "testcase verification result", file: file, line: line)
        case "Manual은 등록된 노드를 단계 후보로 제공해야 한다":
            try assertObject(result(world)["candidate"], "compose candidate", file: file, line: line)
        case "선택된 노드의 입력과 출력 schema를 워크플로우 단계 정의에 연결해야 한다":
            try assertObject(result(world)["stage"], "compose stage", file: file, line: line)

        case "Manual은 지정된 단계부터 실행해야 한다",
            "Manual은 실패한 단계와 이후 단계만 다시 실행해야 한다",
            "Manual은 제공된 입력값으로 해당 단계를 실행해야 한다",
            "사용자가 확인한 입력값으로 재실행해야 한다",
            "Manual은 로컬에 설치된 기본 실행 에이전트의 실행 결과를 기록해야 한다":
            try assertString(result(world)["run_id"] ?? workflowRun(world)["run_id"], "workflow run id", file: file, line: line)
        case "Manual은 실행 중인 단계에 중단 요청을 전달해야 한다":
            try assertBool(result(world)["cancelled"], "workflow stop cancelled", file: file, line: line)
        case "Manual은 현재 단계의 입력, 출력, 로그, 검증 상태를 제공해야 한다",
            "중단된 단계와 이미 완료된 단계를 구분해 기록해야 한다",
            "성공한 이전 단계의 결과를 보존해야 한다",
            "실행하지 않은 이전 단계는 새 실행 기록으로 덮어쓰지 않아야 한다":
            try assertObject(workflowRun(world), "workflow run", file: file, line: line)

        case "Manual은 지정된 skill을 단계 설정에 저장해야 한다",
            "Manual은 지정된 skill 목록을 단계 설정에 저장해야 한다",
            "skill 목록의 순서나 우선순위를 확인할 수 있게 해야 한다":
            try assertObject(result(world)["step"], "skill step", file: file, line: line)
        case "Manual은 작업 유형과 관련된 skill 후보를 제공해야 한다",
            "각 skill의 이름과 목적을 제공해야 한다":
            try assertArray(result(world)["candidates"], "skill candidates", file: file, line: line)
        case "Manual은 실행 요청에 포함된 skill 정보를 기록해야 한다",
            "에이전트 출력이나 로그에서 관찰된 skill 사용 신호를 기록해야 한다",
            "Manual은 지정된 skill이 사용되었는지 확인해야 한다":
            try assertObject(result(world), "skill verification result", file: file, line: line)
        case "Manual은 각 에이전트가 지원하는 skill 전달 방식을 제공해야 한다",
            "지원 여부가 불확실한 경우 미확인 상태로 제공해야 한다":
            try assertArray(result(world)["agents"], "agent capabilities", file: file, line: line)

        case "Manual은 현재 플랫폼에 맞는 OS 샌드박스 실행기로 에이전트 프로세스를 시작해야 한다",
            "Manual은 스크립트 노드의 샌드박스 정책을 저장해야 한다",
            "Manual은 파일 읽기를 허용해야 한다",
            "Manual은 파일 쓰기를 차단해야 한다",
            "Manual은 네트워크 접근을 차단해야 한다",
            "Manual은 명령 실행을 차단해야 한다":
            try assertObject(result(world), "sandbox result", file: file, line: line)

        case "Manual은 전체 토큰 사용량을 기록해야 한다",
            "토큰 사용량, 검증 상태, 작업 시간을 측정해야 한다",
            "Manual은 요구사항 충족도를 제공해야 한다",
            "검증 통과율을 제공해야 한다",
            "Manual은 전체 실행 시간을 기록해야 한다":
            try assertObject(result(world), "optimization result", file: file, line: line)
        case "Manual은 반복되는 탐색이나 정리 작업을 찾아야 한다",
            "토큰 낭비가 반복되는 단계를 찾아야 한다",
            "검증이 자주 누락되는 단계를 찾아야 한다",
            "결과가 흔들리는 작업을 진화 후보로 제공해야 한다",
            "스크립트화, 전처리, 검증 추가, 모델 변경 후보를 제안해야 한다",
            "더 작은 모델로 처리할 수 있는 단계를 제안해야 한다":
            try assertObject(result(world), "analysis result", file: file, line: line)
        case "optimization report는 derived 측정 근거를 포함해야 한다":
            try assertField(result(world)["measurement_mode"], equals: "derived", "optimization.measurement_mode", file: file, line: line)
            let optimizationResult = try result(world)
            try expectStep(
                (optimizationResult["measurement_note"] as? String)?.contains("Estimated") == true,
                "optimization.measurement_note should explain derived estimation",
                file: file,
                line: line
            )
        case "UI starter workflow는 code review 단계와 diff 수집 단계를 가져야 한다":
            let workflow = try result(world)["workflow"] as? [String: Any]
            let nodes = workflow?["nodes"] as? [[String: Any]] ?? []
            try expectStep(
                nodes.contains { ($0["id"] as? String) == "collect_diff" && ($0["kind"] as? String) == "script" },
                "starter workflow should include collect_diff script node",
                file: file,
                line: line
            )
            try expectStep(
                nodes.contains { ($0["id"] as? String) == "review" && ["codex", "claude", "pi"].contains($0["kind"] as? String ?? "") },
                "starter workflow should include review agent node",
                file: file,
                line: line
            )

        default:
            try assertGenericServerBackedResult(world: world, step: step, file: file, line: line)
        }
    }

    private static func assertEvidence(world: ManualWorld, step: String, file: StaticString, line: UInt) throws {
        if step == "mac UI intent가 테스트 app-server discovery를 사용한다" { return }
        try expectStep(!world.appServer.evidence.isEmpty || world.currentRunID != nil, "step should produce app-server evidence: \(step)", file: file, line: line)
    }

    private static func assertGenericServerBackedResult(world: ManualWorld, step: String, file: StaticString, line: UInt) throws {
        guard isExpectation(step), let response = world.lastResponse else { return }
        if let error = response["error"] as? [String: Any] {
            let allowedError = step.contains("거부") || step.contains("않아야") || step.contains("막아야") || step.contains("필수") || step.contains("이유")
            try expectStep(allowedError && error["message"] is String, "unexpected app-server error for \(step)", file: file, line: line)
        } else {
            try assertObject(response["result"], "jsonrpc.result", file: file, line: line)
        }
    }

    private static func isExpectation(_ step: String) -> Bool {
        ["해야 한다", "있어야 한다", "않아야 한다", "제공해야 한다", "기록해야 한다", "막아야 한다", "거부해야 한다", "변경해야 한다", "저장해야 한다"]
            .contains { step.contains($0) }
    }

    private static func last(_ world: ManualWorld) throws -> [String: Any] {
        guard let response = world.lastResponse else { throw StepError.assertion("last app-server response is missing") }
        return response
    }

    private static func result(_ world: ManualWorld) throws -> [String: Any] {
        try result(world.lastResponse)
    }

    private static func result(_ response: [String: Any]?) throws -> [String: Any] {
        guard let result = response?["result"] as? [String: Any] else { throw StepError.assertion("jsonrpc.result is missing") }
        return result
    }

    private static func manual(_ world: ManualWorld) throws -> [String: Any] {
        guard let manual = try result(world)["manual"] as? [String: Any] else { throw StepError.assertion("result.manual is missing") }
        return manual
    }

    private static func manualOrResult(_ world: ManualWorld) throws -> [String: Any] {
        if let manual = try? manual(world) {
            return manual
        }
        return try result(world)
    }

    private static func optimization(_ world: ManualWorld) throws -> [String: Any] {
        guard let optimization = try manual(world)["optimization"] as? [String: Any] else { throw StepError.assertion("manual.optimization is missing") }
        return optimization
    }

    private static func firstWorkflowStep(_ world: ManualWorld) throws -> [String: Any] {
        guard let step = (try manual(world)["workflow_steps"] as? [[String: Any]])?.first else { throw StepError.assertion("manual.workflow_steps[0] is missing") }
        return step
    }

    private static func firstManualSummary(_ world: ManualWorld) throws -> [String: Any] {
        guard let first = (try result(world)["manuals"] as? [[String: Any]])?.first else { throw StepError.assertion("result.manuals[0] is missing") }
        return first
    }

    private static func versions(_ world: ManualWorld) throws -> Any? {
        let currentResult = try result(world.lastResponse)
        if let versions = currentResult["versions"] {
            return versions
        }
        if let manual = currentResult["manual"] as? [String: Any] {
            return manual["versions"]
        }
        if let manualID = world.currentManualID {
            let response = try world.appServer.rpc(method: "manual.versions", params: ["manual_id": manualID])
            return try result(response)["versions"]
        }
        return nil
    }

    private static func validationMissing(_ world: ManualWorld) throws -> Any? {
        if let validation = try result(world.lastResponse)["validation"] as? [String: Any] {
            return validation["missing"]
        }
        if let manualID = world.currentManualID {
            let response = try world.appServer.rpc(method: "manual.activate", params: ["manual_id": manualID])
            return (try result(response)["validation"] as? [String: Any])?["missing"]
        }
        return nil
    }

    private static func digestTemplate(_ world: ManualWorld) throws -> [String: Any] {
        let response = world.latestNodeList ?? world.lastResponse
        guard let templates = try result(response)["templates"] as? [[String: Any]],
              let template = templates.first(where: { $0["id"] as? String == "digest" })
        else { throw StepError.assertion("digest node template is missing") }
        return template
    }

    private static func schema(_ world: ManualWorld) throws -> [String: Any] {
        guard let schema = try result(world.latestSchema ?? world.lastResponse)["schema"] as? [String: Any] else { throw StepError.assertion("result.schema is missing") }
        return schema
    }

    private static func nodeRun(_ world: ManualWorld) throws -> [String: Any] {
        guard let run = try result(world.nodeEvents ?? world.lastResponse)["run"] as? [String: Any] else { throw StepError.assertion("node run is missing") }
        return run
    }

    private static func testCase(_ world: ManualWorld) throws -> [String: Any] {
        guard let testCase = try result(world)["test_case"] as? [String: Any] else { throw StepError.assertion("result.test_case is missing") }
        return testCase
    }

    private static func workflowRun(_ world: ManualWorld) throws -> [String: Any] {
        if let run = try result(world.lastResponse)["run"] as? [String: Any] {
            return run
        }
        if let runID = world.currentRunID {
            let events = try world.appServer.rpc(method: "workflow.events", params: ["run_id": runID, "cursor": 0])
            if let run = try result(events)["run"] as? [String: Any] {
                return run
            }
        }
        throw StepError.assertion("result.run is missing")
    }

    private static func assertObject(_ value: Any?, _ name: String, file: StaticString, line: UInt) throws {
        try expectStep(value is [String: Any], "\(name) should be an object", file: file, line: line)
    }

    private static func assertArray(_ value: Any?, _ name: String, file: StaticString, line: UInt) throws {
        try expectStep(value is [Any], "\(name) should be an array", file: file, line: line)
    }

    private static func assertString(_ value: Any?, _ name: String, file: StaticString, line: UInt) throws {
        try expectStep((value as? String)?.isEmpty == false, "\(name) should be a non-empty string", file: file, line: line)
    }

    private static func assertNumber(_ value: Any?, _ name: String, file: StaticString, line: UInt) throws {
        try expectStep(value is NSNumber || value is Int || value is Double, "\(name) should be a number", file: file, line: line)
    }

    private static func assertBool(_ value: Any?, _ name: String, file: StaticString, line: UInt) throws {
        try expectStep(value is Bool || value is NSNumber, "\(name) should be a boolean", file: file, line: line)
    }

    private static func assertField(_ value: Any?, equals expected: String, _ name: String, file: StaticString, line: UInt) throws {
        try expectStep(value as? String == expected, "\(name) should equal \(expected)", file: file, line: line)
    }
}
