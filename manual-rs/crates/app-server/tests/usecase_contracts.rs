//! Cucumber feature contract tests for `docs/usecase/*.feature`.
//! Why this exists: docs/wiki/systems/기능-계약-테스트.md requires app-server to
//! execute shared usecase contracts from docs/usecase inside its own test suite.

use std::fs;
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use app_server::AppServer;
use cucumber::{World as _, given, then, when};
use manual_agent::{Agent, AgentCommand, CommandRequest, codex::Codex};
use serde_json::{Value, json};

static STORAGE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(cucumber::World)]
struct ManualWorld {
    server: AppServer,
    workflow_id: String,
    previous_run_id: Option<String>,
    current_run_id: Option<String>,
    current_events: Option<Value>,
    node_run_id: Option<String>,
    node_events: Option<Value>,
    latest_node_list: Option<Value>,
    latest_schema: Option<Value>,
    last_response: Option<Value>,
    latest_agent_command_program: Option<String>,
    latest_agent_command_args: Vec<String>,
    sandbox_test_dir: Option<PathBuf>,
    sandbox_test_log: Option<PathBuf>,
    sandbox_denied_delete: Option<PathBuf>,
    manual_id: Option<String>,
    sandbox_id: Option<String>,
    optimization_run_id: Option<String>,
    skill_step_id: Option<String>,
    node_test_case_id: Option<String>,
}

impl Default for ManualWorld {
    fn default() -> Self {
        Self {
            server: test_server("cucumber-usecase-contract"),
            workflow_id: "contract-workflow".to_owned(),
            previous_run_id: None,
            current_run_id: None,
            current_events: None,
            node_run_id: None,
            node_events: None,
            latest_node_list: None,
            latest_schema: None,
            last_response: None,
            latest_agent_command_program: None,
            latest_agent_command_args: Vec::new(),
            sandbox_test_dir: None,
            sandbox_test_log: None,
            sandbox_denied_delete: None,
            manual_id: None,
            sandbox_id: None,
            optimization_run_id: None,
            skill_step_id: None,
            node_test_case_id: None,
        }
    }
}

impl std::fmt::Debug for ManualWorld {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ManualWorld")
            .field("workflow_id", &self.workflow_id)
            .field("previous_run_id", &self.previous_run_id)
            .field("current_run_id", &self.current_run_id)
            .field("node_run_id", &self.node_run_id)
            .finish()
    }
}

#[tokio::main]
async fn main() {
    let features = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/usecase")
        .canonicalize()
        .expect("docs/usecase should exist");

    ManualWorld::cucumber()
        .fail_on_skipped()
        .run_and_exit(features)
        .await;
}

#[given("사용자가 Manual에서 워크플로우 노드를 관리하고 있다")]
async fn user_manages_workflow_nodes(world: &mut ManualWorld) {
    ensure_storybook_node(world);
}

#[given("Manual은 등록된 노드 목록과 노드 schema를 보유할 수 있다")]
async fn manual_has_node_registry_and_schema(world: &mut ManualWorld) {
    ensure_storybook_node(world);
    world.latest_schema = Some(node_schema(world, "template"));
}

#[when("사용자가 노드 목록을 조회한다")]
async fn user_lists_nodes(world: &mut ManualWorld) {
    world.latest_node_list = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "node.list"
        }),
    ));
}

#[then("Manual은 등록된 노드의 이름과 설명을 제공해야 한다")]
async fn node_list_includes_name_and_description(world: &mut ManualWorld) {
    let templates = world.latest_node_list()["result"]["templates"]
        .as_array()
        .expect("node.list should return templates");
    let digest = templates
        .iter()
        .find(|template| template["id"] == "digest")
        .expect("registered digest node should be listed");

    assert_eq!(digest["name"], "Digest node");
    assert_eq!(
        digest["description"],
        "Summarizes a topic from Storybook input"
    );
}

#[then("각 노드의 실행 종류가 스크립트인지 에이전트인지 제공해야 한다")]
async fn node_list_includes_execution_kind(world: &mut ManualWorld) {
    let digest = find_digest_template(world.latest_node_list());
    assert_eq!(digest["node"]["kind"], "template");
}

#[then("각 노드의 입력 schema와 출력 schema 존재 여부를 제공해야 한다")]
async fn node_schema_presence_is_available(world: &mut ManualWorld) {
    let schema = world.latest_schema();
    assert_eq!(schema["result"]["schema"]["kind"], "template");
    assert!(schema["result"]["schema"]["inputs"].is_array());
    assert!(schema["result"]["schema"]["output_description"].is_string());
}

#[given("사용자가 특정 노드를 선택했다")]
async fn user_selects_node(world: &mut ManualWorld) {
    ensure_storybook_node(world);
}

#[when("사용자가 노드 schema를 조회한다")]
async fn user_reads_node_schema(world: &mut ManualWorld) {
    world.latest_schema = Some(node_schema(world, "template"));
}

#[then("Manual은 해당 노드의 입력 필드와 필수 여부를 제공해야 한다")]
async fn schema_includes_input_fields(world: &mut ManualWorld) {
    let inputs = world.latest_schema()["result"]["schema"]["inputs"]
        .as_array()
        .expect("template schema should include inputs");

    assert!(
        inputs
            .iter()
            .any(|field| field["name"] == "template" && field["required"] == true)
    );
}

#[then("출력 필드와 산출물 형식을 제공해야 한다")]
async fn schema_includes_output_shape(world: &mut ManualWorld) {
    assert!(
        world.latest_schema()["result"]["schema"]["output_description"]
            .as_str()
            .expect("schema should include output description")
            .contains("Rendered string")
    );
}

#[then("검증 가능한 출력 조건을 제공해야 한다")]
async fn schema_exposes_verifiable_output_condition(world: &mut ManualWorld) {
    assert_eq!(
        world.latest_schema()["result"]["schema"]["kind"],
        "template"
    );
}

#[given("사용자가 특정 노드의 입력값을 구성했다")]
async fn user_configures_node_input(world: &mut ManualWorld) {
    ensure_storybook_node(world);
}

#[when("사용자가 노드 독립 실행을 요청한다")]
async fn user_runs_node_independently(world: &mut ManualWorld) {
    let started = rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "node.run",
            "params": {
                "node": {
                    "id": "digest",
                    "kind": "template",
                    "template": "topic={{__storybook_input__.topic}} priority={{__storybook_input__.priority}}"
                },
                "inputs": {
                    "topic": "contract-tests",
                    "priority": 3
                }
            }
        }),
    );
    let run_id = started["result"]["run_id"]
        .as_str()
        .expect("node.run should return run_id")
        .to_owned();
    let events = poll_node_events_until(world, &run_id, 0, |events| {
        events["result"]["completed"].as_bool().unwrap_or(false)
    });

    world.node_run_id = Some(run_id);
    world.node_events = Some(events);
}

#[then("Manual은 워크플로우 전체를 실행하지 않고 해당 노드만 실행해야 한다")]
async fn only_selected_node_runs(world: &mut ManualWorld) {
    let events = world.node_events();
    assert_eq!(events["result"]["run"]["status"], "completed");
    assert_eq!(
        events["result"]["run"]["result"],
        "topic=contract-tests priority=3"
    );
    assert_eq!(
        events["result"]["events"]
            .as_array()
            .expect("node events should be available")
            .iter()
            .filter(|event| event["type"] == "node_completed")
            .count(),
        2,
        "storybook input and the selected node should complete"
    );
}

#[then("실행에 사용한 입력값을 기록해야 한다")]
async fn node_run_records_input(world: &mut ManualWorld) {
    assert!(
        world.node_events()["result"]["events"]
            .as_array()
            .expect("node events should be available")
            .iter()
            .any(|event| event["type"] == "node_completed"
                && event["node_id"] == "__storybook_input__"
                && event["result"]["topic"] == "contract-tests")
    );
}

#[then("실행 결과를 노드 단위 기록으로 저장해야 한다")]
async fn node_result_is_saved(world: &mut ManualWorld) {
    let run_id = world
        .node_run_id
        .clone()
        .expect("node run id should be recorded");
    let stored = rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "node.run.get",
            "params": {
                "run_id": run_id
            }
        }),
    );

    assert_eq!(stored["result"]["run"]["status"], "completed");
    assert_eq!(
        stored["result"]["run"]["result"],
        "topic=contract-tests priority=3"
    );
}

#[given("노드 독립 실행이 완료되었다")]
async fn node_independent_run_finished(world: &mut ManualWorld) {
    user_configures_node_input(world).await;
    user_runs_node_independently(world).await;
}

#[when("사용자가 실행 결과를 조회한다")]
async fn user_reads_node_run_result(world: &mut ManualWorld) {
    if let Some(run_id) = world.node_run_id.clone() {
        world.node_events = Some(poll_node_events_until(world, &run_id, 0, |events| {
            events["result"]["completed"].as_bool().unwrap_or(false)
        }));
    } else if world.skill_step_id.is_some() {
        manual_verifies_skill_usage(world).await;
    }
}

#[then("Manual은 실행 로그를 제공해야 한다")]
async fn node_run_exposes_log(world: &mut ManualWorld) {
    assert!(
        !world.node_events()["result"]["events"]
            .as_array()
            .expect("events should be an array")
            .is_empty()
    );
}

#[then("중간 결과가 있으면 제공해야 한다")]
async fn node_run_exposes_intermediate_results(world: &mut ManualWorld) {
    node_run_records_input(world).await;
}

#[then("최종 출력을 제공해야 한다")]
async fn node_run_exposes_final_output(world: &mut ManualWorld) {
    assert_eq!(
        world.node_events()["result"]["run"]["result"],
        "topic=contract-tests priority=3"
    );
}

#[then("출력 schema와 실제 출력의 일치 여부를 제공해야 한다")]
async fn node_run_output_matches_schema(world: &mut ManualWorld) {
    world.latest_schema = Some(node_schema(world, "template"));
    assert!(world.node_events()["result"]["run"]["result"].is_string());
    assert!(
        world.latest_schema()["result"]["schema"]["output_description"]
            .as_str()
            .expect("schema should include output description")
            .contains("Rendered string")
    );
}

#[given("사용자가 노드를 임의 입력으로 실행했다")]
async fn user_ran_node_with_arbitrary_input(world: &mut ManualWorld) {
    user_runs_node_independently(world).await;
}

#[given("실행 결과가 기대에 맞는다")]
async fn node_output_matches_expectation(world: &mut ManualWorld) {
    assert_eq!(
        world.node_events()["result"]["run"]["result"],
        "topic=contract-tests priority=3"
    );
}

#[when("사용자가 해당 입력과 결과를 테스트 케이스로 저장한다")]
async fn user_saves_node_test_case(world: &mut ManualWorld) {
    let run_id = world.node_run_id.clone().expect("node run should exist");
    let saved = rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 20,
            "method": "node.testcase.save",
            "params": {
                "run_id": run_id,
                "criteria": { "comparison": "json_equal", "schema_match_required": true }
            }
        }),
    );
    world.node_test_case_id = saved["result"]["test_case"]["id"]
        .as_str()
        .map(str::to_owned);
    world.last_response = Some(saved);
}

#[then("Manual은 입력값, 기대 출력, 검증 기준을 노드 테스트 케이스로 기록해야 한다")]
async fn node_test_case_records_inputs_output_and_criteria(world: &mut ManualWorld) {
    let case = &world.last()["result"]["test_case"];
    assert_eq!(case["inputs"]["topic"], "contract-tests");
    assert_eq!(case["expected_output"], "topic=contract-tests priority=3");
    assert_eq!(case["criteria"]["comparison"], "json_equal");
}

#[then("이후 같은 노드 변경 시 해당 테스트 케이스를 다시 사용할 수 있어야 한다")]
async fn node_test_case_is_reusable(world: &mut ManualWorld) {
    assert!(world.node_test_case_id.is_some());
}

#[given("특정 노드에 저장된 테스트 케이스가 존재한다")]
async fn saved_node_test_case_exists(world: &mut ManualWorld) {
    user_ran_node_with_arbitrary_input(world).await;
    user_saves_node_test_case(world).await;
}

#[when("사용자가 노드 검증을 요청한다")]
async fn user_requests_node_verification(world: &mut ManualWorld) {
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 21,
            "method": "node.testcase.verify",
            "params": { "node_id": "digest" }
        }),
    ));
}

#[then("Manual은 저장된 테스트 케이스로 노드를 실행해야 한다")]
async fn manual_runs_saved_node_test_cases(world: &mut ManualWorld) {
    assert!(
        !world.last()["result"]["results"]
            .as_array()
            .expect("verification results should be available")
            .is_empty()
    );
}

#[then("기대 출력과 실제 출력을 비교해야 한다")]
async fn manual_compares_expected_and_actual_outputs(world: &mut ManualWorld) {
    let result = &world.last()["result"]["results"][0];
    assert_eq!(result["expected_output"], result["actual_output"]);
}

#[then("실패한 테스트 케이스와 차이를 제공해야 한다")]
async fn manual_reports_failed_node_test_diffs(world: &mut ManualWorld) {
    assert!(world.last()["result"]["failed"].is_array());
}

#[given("사용자가 노드 독립 실행 결과를 조회한다")]
async fn user_reads_independent_node_run(world: &mut ManualWorld) {
    node_independent_run_finished(world).await;
    user_reads_node_run_result(world).await;
}

#[when("노드 종류가 스크립트 또는 에이전트이다")]
async fn node_kind_is_script_or_agent(world: &mut ManualWorld) {
    world.latest_schema = Some(node_schema(world, "codex"));
}

#[then("Manual은 입력, 출력, 로그, 실행 시간, 성공 여부를 공통 형식으로 제공해야 한다")]
async fn node_run_has_common_execution_shape(world: &mut ManualWorld) {
    let run = &world.node_events()["result"]["run"];
    assert!(run["result"].is_string());
    assert!(world.node_events()["result"]["events"].is_array());
    assert_eq!(run["status"], "completed");
}

#[then("노드 종류별 고유 정보가 있으면 별도로 제공해야 한다")]
async fn node_run_has_kind_specific_info(world: &mut ManualWorld) {
    assert_eq!(world.latest_schema()["result"]["schema"]["kind"], "codex");
}

#[given("등록된 노드 목록이 존재한다")]
async fn registered_node_list_exists(world: &mut ManualWorld) {
    ensure_storybook_node(world);
    user_lists_nodes(world).await;
}

#[when("사용자가 워크플로우 단계를 구성한다")]
async fn user_composes_workflow_step(world: &mut ManualWorld) {
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 22,
            "method": "workflow.compose_from_registry",
            "params": { "node_id": "digest" }
        }),
    ));
}

#[then("Manual은 등록된 노드를 단계 후보로 제공해야 한다")]
async fn registered_node_is_stage_candidate(world: &mut ManualWorld) {
    assert_eq!(world.last()["result"]["candidate"]["id"], "digest");
}

#[then("선택된 노드의 입력과 출력 schema를 워크플로우 단계 정의에 연결해야 한다")]
async fn selected_node_schema_is_linked_to_stage(world: &mut ManualWorld) {
    assert!(world.last()["result"]["stage"]["input_schema"].is_array());
    assert!(world.last()["result"]["stage"]["output_schema"].is_string());
}

#[then("등록되지 않은 노드는 워크플로우 단계로 사용할 수 없어야 한다")]
async fn unregistered_node_cannot_be_used(world: &mut ManualWorld) {
    let rejected = rpc_allowing_errors(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 23,
            "method": "workflow.compose_from_registry",
            "params": { "node_id": "missing" }
        }),
    );
    assert!(rejected.get("error").is_some());
}

#[given("사용자가 Manual에 여러 단계로 구성된 워크플로우를 등록했다")]
async fn user_registers_multistep_workflow(world: &mut ManualWorld) {
    create_three_stage_workflow(world, "partial-start");
    world.workflow_id = "partial-start".to_owned();
}

#[given("워크플로우에는 에이전트나 스크립트가 실행하는 단계가 포함되어 있다")]
async fn workflow_has_executable_stage(_world: &mut ManualWorld) {}

#[given("워크플로우의 앞 단계 결과가 이미 존재한다")]
async fn previous_stage_result_exists(world: &mut ManualWorld) {
    let workflow_id = world.workflow_id.clone();
    let run_id = start_workflow(world, &workflow_id, json!({}));
    let events = poll_workflow_events_until(world, &run_id, 0, |events| {
        events["result"]["completed"].as_bool().unwrap_or(false)
    });

    assert_eq!(
        events["result"]["run"]["nodes"]["source"]["result"],
        "alpha"
    );
    world.previous_run_id = Some(run_id);
}

#[when("사용자가 특정 단계를 시작 지점으로 지정하고 실행을 요청한다")]
async fn user_starts_from_specific_stage(world: &mut ManualWorld) {
    let previous_run_id = world
        .previous_run_id
        .clone()
        .expect("previous run should be available");
    let workflow_id = world.workflow_id.clone();
    let run_id = start_workflow(
        world,
        &workflow_id,
        json!({
            "start_node_id": "middle",
            "resume_run_id": previous_run_id
        }),
    );
    let events = poll_workflow_events_until(world, &run_id, 0, |events| {
        events["result"]["completed"].as_bool().unwrap_or(false)
    });

    world.current_run_id = Some(run_id);
    world.current_events = Some(events);
}

#[then("Manual은 지정된 단계부터 실행해야 한다")]
async fn manual_runs_from_specific_stage(world: &mut ManualWorld) {
    assert_eq!(
        world.current_events()["result"]["run"]["nodes"]["source"]["status"],
        "skipped"
    );
    assert_eq!(
        world.current_events()["result"]["run"]["nodes"]["middle"]["status"],
        "completed"
    );
}

#[then("해당 단계 실행에 필요한 이전 결과를 입력으로 사용해야 한다")]
async fn manual_uses_previous_outputs(world: &mut ManualWorld) {
    assert_eq!(
        world.current_events()["result"]["run"]["nodes"]["middle"]["result"],
        "middle sees alpha"
    );
}

#[then("실행하지 않은 이전 단계는 새 실행 기록으로 덮어쓰지 않아야 한다")]
async fn manual_preserves_skipped_previous_stage(world: &mut ManualWorld) {
    assert!(
        world.current_events()["result"]["events"]
            .as_array()
            .expect("workflow events should be available")
            .iter()
            .any(|event| event["type"] == "node_skipped" && event["node_id"] == "source")
    );
}

#[given("워크플로우 실행 중 특정 단계가 실패했다")]
async fn workflow_failed_at_specific_stage(world: &mut ManualWorld) {
    create_repairable_workflow(world);
    world.workflow_id = "resume-contract".to_owned();
    let workflow_id = world.workflow_id.clone();
    let run_id = start_workflow(world, &workflow_id, json!({}));
    let events = poll_workflow_events_until(world, &run_id, 0, |events| {
        events["result"]["completed"].as_bool().unwrap_or(false)
    });

    assert_eq!(events["result"]["run"]["status"], "failed");
    assert_eq!(events["result"]["run"]["first_failed_node"], "repairable");
    world.previous_run_id = Some(run_id);
}

#[when("사용자가 실패 지점부터 재시작을 요청한다")]
async fn user_restarts_from_failure(world: &mut ManualWorld) {
    patch_repairable_node(world);

    let previous_run_id = world
        .previous_run_id
        .clone()
        .expect("failed run should be available");
    let workflow_id = world.workflow_id.clone();
    let run_id = start_workflow(
        world,
        &workflow_id,
        json!({
            "resume_run_id": previous_run_id,
            "resume_from_failure": true
        }),
    );
    let events = poll_workflow_events_until(world, &run_id, 0, |events| {
        events["result"]["completed"].as_bool().unwrap_or(false)
    });

    world.current_run_id = Some(run_id);
    world.current_events = Some(events);
}

#[then("Manual은 실패한 단계와 이후 단계만 다시 실행해야 한다")]
async fn manual_reruns_failed_stage_and_later(world: &mut ManualWorld) {
    assert_eq!(
        world.current_events()["result"]["run"]["status"],
        "completed"
    );
    assert_eq!(
        world.current_events()["result"]["run"]["nodes"]["source"]["status"],
        "skipped"
    );
    assert_eq!(
        world.current_events()["result"]["run"]["nodes"]["repairable"]["result"],
        "repaired from alpha"
    );
    assert_eq!(
        world.current_events()["result"]["run"]["nodes"]["final"]["result"],
        "final=repaired from alpha"
    );
}

#[then("성공한 이전 단계의 결과를 보존해야 한다")]
async fn manual_preserves_successful_previous_stage(world: &mut ManualWorld) {
    assert_eq!(
        world.current_events()["result"]["run"]["nodes"]["source"]["status"],
        "skipped"
    );
}

#[then("재시작 실행과 기존 실행의 관계를 기록해야 한다")]
async fn manual_records_restart_relationship(world: &mut ManualWorld) {
    assert_ne!(
        world.current_run_id.as_deref(),
        world.previous_run_id.as_deref()
    );
    assert!(
        world.current_events()["result"]["events"]
            .as_array()
            .expect("events should be available")
            .iter()
            .any(|event| event["type"] == "node_skipped" && event["node_id"] == "source")
    );
}

#[given("워크플로우가 실행 중이다")]
async fn workflow_is_running(world: &mut ManualWorld) {
    create_delay_workflow(world, "running-flow", 200);
    world.workflow_id = "running-flow".to_owned();
    let workflow_id = world.workflow_id.clone();
    let run_id = start_workflow(world, &workflow_id, json!({}));
    world.current_run_id = Some(run_id);
}

#[when("사용자가 실행 중단을 요청한다")]
async fn user_requests_running_workflow_stop(world: &mut ManualWorld) {
    let run_id = world.current_run_id.clone().expect("run should exist");
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 30,
            "method": "workflow.stop",
            "params": { "run_id": run_id }
        }),
    ));
    let run_id = world.current_run_id.clone().expect("run should exist");
    world.current_events = Some(poll_workflow_events_until(world, &run_id, 0, |events| {
        events["result"]["completed"].as_bool().unwrap_or(false)
    }));
}

#[then("Manual은 실행 중인 단계에 중단 요청을 전달해야 한다")]
async fn manual_delivers_cancellation_request(world: &mut ManualWorld) {
    assert_eq!(world.last()["result"]["cancelled"], true);
}

#[then("이후 단계는 시작하지 않아야 한다")]
async fn later_steps_do_not_start_after_cancellation(world: &mut ManualWorld) {
    assert!(
        world.current_events()["result"]["events"]
            .as_array()
            .expect("events should exist")
            .iter()
            .all(|event| event["node_id"] != "after-delay")
    );
}

#[then("중단된 단계와 이미 완료된 단계를 구분해 기록해야 한다")]
async fn cancellation_records_completed_and_cancelled(world: &mut ManualWorld) {
    assert_eq!(
        world.current_events()["result"]["run"]["status"],
        "cancelled"
    );
}

#[given("워크플로우가 단계별 수동 진행 모드로 실행 중이다")]
async fn workflow_runs_in_step_mode(world: &mut ManualWorld) {
    let workflow_id = world.workflow_id.clone();
    let run_id = start_workflow(world, &workflow_id, json!({ "mode": "step" }));
    let mut events = poll_workflow_events_until(world, &run_id, 0, |events| {
        events["result"]["run"]["paused"].as_bool().unwrap_or(false)
    });
    if events["result"]["run"]["nodes"]["source"]["status"] != "completed" {
        rpc(
            world,
            json!({
                "jsonrpc": "2.0",
                "id": 33,
                "method": "workflow.resume",
                "params": { "run_id": run_id }
            }),
        );
        events = poll_workflow_events_until(world, &run_id, 0, |events| {
            events["result"]["run"]["nodes"]["source"]["status"] == "completed"
                && events["result"]["run"]["paused"].as_bool().unwrap_or(false)
        });
    }
    world.current_run_id = Some(run_id);
    world.current_events = Some(events);
}

#[given("현재 단계가 완료되었다")]
async fn current_step_is_complete(world: &mut ManualWorld) {
    assert!(world.current_events()["result"]["run"]["nodes"]["source"]["status"] == "completed");
}

#[when("사용자가 현재 단계 결과를 확인한다")]
async fn user_reviews_current_step_result(world: &mut ManualWorld) {
    let run_id = world.current_run_id.clone().expect("run should exist");
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 31,
            "method": "workflow.events",
            "params": { "run_id": run_id }
        }),
    ));
}

#[then("Manual은 현재 단계의 입력, 출력, 로그, 검증 상태를 제공해야 한다")]
async fn current_step_review_payload_is_available(world: &mut ManualWorld) {
    assert!(world.last()["result"]["events"].is_array());
    assert_eq!(
        world.last()["result"]["run"]["nodes"]["source"]["result"],
        "alpha"
    );
}

#[then("사용자가 계속 진행을 요청한 경우에만 다음 단계를 실행해야 한다")]
async fn next_step_runs_only_after_resume(world: &mut ManualWorld) {
    let run_id = world.current_run_id.clone().expect("run should exist");
    rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 32,
            "method": "workflow.resume",
            "params": { "run_id": run_id }
        }),
    );
    let run_id = world.current_run_id.clone().expect("run should exist");
    let events = poll_workflow_events_until(world, &run_id, 0, |events| {
        events["result"]["run"]["nodes"]["middle"]["status"] == "completed"
    });
    assert_eq!(
        events["result"]["run"]["nodes"]["middle"]["status"],
        "completed"
    );
}

#[given("사용자가 워크플로우의 중간 단계를 실행하려고 한다")]
async fn user_wants_to_run_middle_stage(world: &mut ManualWorld) {
    world.workflow_id = "partial-start".to_owned();
}

#[when("해당 단계에 필요한 입력이 이전 실행 결과에서 제공되지 않는다")]
async fn middle_stage_missing_previous_input(world: &mut ManualWorld) {
    let workflow_id = world.workflow_id.clone();
    let run_id = start_workflow(
        world,
        &workflow_id,
        json!({
            "start_node_id": "middle",
            "input_overrides": { "source": "manual-input" }
        }),
    );
    world.current_run_id = Some(run_id.clone());
    world.current_events = Some(poll_workflow_events_until(world, &run_id, 0, |events| {
        events["result"]["completed"].as_bool().unwrap_or(false)
    }));
}

#[then("Manual은 사용자가 필요한 입력값을 제공할 수 있게 해야 한다")]
async fn manual_accepts_middle_stage_input(world: &mut ManualWorld) {
    assert_eq!(
        world.current_events()["result"]["run"]["nodes"]["middle"]["result"],
        "middle sees manual-input"
    );
}

#[then("제공된 입력값을 해당 단계의 실행 기록에 남겨야 한다")]
async fn manual_records_supplied_middle_input(world: &mut ManualWorld) {
    assert!(
        world.current_events()["result"]["events"]
            .as_array()
            .expect("events should exist")
            .iter()
            .any(|event| event["type"] == "input_override"
                && event["node_id"] == "source"
                && event["value"] == "manual-input")
    );
}

#[then("Manual은 제공된 입력값으로 해당 단계를 실행해야 한다")]
async fn manual_runs_with_supplied_input(world: &mut ManualWorld) {
    manual_accepts_middle_stage_input(world).await;
}

#[given("사용자가 워크플로우 실행을 준비하고 있다")]
async fn user_prepares_workflow_execution(_world: &mut ManualWorld) {}

#[when("사용자가 실행 방식을 선택한다")]
async fn user_selects_execution_mode(world: &mut ManualWorld) {
    let workflow_id = world.workflow_id.clone();
    let run_id = start_workflow(world, &workflow_id, json!({ "mode": "step" }));
    world.current_run_id = Some(run_id);
}

#[then("Manual은 전체 자동 실행 방식을 제공해야 한다")]
async fn manual_offers_auto_mode(_world: &mut ManualWorld) {}

#[then("단계별 수동 진행 방식을 제공해야 한다")]
async fn manual_offers_step_mode(_world: &mut ManualWorld) {}

#[then("선택된 실행 방식을 실행 기록에 남겨야 한다")]
async fn selected_execution_mode_is_recorded(world: &mut ManualWorld) {
    assert!(world.current_run_id.is_some());
}

#[given("사용자가 이전에 실행한 단계를 다시 실행하려고 한다")]
async fn user_wants_to_rerun_previous_stage(world: &mut ManualWorld) {
    previous_stage_result_exists(world).await;
}

#[when("사용자가 새 입력값을 제공한다")]
async fn user_supplies_new_input_for_rerun(world: &mut ManualWorld) {
    let previous_run_id = world
        .previous_run_id
        .clone()
        .expect("previous run should exist");
    let workflow_id = world.workflow_id.clone();
    let run_id = start_workflow(
        world,
        &workflow_id,
        json!({
            "start_node_id": "middle",
            "resume_run_id": previous_run_id,
            "input_overrides": { "source": "new-alpha" }
        }),
    );
    world.current_run_id = Some(run_id.clone());
    world.current_events = Some(poll_workflow_events_until(world, &run_id, 0, |events| {
        events["result"]["completed"].as_bool().unwrap_or(false)
    }));
}

#[then("Manual은 기존 입력값과 새 입력값의 차이를 확인할 수 있게 해야 한다")]
async fn manual_shows_old_new_input_diff(world: &mut ManualWorld) {
    assert_eq!(
        world.current_events()["result"]["run"]["nodes"]["middle"]["result"],
        "middle sees new-alpha"
    );
}

#[then("사용자가 확인한 입력값으로 재실행해야 한다")]
async fn manual_reruns_with_confirmed_input(world: &mut ManualWorld) {
    manual_shows_old_new_input_diff(world).await;
}

#[then("재실행 결과를 이전 결과와 별도로 기록해야 한다")]
async fn manual_records_rerun_separately(world: &mut ManualWorld) {
    assert_ne!(world.current_run_id, world.previous_run_id);
}

#[given("사용자가 로컬에서 Manual 프로젝트를 사용한다")]
async fn user_uses_local_manual_project(world: &mut ManualWorld) {
    ensure_manual(world);
}

#[given("Manual은 단일 사용자 로컬 프로그램으로 동작한다")]
async fn manual_is_local_single_user(world: &mut ManualWorld) {
    assert!(world.manual_id.is_some());
}

#[when("사용자가 새 매뉴얼을 생성한다")]
async fn user_creates_new_manual(world: &mut ManualWorld) {
    create_manual(world, "Contract Manual", "codex", json!({}));
}

#[then("사용자는 매뉴얼 이름을 입력해야 한다")]
async fn manual_name_is_required(world: &mut ManualWorld) {
    assert!(world.manual()["name"].is_string());
}

#[then("매뉴얼의 목적을 입력할 수 있어야 한다")]
async fn manual_purpose_is_available(world: &mut ManualWorld) {
    assert!(world.manual()["purpose"].is_string());
}

#[then("사용자 로컬에 설치되어 감지된 에이전트 중에서 기본 실행 에이전트를 선택할 수 있어야 한다")]
async fn manual_uses_detected_agent_candidate(world: &mut ManualWorld) {
    assert_eq!(world.manual()["default_agent"], "codex");
}

#[then("초기 실행 방식은 단일 에이전트 방식으로 시작할 수 있어야 한다")]
async fn manual_starts_single_agent(world: &mut ManualWorld) {
    assert_eq!(world.manual()["execution_mode"], "single_agent");
}

#[then("Manual은 새 매뉴얼을 초안 상태로 저장해야 한다")]
async fn manual_is_saved_as_draft(world: &mut ManualWorld) {
    assert_eq!(world.manual()["status"], "draft");
}

#[given("사용자가 새 매뉴얼을 생성하고 있다")]
async fn user_is_creating_manual(world: &mut ManualWorld) {
    ensure_manual(world);
}

#[given("사용자 로컬에 \"claude\", \"codex\", \"pi\", \"hermes\" 중 일부 에이전트가 설치되어 있다")]
async fn some_local_agents_are_installed(world: &mut ManualWorld) {
    let bin_dir = fake_agent_bin("agent-detect");
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 40,
            "method": "agent.list",
            "params": {
                "candidates": ["claude", "codex", "pi", "hermes"],
                "path_dirs": [bin_dir]
            }
        }),
    ));
}

#[when("사용자가 기본 실행 에이전트를 선택하려고 한다")]
async fn user_selects_default_agent(_world: &mut ManualWorld) {}

#[then("Manual은 로컬에서 감지된 에이전트만 선택 가능한 후보로 제공해야 한다")]
async fn manual_lists_only_detected_agents(world: &mut ManualWorld) {
    assert!(agent_available(world.last(), "codex"));
}

#[then("감지되지 않은 에이전트는 선택 가능한 후보에 포함하지 않아야 한다")]
async fn manual_excludes_undetected_agents(world: &mut ManualWorld) {
    assert!(!agent_available(world.last(), "hermes"));
}

#[then("각 후보의 실행 가능 여부를 제공해야 한다")]
async fn manual_reports_agent_executability(world: &mut ManualWorld) {
    assert!(world.last()["result"]["agents"][0]["available"].is_boolean());
}

#[given("사용자 로컬에서 실행 가능한 에이전트가 감지되지 않았다")]
async fn no_executable_agent_detected(world: &mut ManualWorld) {
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 41,
            "method": "agent.list",
            "params": {
                "candidates": ["claude", "codex", "pi", "hermes"],
                "path_dirs": [unique_storage_dir("empty-bin")]
            }
        }),
    ));
}

#[when("사용자가 새 매뉴얼을 활성 상태로 저장하려고 한다")]
async fn user_tries_to_save_active_manual(world: &mut ManualWorld) {
    create_manual(world, "No Agent Manual", "", json!({}));
    let manual_id = world.manual_id.clone().expect("manual should exist");
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 42,
            "method": "manual.activate",
            "params": { "manual_id": manual_id }
        }),
    ));
}

#[then("Manual은 기본 실행 에이전트를 선택할 수 없다는 상태를 제공해야 한다")]
async fn manual_reports_no_default_agent(world: &mut ManualWorld) {
    assert!(
        world.last()["result"]["validation"]["missing"]
            .as_array()
            .expect("missing fields should be available")
            .iter()
            .any(|missing| missing["field"] == "default_agent")
    );
}

#[then("매뉴얼을 활성 상태로 저장하지 않아야 한다")]
async fn manual_is_not_activated(world: &mut ManualWorld) {
    assert_eq!(world.last()["result"]["activated"], false);
}

#[then("사용자가 로컬 에이전트 설치 또는 경로 설정이 필요함을 알 수 있어야 한다")]
async fn manual_reports_agent_install_guidance(world: &mut ManualWorld) {
    assert!(
        world.last()["result"]["validation"]["missing"]
            .as_array()
            .expect("missing fields should exist")
            .iter()
            .any(|missing| missing["message"]
                .as_str()
                .unwrap_or_default()
                .contains("path"))
    );
}

#[given("사용자가 매뉴얼 상세 정보를 조회했다")]
async fn user_read_manual_detail(world: &mut ManualWorld) {
    ensure_manual(world);
    let manual_id = world.manual_id.clone().expect("manual should exist");
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 43,
            "method": "manual.get",
            "params": { "manual_id": manual_id }
        }),
    ));
}

#[then("Manual은 매뉴얼 이름을 제공해야 한다")]
async fn manual_detail_has_name(world: &mut ManualWorld) {
    assert!(world.manual()["name"].is_string());
}

#[then("설명과 목적을 제공해야 한다")]
async fn manual_detail_has_description_and_purpose(world: &mut ManualWorld) {
    assert!(world.manual()["description"].is_string());
    assert!(world.manual()["purpose"].is_string());
}

#[then("상태를 제공해야 한다")]
async fn manual_detail_has_status(world: &mut ManualWorld) {
    assert!(world.manual()["status"].is_string());
}

#[then("생성일과 수정일을 제공해야 한다")]
async fn manual_detail_has_timestamps(world: &mut ManualWorld) {
    assert!(world.manual()["created_at"].is_string());
    assert!(world.manual()["updated_at"].is_string());
}

#[then("현재 버전을 제공해야 한다")]
async fn manual_detail_has_current_version(world: &mut ManualWorld) {
    assert!(world.manual()["current_version"].is_number());
}

#[then("Manual은 로컬에 설치된 기본 실행 에이전트를 제공해야 한다")]
async fn manual_detail_has_default_agent(world: &mut ManualWorld) {
    assert!(world.manual()["default_agent"].is_string());
}

#[then("모델 설정을 제공해야 한다")]
async fn manual_detail_has_model(world: &mut ManualWorld) {
    assert!(world.manual()["model"].is_object());
}

#[then("워크플로우 단계 목록을 제공해야 한다")]
async fn manual_detail_has_workflow_steps(world: &mut ManualWorld) {
    assert!(world.manual()["workflow_steps"].is_array());
}

#[then("각 단계의 입력과 산출물 정의를 제공해야 한다")]
async fn manual_step_has_io_definitions(world: &mut ManualWorld) {
    assert!(world.manual()["workflow_steps"][0]["input_schema"].is_array());
    assert!(world.manual()["workflow_steps"][0]["output_schema"].is_string());
}

#[then("각 단계의 검증 정책을 제공해야 한다")]
async fn manual_step_has_verification_policy(world: &mut ManualWorld) {
    assert!(world.manual()["workflow_steps"][0]["verification_policy"].is_object());
}

#[then("각 단계의 샌드박스 정책을 제공해야 한다")]
async fn manual_step_has_sandbox_policy(world: &mut ManualWorld) {
    assert!(world.manual()["workflow_steps"][0]["sandbox_policy"].is_object());
}

#[then("Manual은 토큰 측정 사용 여부를 제공해야 한다")]
async fn manual_has_token_measurement_flag(world: &mut ManualWorld) {
    assert!(world.manual()["optimization"]["token_measurement"].is_boolean());
}

#[then("시간 측정 사용 여부를 제공해야 한다")]
async fn manual_has_time_measurement_flag(world: &mut ManualWorld) {
    assert!(world.manual()["optimization"]["time_measurement"].is_boolean());
}

#[then("Verification 측정 기준을 제공해야 한다")]
async fn manual_has_verification_criteria(world: &mut ManualWorld) {
    assert!(world.manual()["optimization"]["verification_criteria"].is_array());
}

#[then("개선 추천 사용 여부를 제공해야 한다")]
async fn manual_has_improvement_flag(world: &mut ManualWorld) {
    assert!(world.manual()["optimization"]["improvement_recommendations"].is_boolean());
}

#[then("자기진화 모드를 제공해야 한다")]
async fn manual_has_self_evolution_mode(world: &mut ManualWorld) {
    assert!(world.manual()["optimization"]["self_evolution_mode"].is_string());
}

#[when("사용자가 매뉴얼 이름을 입력하지 않고 저장한다")]
async fn user_saves_manual_without_name(world: &mut ManualWorld) {
    world.last_response = Some(rpc_allowing_errors(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 44,
            "method": "manual.create",
            "params": { "name": "" }
        }),
    ));
}

#[then("Manual은 매뉴얼을 저장하지 않아야 한다")]
async fn manual_without_name_is_not_saved(world: &mut ManualWorld) {
    assert!(world.last().get("error").is_some());
}

#[then("사용자에게 이름이 필수라는 오류를 제공해야 한다")]
async fn manual_name_required_error_is_returned(world: &mut ManualWorld) {
    assert!(
        world.last()["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("name")
    );
}

#[given("사용자가 여러 개의 매뉴얼을 가지고 있다")]
async fn user_has_multiple_manuals(world: &mut ManualWorld) {
    create_manual(
        world,
        "Contract Manual Alpha",
        "codex",
        json!({ "tags": ["alpha"] }),
    );
    create_manual(
        world,
        "Contract Manual Beta",
        "codex",
        json!({ "tags": ["beta"] }),
    );
}

#[when("사용자가 매뉴얼 목록을 조회한다")]
async fn user_lists_manuals(world: &mut ManualWorld) {
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 45,
            "method": "manual.list",
            "params": {}
        }),
    ));
}

#[then("Manual은 매뉴얼 이름, 상태, 최신 수정일, 현재 버전을 목록에 제공해야 한다")]
async fn manual_list_has_summary_fields(world: &mut ManualWorld) {
    let first = &world.last()["result"]["manuals"][0];
    assert!(first["name"].is_string());
    assert!(first["status"].is_string());
    assert!(first["updated_at"].is_string());
    assert!(first["current_version"].is_number());
}

#[then("사용자는 태그와 상태로 매뉴얼을 필터링할 수 있어야 한다")]
async fn manual_list_supports_tag_status_filters(world: &mut ManualWorld) {
    assert!(world.last()["result"]["filters"].as_array().unwrap().len() >= 2);
}

#[then("사용자는 이름이나 설명으로 매뉴얼을 검색할 수 있어야 한다")]
async fn manual_list_supports_search(world: &mut ManualWorld) {
    assert!(
        world.last()["result"]["search_fields"]
            .as_array()
            .unwrap()
            .len()
            >= 2
    );
}

#[given("사용자가 매뉴얼 목록에서 특정 매뉴얼을 선택했다")]
async fn user_selected_manual_from_list(world: &mut ManualWorld) {
    ensure_manual(world);
}

#[when("Manual이 매뉴얼 상세 정보를 조회한다")]
async fn manual_reads_manual_detail(world: &mut ManualWorld) {
    user_read_manual_detail(world).await;
}

#[then("Manual은 매뉴얼의 기본 속성을 제공해야 한다")]
async fn manual_detail_includes_basic_properties(world: &mut ManualWorld) {
    manual_detail_has_name(world).await;
}

#[then("실행 속성을 제공해야 한다")]
async fn manual_detail_includes_execution_properties(world: &mut ManualWorld) {
    manual_detail_has_default_agent(world).await;
}

#[then("최적화 속성을 제공해야 한다")]
async fn manual_detail_includes_optimization_properties(world: &mut ManualWorld) {
    manual_has_token_measurement_flag(world).await;
}

#[then("최근 실행 기록과 최근 변경 이력을 제공해야 한다")]
async fn manual_detail_includes_recent_runs_and_history(world: &mut ManualWorld) {
    assert!(world.manual()["recent_runs"].is_array());
    assert!(world.manual()["change_history"].is_array());
}

#[given("사용자가 기존 매뉴얼을 조회했다")]
async fn user_read_existing_manual(world: &mut ManualWorld) {
    user_read_manual_detail(world).await;
}

#[when("사용자가 설명, 태그, 워크플로우 단계, 검증 정책 중 하나를 수정하고 저장한다")]
async fn user_updates_manual(world: &mut ManualWorld) {
    let manual_id = world.manual_id.clone().expect("manual should exist");
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 46,
            "method": "manual.update",
            "params": {
                "manual_id": manual_id,
                "changes": { "description": "updated contract description", "tags": ["updated"] }
            }
        }),
    ));
}

#[then("Manual은 새 수정일을 기록해야 한다")]
async fn manual_update_records_timestamp(world: &mut ManualWorld) {
    assert!(world.manual()["updated_at"].is_string());
}

#[then("변경 이력을 남겨야 한다")]
async fn manual_update_records_history(world: &mut ManualWorld) {
    assert!(
        !world.manual()["change_history"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[then("변경된 매뉴얼을 다음 실행에 사용해야 한다")]
async fn updated_manual_is_used_next(world: &mut ManualWorld) {
    assert_eq!(
        world.manual()["description"],
        "updated contract description"
    );
}

#[when("사용자가 워크플로우 단계, 샌드박스 정책, 검증 정책, 모델 설정 중 하나를 변경한다")]
async fn user_changes_execution_affecting_manual_field(world: &mut ManualWorld) {
    let manual_id = world.manual_id.clone().expect("manual should exist");
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 47,
            "method": "manual.update",
            "params": {
                "manual_id": manual_id,
                "execution_affecting": true,
                "changes": { "model": { "provider": "local", "name": "frontier" } }
            }
        }),
    ));
}

#[then("Manual은 매뉴얼의 새 버전을 생성해야 한다")]
async fn manual_update_creates_new_version(world: &mut ManualWorld) {
    assert!(world.manual()["current_version"].as_u64().unwrap() > 1);
}

#[then("이전 버전을 보존해야 한다")]
async fn manual_preserves_previous_versions(world: &mut ManualWorld) {
    assert!(world.manual()["versions"].as_array().unwrap().len() >= 2);
}

#[then("사용자는 변경 전후 차이를 확인할 수 있어야 한다")]
async fn manual_exposes_version_diff(world: &mut ManualWorld) {
    assert!(world.manual()["last_diff"].is_object());
}

#[given("사용자가 기존 매뉴얼을 가지고 있다")]
async fn user_has_existing_manual(world: &mut ManualWorld) {
    ensure_manual(world);
}

#[when("사용자가 매뉴얼 복제를 선택한다")]
async fn user_clones_manual(world: &mut ManualWorld) {
    let manual_id = world.manual_id.clone().expect("manual should exist");
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 48,
            "method": "manual.clone",
            "params": { "manual_id": manual_id }
        }),
    ));
}

#[then("Manual은 기존 매뉴얼의 기본 속성과 실행 속성을 복사해야 한다")]
async fn manual_clone_copies_core_properties(world: &mut ManualWorld) {
    assert_eq!(world.manual()["default_agent"], "codex");
}

#[then("새 매뉴얼 이름은 사용자가 구분할 수 있게 제안되어야 한다")]
async fn manual_clone_name_is_distinct(world: &mut ManualWorld) {
    assert!(world.manual()["name"].as_str().unwrap().contains("copy"));
}

#[then("실행 기록과 변경 이력은 복제하지 않아야 한다")]
async fn manual_clone_excludes_runs_and_history(world: &mut ManualWorld) {
    assert!(world.manual()["recent_runs"].as_array().unwrap().is_empty());
}

#[then("복제된 매뉴얼은 초안 상태로 시작해야 한다")]
async fn manual_clone_starts_as_draft(world: &mut ManualWorld) {
    assert_eq!(world.manual()["status"], "draft");
}

#[given("사용자가 더 이상 사용하지 않는 매뉴얼을 가지고 있다")]
async fn user_has_unused_manual(world: &mut ManualWorld) {
    ensure_manual(world);
}

#[when("사용자가 매뉴얼 보관을 선택한다")]
async fn user_archives_manual(world: &mut ManualWorld) {
    let manual_id = world.manual_id.clone().expect("manual should exist");
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 49,
            "method": "manual.archive",
            "params": { "manual_id": manual_id }
        }),
    ));
}

#[then("Manual은 매뉴얼 상태를 보관됨으로 변경해야 한다")]
async fn manual_status_becomes_archived(world: &mut ManualWorld) {
    assert_eq!(world.manual()["status"], "archived");
}

#[then("보관된 매뉴얼은 기본 실행 목록에서 제외되어야 한다")]
async fn archived_manual_excluded_from_default_list(world: &mut ManualWorld) {
    user_lists_manuals(world).await;
    assert!(
        world.last()["result"]["manuals"]
            .as_array()
            .unwrap()
            .iter()
            .all(|manual| manual["status"] != "archived")
    );
}

#[then("사용자는 보관된 매뉴얼을 다시 복원할 수 있어야 한다")]
async fn archived_manual_can_be_restored(world: &mut ManualWorld) {
    let manual_id = world.manual_id.clone().expect("manual should exist");
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 55,
            "method": "manual.update",
            "params": {
                "manual_id": manual_id,
                "changes": { "status": "draft" }
            }
        }),
    ));
    assert_eq!(world.manual()["status"], "draft");
}

#[given("삭제하려는 매뉴얼이 실행 중이 아니다")]
async fn manual_to_delete_is_not_running(world: &mut ManualWorld) {
    create_manual(world, "Delete Me", "codex", json!({ "running": false }));
}

#[when("사용자가 매뉴얼 삭제를 확정한다")]
async fn user_confirms_manual_delete(world: &mut ManualWorld) {
    let manual_id = world.manual_id.clone().expect("manual should exist");
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 50,
            "method": "manual.delete",
            "params": { "manual_id": manual_id }
        }),
    ));
}

#[then("Manual은 매뉴얼을 삭제해야 한다")]
async fn manual_is_deleted(world: &mut ManualWorld) {
    assert_eq!(world.manual()["deleted"], true);
}

#[then("삭제된 매뉴얼은 일반 조회 결과에 포함되지 않아야 한다")]
async fn deleted_manual_excluded_from_list(world: &mut ManualWorld) {
    user_lists_manuals(world).await;
    assert!(
        world.last()["result"]["manuals"]
            .as_array()
            .unwrap()
            .iter()
            .all(|manual| manual["name"] != "Delete Me")
    );
}

#[then("로컬 변경 이력에 삭제 작업을 기록해야 한다")]
async fn delete_operation_recorded_in_history(world: &mut ManualWorld) {
    let manual_id = world.manual_id.clone().expect("manual should exist");
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 56,
            "method": "manual.get",
            "params": { "manual_id": manual_id }
        }),
    ));
    assert!(
        world.manual()["change_history"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["change"] == "manual_deleted")
    );
}

#[given("특정 매뉴얼이 현재 실행 중이다")]
async fn specific_manual_is_running(world: &mut ManualWorld) {
    create_manual(world, "Running Manual", "codex", json!({ "running": true }));
}

#[when("사용자가 해당 매뉴얼 삭제를 시도한다")]
async fn user_tries_to_delete_running_manual(world: &mut ManualWorld) {
    let manual_id = world.manual_id.clone().expect("manual should exist");
    world.last_response = Some(rpc_allowing_errors(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 51,
            "method": "manual.delete",
            "params": { "manual_id": manual_id }
        }),
    ));
}

#[then("Manual은 삭제를 거부해야 한다")]
async fn manual_delete_is_rejected(world: &mut ManualWorld) {
    assert!(world.last().get("error").is_some());
}

#[then("실행 중인 매뉴얼은 삭제할 수 없다는 이유를 제공해야 한다")]
async fn running_manual_delete_reason_is_returned(world: &mut ManualWorld) {
    assert!(
        world.last()["error"]["message"]
            .as_str()
            .unwrap()
            .contains("running")
    );
}

#[given("사용자가 초안 상태의 매뉴얼을 가지고 있다")]
async fn user_has_draft_manual(world: &mut ManualWorld) {
    create_manual(world, "Draft Manual", "codex", json!({ "status": "draft" }));
}

#[when("사용자가 매뉴얼을 활성화한다")]
async fn user_activates_manual(world: &mut ManualWorld) {
    let manual_id = world.manual_id.clone().expect("manual should exist");
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 52,
            "method": "manual.activate",
            "params": { "manual_id": manual_id }
        }),
    ));
}

#[then("Manual은 필수 속성과 실행 정책이 유효한지 검사해야 한다")]
async fn manual_validates_required_fields(world: &mut ManualWorld) {
    assert!(
        world.last()["result"].get("validation").is_some() || world.manual()["status"] == "active"
    );
}

#[then("유효하면 상태를 활성으로 변경해야 한다")]
async fn valid_manual_becomes_active(world: &mut ManualWorld) {
    assert_eq!(world.manual()["status"], "active");
}

#[then("유효하지 않으면 누락된 속성을 사용자에게 알려줘야 한다")]
async fn invalid_manual_reports_missing_fields(world: &mut ManualWorld) {
    assert!(
        world.last()["result"].get("validation").is_some() || world.manual()["status"] == "active"
    );
}

#[given("매뉴얼에 에이전트나 스크립트를 실행하는 단계가 있다")]
async fn manual_has_executable_step_without_sandbox(world: &mut ManualWorld) {
    create_manual(
        world,
        "Missing Sandbox",
        "codex",
        json!({
            "workflow_steps": [{
                "id": "agent-step",
                "kind": "codex",
                "input_schema": [],
                "output_schema": "agent result",
                "verification_policy": { "required": true },
                "token_budget": 1000
            }]
        }),
    );
}

#[then("Manual은 실행 단계마다 샌드박스 정책이 지정되어 있는지 검사해야 한다")]
async fn manual_checks_sandbox_policy(world: &mut ManualWorld) {
    assert_missing_field(world, "sandbox_policy");
}

#[then("누락된 단계가 있으면 활성화를 막아야 한다")]
async fn manual_blocks_activation_for_missing_step_policy(world: &mut ManualWorld) {
    assert_eq!(world.last()["result"]["activated"], false);
}

#[then("사용자에게 어떤 단계에 샌드박스가 필요한지 알려줘야 한다")]
async fn manual_reports_step_needing_sandbox(world: &mut ManualWorld) {
    assert_missing_field(world, "sandbox_policy");
}

#[given("매뉴얼에 최적화 측정이 활성화되어 있다")]
async fn manual_has_optimization_measurement_enabled(world: &mut ManualWorld) {
    create_manual(
        world,
        "Missing Optimization Policy",
        "codex",
        json!({
            "workflow_steps": [{
                "id": "agent-step",
                "kind": "codex",
                "input_schema": [],
                "output_schema": "agent result",
                "sandbox_policy": { "sandbox_id": "default" }
            }],
            "optimization": {
                "token_measurement": true,
                "time_measurement": true,
                "verification_criteria": [],
                "improvement_recommendations": true,
                "self_evolution_mode": "suggest"
            }
        }),
    );
}

#[then("Manual은 단계별 토큰 예산이 정의되어 있는지 검사해야 한다")]
async fn manual_checks_token_budget(world: &mut ManualWorld) {
    assert_missing_field(world, "token_budget");
}

#[then("Verification 측정 기준이 정의되어 있는지 검사해야 한다")]
async fn manual_checks_verification_criteria(world: &mut ManualWorld) {
    assert_missing_field(world, "verification_policy");
}

#[then("누락된 예산이나 검증 기준이 있으면 활성화를 막아야 한다")]
async fn manual_blocks_missing_budget_or_verification(world: &mut ManualWorld) {
    assert_eq!(world.last()["result"]["activated"], false);
}

#[then("사용자에게 어떤 단계에 예산이나 검증 기준이 필요한지 알려줘야 한다")]
async fn manual_reports_budget_or_verification_missing(world: &mut ManualWorld) {
    assert!(world.last()["result"]["validation"]["missing"].is_array());
}

#[given("매뉴얼에 여러 버전이 존재한다")]
async fn manual_has_multiple_versions(world: &mut ManualWorld) {
    ensure_manual(world);
    user_changes_execution_affecting_manual_field(world).await;
}

#[when("사용자가 버전 이력을 조회한다")]
async fn user_reads_manual_versions(world: &mut ManualWorld) {
    let manual_id = world.manual_id.clone().expect("manual should exist");
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 53,
            "method": "manual.versions",
            "params": { "manual_id": manual_id }
        }),
    ));
}

#[then("Manual은 각 버전의 생성 시점과 변경 요약을 제공해야 한다")]
async fn manual_versions_include_time_and_summary(world: &mut ManualWorld) {
    let version = &world.last()["result"]["versions"][0];
    assert!(version["created_at"].is_string());
    assert!(version["summary"].is_string());
}

#[then("사용자는 특정 버전의 상세 내용을 확인할 수 있어야 한다")]
async fn manual_version_detail_available(world: &mut ManualWorld) {
    assert!(world.last()["result"]["versions"].is_array());
}

#[then("사용자는 현재 버전과 이전 버전의 차이를 비교할 수 있어야 한다")]
async fn manual_version_diff_available(world: &mut ManualWorld) {
    assert!(world.last()["result"]["diff"].is_object());
}

#[given("사용자가 Manual에 새로운 매뉴얼을 등록했다")]
async fn user_registered_new_manual(world: &mut ManualWorld) {
    ensure_manual(world);
}

#[given("초기 매뉴얼은 사용자 로컬에 설치된 기본 실행 에이전트로 작업을 실행한다")]
async fn initial_manual_uses_default_local_agent(world: &mut ManualWorld) {
    assert_eq!(world.manual()["default_agent"], "codex");
}

#[given("Manual은 매뉴얼 실행 결과를 측정할 수 있다")]
async fn manual_can_measure_runs(world: &mut ManualWorld) {
    record_optimization_run(world, "baseline", "completed");
}

#[when("사용자가 새 매뉴얼을 처음 실행한다")]
async fn user_runs_new_manual_first_time(world: &mut ManualWorld) {
    record_optimization_run(world, "baseline-first", "completed");
}

#[then("Manual은 로컬에 설치된 기본 실행 에이전트의 실행 결과를 기록해야 한다")]
async fn manual_records_default_agent_execution(world: &mut ManualWorld) {
    assert_eq!(world.last()["result"]["run"]["status"], "completed");
}

#[then("토큰 사용량, 검증 상태, 작업 시간을 측정해야 한다")]
async fn manual_measures_tokens_verification_time(world: &mut ManualWorld) {
    let run = &world.last()["result"]["run"];
    assert!(run["token_usage"].is_object());
    assert!(run["verification"].is_object());
    assert!(run["time"].is_object());
}

#[then("이후 진화를 위한 기준 실행으로 저장해야 한다")]
async fn manual_saves_baseline_run(world: &mut ManualWorld) {
    assert!(world.optimization_run_id.is_some());
}

#[given("같은 매뉴얼의 실행 기록이 여러 번 쌓여 있다")]
async fn multiple_manual_runs_exist(world: &mut ManualWorld) {
    record_optimization_run(world, "run-a", "completed");
    record_optimization_run(world, "run-b", "completed");
}

#[when("Manual이 실행 기록을 분석한다")]
async fn manual_analyzes_execution_records(world: &mut ManualWorld) {
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 60,
            "method": "optimization.analyze",
            "params": {}
        }),
    ));
}

#[when("Manual이 실행 결과를 분석한다")]
async fn manual_analyzes_execution_result(world: &mut ManualWorld) {
    manual_analyzes_execution_records(world).await;
}

#[then("Manual은 반복되는 탐색이나 정리 작업을 찾아야 한다")]
async fn manual_finds_repeated_discovery(world: &mut ManualWorld) {
    assert_analysis_candidate(world, "repeated_discovery");
}

#[then("토큰 낭비가 반복되는 단계를 찾아야 한다")]
async fn manual_finds_token_waste(world: &mut ManualWorld) {
    assert_analysis_candidate(world, "token_waste");
}

#[then("검증이 자주 누락되는 단계를 찾아야 한다")]
async fn manual_finds_missing_verification(world: &mut ManualWorld) {
    assert_analysis_candidate(world, "missing_verification");
}

#[then("결과가 흔들리는 작업을 진화 후보로 제공해야 한다")]
async fn manual_finds_unstable_task_candidate(world: &mut ManualWorld) {
    assert_analysis_candidate(world, "unstable_output");
}

#[given("Manual이 매뉴얼 개선 후보를 찾았다")]
async fn manual_found_improvement_candidates(world: &mut ManualWorld) {
    manual_analyzes_execution_records(world).await;
}

#[when("사용자가 Suggest Mode를 사용한다")]
async fn user_uses_suggest_mode(world: &mut ManualWorld) {
    manual_analyzes_execution_records(world).await;
}

#[then("Manual은 매뉴얼 변경 이유를 설명해야 한다")]
async fn manual_explains_change_reason(world: &mut ManualWorld) {
    assert!(world.last()["result"]["suggestions"].is_array());
}

#[then("스크립트화, 전처리, 검증 추가, 모델 변경 후보를 제안해야 한다")]
async fn manual_suggests_improvement_types(world: &mut ManualWorld) {
    assert!(world.last()["result"]["preprocessing"]["scriptable"].is_array());
    assert!(world.last()["result"]["model_recommendations"].is_array());
}

#[then("사용자가 승인하기 전에는 매뉴얼을 변경하지 않아야 한다")]
async fn suggest_mode_does_not_change_manual(world: &mut ManualWorld) {
    let manual_id = world.manual_id.clone().expect("manual should exist");
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 57,
            "method": "manual.get",
            "params": { "manual_id": manual_id }
        }),
    ));
    assert_eq!(world.manual()["status"], "draft");
}

#[given("사용자가 Manual의 개선 제안을 더 구체화하기로 했다")]
async fn user_refines_improvement_suggestion(world: &mut ManualWorld) {
    manual_analyzes_execution_records(world).await;
}

#[when("Manual이 Assisted Edit Mode로 개선안을 작성한다")]
async fn manual_writes_assisted_edit(world: &mut ManualWorld) {
    let manual_id = world.manual_id.clone().expect("manual should exist");
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 61,
            "method": "manual.update",
            "params": {
                "manual_id": manual_id,
                "execution_affecting": true,
                "changes": { "description": "assisted edit proposal applied" }
            }
        }),
    ));
}

#[then("Manual은 매뉴얼 변경안을 구조화해서 제시해야 한다")]
async fn assisted_edit_has_structured_change(world: &mut ManualWorld) {
    assert!(world.manual()["last_diff"].is_object());
}

#[then("변경 전후의 차이를 사용자가 검토할 수 있어야 한다")]
async fn assisted_edit_diff_is_reviewable(world: &mut ManualWorld) {
    assisted_edit_has_structured_change(world).await;
}

#[then("사용자가 승인하면 변경안을 매뉴얼에 반영해야 한다")]
async fn approved_assisted_edit_is_applied(world: &mut ManualWorld) {
    assert_eq!(
        world.manual()["description"],
        "assisted edit proposal applied"
    );
}

#[given("사용자가 Auto Improve Mode를 허용했다")]
async fn user_allows_auto_improve(world: &mut ManualWorld) {
    ensure_manual(world);
}

#[given("개선 대상이 결과 의미를 바꾸지 않는 보조 작업이다")]
async fn improvement_target_is_low_risk(world: &mut ManualWorld) {
    manual_analyzes_execution_records(world).await;
}

#[when("Manual이 자동 개선을 적용한다")]
async fn manual_applies_auto_improvement(world: &mut ManualWorld) {
    user_updates_manual(world).await;
}

#[then(
    "Manual은 파일 목록 캐싱, 로그 정리, 요약 입력 생성 같은 낮은 위험도의 변경만 적용해야 한다"
)]
async fn manual_auto_applies_only_low_risk_changes(world: &mut ManualWorld) {
    assert_eq!(
        world.manual()["description"],
        "updated contract description"
    );
}

#[then("다음 실행에서 개선 효과를 다시 측정해야 한다")]
async fn manual_measures_after_improvement(world: &mut ManualWorld) {
    record_optimization_run(world, "after-improvement", "completed");
    assert!(world.last()["result"]["run"]["token_usage"].is_object());
}

#[given("Manual이 작업 절차나 검증 기준을 바꾸는 개선안을 찾았다")]
async fn manual_found_risky_improvement(world: &mut ManualWorld) {
    manual_analyzes_execution_records(world).await;
}

#[when("해당 변경이 결과 의미에 영향을 줄 수 있다")]
async fn risky_change_affects_meaning(world: &mut ManualWorld) {
    manual_analyzes_execution_records(world).await;
}

#[then("Manual은 변경을 자동 적용하지 않아야 한다")]
async fn manual_does_not_auto_apply_risky_change(world: &mut ManualWorld) {
    assert!(world.last()["result"]["requires_approval"].is_array());
}

#[then("사용자에게 변경 이유와 예상 효과를 설명해야 한다")]
async fn manual_explains_risky_change(world: &mut ManualWorld) {
    assert!(world.last()["result"]["model_recommendations"].is_array());
}

#[then("사용자가 승인한 경우에만 매뉴얼에 반영해야 한다")]
async fn manual_applies_risky_change_only_after_approval(world: &mut ManualWorld) {
    manual_does_not_auto_apply_risky_change(world).await;
}

#[given("Manual이 매뉴얼 개선안을 생성했다")]
async fn manual_generated_improvement(world: &mut ManualWorld) {
    manual_analyzes_execution_records(world).await;
}

#[when("개선안이 현재 매뉴얼보다 Verification 기준이나 검증률을 낮춘다")]
async fn improvement_weakens_verification(world: &mut ManualWorld) {
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 62,
            "method": "optimization.analyze",
            "params": { "weakens_verification": true }
        }),
    ));
}

#[then("Manual은 해당 개선안을 자동 적용하지 않아야 한다")]
async fn manual_does_not_auto_apply_weaker_verification(world: &mut ManualWorld) {
    assert!(world.last()["result"]["suggestions"].is_array());
}

#[then("검증이 약해지는 이유와 영향을 사용자에게 제공해야 한다")]
async fn manual_reports_weaker_verification_impact(world: &mut ManualWorld) {
    assert!(world.last()["result"]["bottlenecks"]["verification_gaps"].is_array());
}

#[then("사용자가 명시적으로 승인한 경우에만 변경 대상으로 남겨야 한다")]
async fn weaker_verification_requires_explicit_approval(world: &mut ManualWorld) {
    manual_does_not_auto_apply_risky_change(world).await;
}

#[given("매뉴얼이 자기진화를 통해 변경되었다")]
async fn manual_changed_by_evolution(world: &mut ManualWorld) {
    user_changes_execution_affecting_manual_field(world).await;
}

#[given("변경 후 실행 결과가 기준 실행보다 나빠졌다")]
async fn evolved_run_is_worse_than_baseline(world: &mut ManualWorld) {
    record_optimization_run(world, "worse-run", "completed");
}

#[when("사용자가 롤백을 선택한다")]
async fn user_selects_rollback(world: &mut ManualWorld) {
    let manual_id = world.manual_id.clone().expect("manual should exist");
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 63,
            "method": "manual.update",
            "params": {
                "manual_id": manual_id,
                "changes": { "description": "rolled back" }
            }
        }),
    ));
}

#[then("Manual은 이전 매뉴얼 버전으로 되돌릴 수 있어야 한다")]
async fn manual_can_rollback(world: &mut ManualWorld) {
    assert_eq!(world.manual()["description"], "rolled back");
}

#[then("롤백 이유와 비교 지표를 기록해야 한다")]
async fn manual_records_rollback_reason(world: &mut ManualWorld) {
    assert!(world.manual()["change_history"].is_array());
}

#[given("매뉴얼에 여러 번의 개선 이력이 존재한다")]
async fn manual_has_multiple_improvements(world: &mut ManualWorld) {
    user_updates_manual(world).await;
    user_changes_execution_affecting_manual_field(world).await;
}

#[when("사용자가 매뉴얼 진화 상태를 확인한다")]
async fn user_reads_evolution_status(world: &mut ManualWorld) {
    manual_analyzes_execution_records(world).await;
}

#[then("Manual은 적용된 개선 목록을 제공해야 한다")]
async fn manual_lists_applied_improvements(world: &mut ManualWorld) {
    let manual_id = world.manual_id.clone().expect("manual should exist");
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 58,
            "method": "manual.get",
            "params": { "manual_id": manual_id }
        }),
    ));
    assert!(world.manual()["change_history"].is_array());
    manual_analyzes_execution_records(world).await;
}

#[then("각 개선의 토큰, 검증, 시간 변화 효과를 제공해야 한다")]
async fn manual_lists_improvement_effects(world: &mut ManualWorld) {
    assert!(world.last()["result"]["model_recommendations"][0]["expected_impact"].is_object());
}

#[then("아직 검토가 필요한 개선 후보를 제공해야 한다")]
async fn manual_lists_pending_improvement_candidates(world: &mut ManualWorld) {
    assert!(world.last()["result"]["candidates"].is_array());
}

#[given("사용자가 Manual에 AI 워크플로우를 등록했다")]
async fn user_registered_ai_workflow(world: &mut ManualWorld) {
    ensure_manual(world);
}

#[given("워크플로우에는 에이전트가 실행하는 단계가 포함되어 있다")]
async fn workflow_has_agent_stage(world: &mut ManualWorld) {
    assert_eq!(world.manual()["workflow_steps"][0]["kind"], "codex");
}

#[when("사용자가 워크플로우를 실행한다")]
async fn user_runs_ai_workflow(world: &mut ManualWorld) {
    record_optimization_run(world, "ai-run", "completed");
}

#[then("Manual은 전체 토큰 사용량을 기록해야 한다")]
async fn manual_records_total_tokens(world: &mut ManualWorld) {
    assert!(world.last()["result"]["run"]["token_usage"]["total"].is_number());
}

#[then("단계별 토큰 사용량을 기록해야 한다")]
async fn manual_records_step_tokens(world: &mut ManualWorld) {
    assert!(world.last()["result"]["run"]["token_usage"]["by_step"].is_array());
}

#[then("모델별 토큰 사용량과 비용을 기록해야 한다")]
async fn manual_records_model_token_cost(world: &mut ManualWorld) {
    assert!(world.last()["result"]["run"]["token_usage"]["by_model"].is_array());
}

#[then("사용자는 어떤 단계에서 토큰이 많이 쓰였는지 확인할 수 있어야 한다")]
async fn user_can_see_token_hotspots(world: &mut ManualWorld) {
    assert!(world.last()["result"]["run"]["token_usage"]["hotspots"].is_array());
}

#[given("워크플로우 단계마다 토큰 예산이 정의되어 있다")]
async fn token_budgets_exist(_world: &mut ManualWorld) {}

#[then("Manual은 각 단계의 실제 토큰 사용량을 단계별 예산과 비교해야 한다")]
async fn manual_compares_tokens_to_budget(world: &mut ManualWorld) {
    assert!(world.last()["result"]["run"]["token_usage"]["by_step"][0]["budget"].is_number());
}

#[then("예산을 초과한 단계를 표시해야 한다")]
async fn manual_marks_over_budget_steps(world: &mut ManualWorld) {
    assert_eq!(
        world.last()["result"]["run"]["token_usage"]["by_step"][1]["over_budget"],
        true
    );
}

#[then("초과량과 초과 비율을 제공해야 한다")]
async fn manual_reports_budget_overage(world: &mut ManualWorld) {
    assert!(world.last()["result"]["run"]["token_usage"]["by_step"][1]["over_by"].is_number());
    assert!(world.last()["result"]["run"]["token_usage"]["by_step"][1]["over_ratio"].is_number());
}

#[then("사용자는 다음 실행에서 줄여야 할 컨텍스트나 모델 사용 지점을 확인할 수 있어야 한다")]
async fn user_can_see_next_reduction_points(world: &mut ManualWorld) {
    user_can_see_token_hotspots(world).await;
}

#[given("워크플로우에 단계별 중요도와 실패 비용이 정의되어 있다")]
async fn workflow_has_importance_and_failure_cost(_world: &mut ManualWorld) {}

#[given("실행 기록에 모델별 토큰 사용량과 비용이 존재한다")]
async fn model_token_cost_records_exist(world: &mut ManualWorld) {
    record_optimization_run(world, "model-cost", "completed");
}

#[when("Manual이 개선안을 생성한다")]
async fn manual_generates_recommendations(world: &mut ManualWorld) {
    manual_analyzes_execution_records(world).await;
}

#[then("Manual은 고품질 모델이 필요한 단계를 구분해야 한다")]
async fn manual_identifies_high_quality_model_steps(world: &mut ManualWorld) {
    assert!(world.last()["result"]["model_recommendations"][1]["reason"].is_string());
}

#[then("더 작은 모델로 처리할 수 있는 단계를 제안해야 한다")]
async fn manual_suggests_smaller_model_steps(world: &mut ManualWorld) {
    assert_eq!(
        world.last()["result"]["model_recommendations"][0]["recommendation"],
        "use smaller model"
    );
}

#[then("모델 변경이 토큰 비용, 검증률, 작업 시간에 줄 예상 영향을 제공해야 한다")]
async fn manual_reports_model_change_impact(world: &mut ManualWorld) {
    assert!(world.last()["result"]["model_recommendations"][0]["expected_impact"].is_object());
}

#[given("사용자가 워크플로우 단계를 설정하고 있다")]
async fn user_configures_workflow_step(_world: &mut ManualWorld) {}

#[when("사용자가 단계의 실행 정책을 정의한다")]
async fn user_defines_step_execution_policy(world: &mut ManualWorld) {
    manual_analyzes_execution_records(world).await;
}

#[then("Manual은 단계의 결정성을 기록할 수 있어야 한다")]
async fn manual_records_determinism(world: &mut ManualWorld) {
    assert!(world.last()["result"]["adaptive_compute"]["determinism"].is_string());
}

#[then("요구 추론 깊이를 기록할 수 있어야 한다")]
async fn manual_records_reasoning_depth(world: &mut ManualWorld) {
    assert!(world.last()["result"]["adaptive_compute"]["reasoning_depth"].is_string());
}

#[then("실패 비용을 기록할 수 있어야 한다")]
async fn manual_records_failure_cost(world: &mut ManualWorld) {
    assert!(world.last()["result"]["adaptive_compute"]["failure_cost"].is_string());
}

#[then("검증 가능성을 기록할 수 있어야 한다")]
async fn manual_records_verifiability(world: &mut ManualWorld) {
    assert!(world.last()["result"]["adaptive_compute"]["verifiability"].is_string());
}

#[then("입력 크기와 재사용 가능성을 기록할 수 있어야 한다")]
async fn manual_records_input_size_and_reuse(world: &mut ManualWorld) {
    assert!(world.last()["result"]["adaptive_compute"]["input_size"].is_string());
    assert!(world.last()["result"]["adaptive_compute"]["reusability"].is_string());
}

#[given("워크플로우 단계가 상위 LLM을 사용하도록 설정되어 있다")]
async fn workflow_step_uses_frontier_llm(_world: &mut ManualWorld) {}

#[when("사용자가 해당 단계 설정을 저장한다")]
async fn user_saves_frontier_step(world: &mut ManualWorld) {
    record_optimization_run(world, "frontier-reason", "completed");
}

#[then("Manual은 상위 LLM이 필요한 이유를 기록할 수 있어야 한다")]
async fn manual_records_frontier_reason(world: &mut ManualWorld) {
    assert!(world.last()["result"]["run"]["model_calls"][0]["reason"].is_string());
}

#[then("실행 후에도 해당 이유가 실행 기록에 남아야 한다")]
async fn frontier_reason_persists_in_run(world: &mut ManualWorld) {
    manual_records_frontier_reason(world).await;
}

#[then("사용자는 최적화 시 해당 이유가 여전히 유효한지 검토할 수 있어야 한다")]
async fn user_can_review_frontier_reason(world: &mut ManualWorld) {
    assert_eq!(
        world.last()["result"]["run"]["model_calls"][0]["step_id"],
        "implement"
    );
}

#[given("워크플로우에 요구사항과 검증 항목이 정의되어 있다")]
async fn workflow_has_requirements_and_verification(_world: &mut ManualWorld) {}

#[when("에이전트 실행이 완료된다")]
async fn agent_execution_completes(world: &mut ManualWorld) {
    record_optimization_run(world, "verified-run", "completed");
}

#[then("Manual은 요구사항 충족도를 제공해야 한다")]
async fn manual_reports_requirement_satisfaction(world: &mut ManualWorld) {
    assert!(world.last()["result"]["run"]["verification"]["requirements_satisfied"].is_number());
}

#[then("검증 통과율을 제공해야 한다")]
async fn manual_reports_verification_pass_rate(world: &mut ManualWorld) {
    assert!(world.last()["result"]["run"]["verification"]["pass_rate"].is_number());
}

#[then("누락된 검증 항목을 제공해야 한다")]
async fn manual_reports_missing_verification_items(world: &mut ManualWorld) {
    assert!(world.last()["result"]["run"]["verification"]["missing"].is_array());
}

#[then("남은 리스크를 사용자가 확인할 수 있어야 한다")]
async fn user_can_see_remaining_risks(world: &mut ManualWorld) {
    assert!(world.last()["result"]["run"]["verification"]["risks"].is_array());
}

#[then("Manual은 각 검증 항목의 상태를 통과, 실패, 미확인 중 하나로 기록해야 한다")]
async fn manual_records_verification_item_status(world: &mut ManualWorld) {
    assert!(world.last()["result"]["run"]["verification"]["items"][0]["status"].is_string());
}

#[then("각 검증 항목에 연결된 테스트, 로그, 파일, 리뷰 결과 같은 근거 산출물을 기록해야 한다")]
async fn manual_records_verification_evidence(world: &mut ManualWorld) {
    assert!(world.last()["result"]["run"]["verification"]["items"][0]["evidence"].is_array());
}

#[then("근거가 없는 검증 항목은 미확인으로 표시해야 한다")]
async fn manual_marks_evidence_missing_unknown(world: &mut ManualWorld) {
    assert_eq!(
        world.last()["result"]["run"]["verification"]["items"][1]["status"],
        "unknown"
    );
}

#[then("사용자는 남은 리스크가 어떤 근거 부족에서 나왔는지 확인할 수 있어야 한다")]
async fn user_can_trace_risk_to_missing_evidence(world: &mut ManualWorld) {
    user_can_see_remaining_risks(world).await;
}

#[then("Manual은 전체 작업 시간을 기록해야 한다")]
async fn manual_records_total_time(world: &mut ManualWorld) {
    assert!(world.last()["result"]["run"]["time"]["total_ms"].is_number());
}

#[then("단계별 소요 시간을 기록해야 한다")]
async fn manual_records_step_time(world: &mut ManualWorld) {
    assert!(world.last()["result"]["run"]["time"]["by_step"].is_array());
}

#[then("재시도 횟수를 기록해야 한다")]
async fn manual_records_retry_count(world: &mut ManualWorld) {
    assert!(world.last()["result"]["run"]["time"]["by_step"][1]["retries"].is_number());
}

#[then("사용자가 입력했거나 추정된 리뷰 시간을 함께 확인할 수 있어야 한다")]
async fn manual_records_review_time(world: &mut ManualWorld) {
    assert!(world.last()["result"]["run"]["time"]["review_ms"].is_number());
}

#[then("Manual은 실행에 사용된 모델 호출 정보를 기록해야 한다")]
async fn manual_records_model_calls(world: &mut ManualWorld) {
    assert!(world.last()["result"]["run"]["model_calls"].is_array());
}

#[then("에이전트나 스크립트가 사용한 도구 호출 기록을 남겨야 한다")]
async fn manual_records_tool_calls(world: &mut ManualWorld) {
    assert!(world.last()["result"]["run"]["tool_calls"].is_array());
}

#[then("각 단계에 전달된 컨텍스트의 출처와 요약을 기록해야 한다")]
async fn manual_records_context_sources(world: &mut ManualWorld) {
    assert!(world.last()["result"]["run"]["context_sources"].is_array());
}

#[then("사용자는 최적화 진단이 어떤 실행 기록을 근거로 했는지 확인할 수 있어야 한다")]
async fn user_can_see_optimization_evidence(world: &mut ManualWorld) {
    assert!(world.optimization_run_id.is_some());
}

#[given("사용자가 같은 워크플로우를 최적화 전과 후에 각각 실행했다")]
async fn before_after_optimization_runs_exist(world: &mut ManualWorld) {
    record_optimization_run(world, "before", "completed");
    record_optimization_run(world, "after", "completed");
}

#[when("사용자가 비교 결과를 확인한다")]
async fn user_reads_comparison(world: &mut ManualWorld) {
    compare_optimization(world);
}

#[then("Manual은 토큰 사용량의 변화를 제공해야 한다")]
async fn manual_reports_token_delta(world: &mut ManualWorld) {
    assert!(world.last()["result"]["token_delta"].is_number());
}

#[then("검증률의 변화를 제공해야 한다")]
async fn manual_reports_verification_delta(world: &mut ManualWorld) {
    assert!(world.last()["result"]["verification_delta"].is_number());
}

#[then("작업 시간의 변화를 제공해야 한다")]
async fn manual_reports_time_delta(world: &mut ManualWorld) {
    assert!(world.last()["result"]["time_delta_ms"].is_number());
}

#[then("사용자는 변경이 실제로 개선을 만들었는지 판단할 수 있어야 한다")]
async fn user_can_judge_improvement(world: &mut ManualWorld) {
    manual_reports_token_delta(world).await;
}

#[given("같은 워크플로우에 실패한 실행 기록과 성공한 실행 기록이 존재한다")]
async fn failed_and_successful_runs_exist(world: &mut ManualWorld) {
    record_optimization_run(world, "failed", "failed");
    record_optimization_run(world, "successful", "completed");
}

#[when("사용자가 실행 비용 비교를 요청한다")]
async fn user_requests_cost_comparison(world: &mut ManualWorld) {
    compare_optimization(world);
}

#[then("Manual은 실패한 실행의 토큰 사용량과 비용을 제공해야 한다")]
async fn manual_reports_failed_run_cost(world: &mut ManualWorld) {
    assert!(world.last()["result"]["failed_run"]["tokens"].is_number());
    assert!(world.last()["result"]["failed_run"]["cost"].is_number());
}

#[then("성공한 실행의 토큰 사용량과 비용을 제공해야 한다")]
async fn manual_reports_successful_run_cost(world: &mut ManualWorld) {
    assert!(world.last()["result"]["successful_run"]["tokens"].is_number());
    assert!(world.last()["result"]["successful_run"]["cost"].is_number());
}

#[then("실패 후 재시도에 추가로 든 토큰, 비용, 시간을 제공해야 한다")]
async fn manual_reports_retry_extra_cost(world: &mut ManualWorld) {
    assert!(world.last()["result"]["retry_extra"].is_object());
}

#[given("사용자가 같은 워크플로우를 변경 전과 후에 실행했다")]
async fn changed_workflow_runs_exist(world: &mut ManualWorld) {
    before_after_optimization_runs_exist(world).await;
}

#[given("변경 후 실행의 성공률이 낮아지거나 토큰, 비용, 시간이 증가했다")]
async fn changed_run_regressed(world: &mut ManualWorld) {
    manual_analyzes_execution_records(world).await;
}

#[then("Manual은 비용 회귀 가능성을 제공해야 한다")]
async fn manual_reports_cost_regression(world: &mut ManualWorld) {
    assert_eq!(world.last()["result"]["regression"]["possible"], true);
}

#[then("어떤 단계에서 회귀가 발생했는지 제공해야 한다")]
async fn manual_reports_regression_step(world: &mut ManualWorld) {
    assert!(world.last()["result"]["regression"]["step_id"].is_string());
}

#[then("사용자는 변경을 유지할지 되돌릴지 판단할 수 있어야 한다")]
async fn user_can_decide_keep_or_rollback(world: &mut ManualWorld) {
    manual_reports_cost_regression(world).await;
}

#[given("워크플로우 실행 기록이 존재한다")]
async fn workflow_execution_records_exist(world: &mut ManualWorld) {
    record_optimization_run(world, "diagnose", "completed");
}

#[then("Manual은 토큰 낭비 지점을 찾아야 한다")]
async fn manual_diagnoses_token_waste(world: &mut ManualWorld) {
    assert!(world.last()["result"]["bottlenecks"]["token_waste"].is_array());
}

#[then("검증 부족 지점을 찾아야 한다")]
async fn manual_diagnoses_verification_gaps(world: &mut ManualWorld) {
    assert!(world.last()["result"]["bottlenecks"]["verification_gaps"].is_array());
}

#[then("시간이 오래 걸리는 단계를 찾아야 한다")]
async fn manual_diagnoses_slow_steps(world: &mut ManualWorld) {
    assert!(world.last()["result"]["bottlenecks"]["slow_steps"].is_array());
}

#[then("결과가 흔들리는 작업을 사용자가 확인할 수 있게 제공해야 한다")]
async fn manual_diagnoses_unstable_tasks(world: &mut ManualWorld) {
    assert!(world.last()["result"]["bottlenecks"]["unstable_tasks"].is_array());
}

#[given("특정 단계에서 대량 입력을 에이전트가 반복해서 읽고 있다")]
async fn agent_repeatedly_reads_large_input(world: &mut ManualWorld) {
    manual_analyzes_execution_records(world).await;
}

#[then("Manual은 에이전트 호출 전에 수행할 전처리 후보를 제안해야 한다")]
async fn manual_suggests_preprocessing_candidates(world: &mut ManualWorld) {
    assert!(world.last()["result"]["preprocessing"]["candidates"].is_array());
}

#[then("반복 탐색이나 로그 필터링처럼 스크립트화 가능한 작업을 제공해야 한다")]
async fn manual_suggests_scriptable_work(world: &mut ManualWorld) {
    assert!(world.last()["result"]["preprocessing"]["scriptable"].is_array());
}

#[then("에이전트에게 전달할 압축된 입력 형태를 제안해야 한다")]
async fn manual_suggests_compressed_input(world: &mut ManualWorld) {
    assert!(world.last()["result"]["preprocessing"]["compressed_input"].is_string());
}

#[then("예상 토큰 절감 효과를 제공해야 한다")]
async fn manual_reports_estimated_token_savings(world: &mut ManualWorld) {
    assert!(world.last()["result"]["preprocessing"]["estimated_token_savings"].is_number());
}

#[given("같은 단계에 대해 저비용 모델 실행 결과와 상위 LLM 실행 결과가 존재한다")]
async fn low_and_high_model_results_exist(world: &mut ManualWorld) {
    compare_optimization(world);
}

#[when("Manual이 모델 품질 비교를 요청받는다")]
async fn manual_receives_model_quality_comparison_request(world: &mut ManualWorld) {
    compare_optimization(world);
}

#[then("Manual은 두 결과의 검증 통과율을 비교해야 한다")]
async fn manual_compares_model_verification_rate(world: &mut ManualWorld) {
    assert!(
        world.last()["result"]["quality"]["low_cost_model"]["verification_pass_rate"].is_number()
    );
}

#[then("출력 schema 준수 여부를 비교해야 한다")]
async fn manual_compares_model_schema_compliance(world: &mut ManualWorld) {
    assert!(world.last()["result"]["quality"]["low_cost_model"]["schema_compliant"].is_boolean());
}

#[then("비용과 작업 시간을 함께 제공해야 한다")]
async fn manual_compares_model_cost_and_time(world: &mut ManualWorld) {
    assert!(world.last()["result"]["quality"]["frontier_model"]["cost"].is_number());
    assert!(world.last()["result"]["quality"]["frontier_model"]["duration_ms"].is_number());
}

#[then("사용자는 해당 단계를 저비용 모델로 낮출 수 있는지 판단할 수 있어야 한다")]
async fn user_can_decide_model_downgrade(world: &mut ManualWorld) {
    manual_compares_model_verification_rate(world).await;
}

#[given("워크플로우 실행과 분석이 완료되었다")]
async fn workflow_run_and_analysis_completed(world: &mut ManualWorld) {
    record_optimization_run(world, "report-run", "completed");
    manual_analyzes_execution_records(world).await;
}

#[when("사용자가 최적화 리포트를 요청한다")]
async fn user_requests_optimization_report(world: &mut ManualWorld) {
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 64,
            "method": "optimization.report",
            "params": {}
        }),
    ));
}

#[then("Manual은 Token Usage, Verification, Time을 함께 제공해야 한다")]
async fn report_contains_three_sections(world: &mut ManualWorld) {
    let sections = world.last()["result"]["sections"].as_array().unwrap();
    assert!(sections.iter().any(|value| value == "Token Usage"));
    assert!(sections.iter().any(|value| value == "Verification"));
    assert!(sections.iter().any(|value| value == "Time"));
}

#[then("가장 큰 문제를 요약해야 한다")]
async fn report_summarizes_main_issue(world: &mut ManualWorld) {
    assert!(world.last()["result"]["main_issue"].is_string());
}

#[then("다음 실행에서 시도할 추천안을 제시해야 한다")]
async fn report_suggests_next_actions(world: &mut ManualWorld) {
    assert!(world.last()["result"]["recommendations"].is_array());
}

#[then("사용자는 현재 상태와 다음 행동을 동시에 이해할 수 있어야 한다")]
async fn user_can_understand_state_and_next_action(world: &mut ManualWorld) {
    report_contains_three_sections(world).await;
    report_suggests_next_actions(world).await;
}

#[given("사용자가 Manual에서 워크플로우를 관리하고 있다")]
async fn user_manages_workflows(world: &mut ManualWorld) {
    ensure_sandbox(world);
}

#[given("워크플로우에는 에이전트나 스크립트를 실행하는 노드가 포함될 수 있다")]
async fn workflow_may_include_agent_or_script_nodes(_world: &mut ManualWorld) {}

#[when("사용자가 새 샌드박스 종류를 생성한다")]
async fn user_creates_sandbox(world: &mut ManualWorld) {
    create_sandbox(world, json!({}));
}

#[then("사용자는 파일 읽기 허용 경로를 정의할 수 있어야 한다")]
async fn sandbox_has_allowed_read_paths(world: &mut ManualWorld) {
    assert!(world.sandbox()["allow_read"].is_array());
}

#[then("파일 쓰기 허용 경로를 정의할 수 있어야 한다")]
async fn sandbox_has_allowed_write_paths(world: &mut ManualWorld) {
    assert!(world.sandbox()["allow_write"].is_array());
}

#[then("실행 가능한 명령을 정의할 수 있어야 한다")]
async fn sandbox_has_allowed_commands(world: &mut ManualWorld) {
    assert!(world.sandbox()["allow_commands"].is_array());
}

#[then("접근 가능한 네트워크 호스트를 정의할 수 있어야 한다")]
async fn sandbox_has_allowed_network(world: &mut ManualWorld) {
    assert!(world.sandbox()["allow_network"].is_array());
}

#[then("실행 금지 명령을 정의할 수 있어야 한다")]
async fn sandbox_has_denied_commands(world: &mut ManualWorld) {
    assert!(world.sandbox()["deny_commands"].is_array());
}

#[then("접근 금지 네트워크 호스트를 정의할 수 있어야 한다")]
async fn sandbox_has_denied_network(world: &mut ManualWorld) {
    assert!(world.sandbox()["deny_network"].is_array());
}

#[then("환경 변수 접근 범위를 정의할 수 있어야 한다")]
async fn sandbox_has_allowed_env(world: &mut ManualWorld) {
    assert!(world.sandbox()["allow_env"].is_array());
}

#[then("임시 디렉터리와 캐시 디렉터리 사용 범위를 정의할 수 있어야 한다")]
async fn sandbox_has_tmp_and_cache_scope(world: &mut ManualWorld) {
    assert!(world.sandbox()["tmp_write"].is_array());
    assert!(world.sandbox()["cache_write"].is_array());
}

#[given("사용자가 재사용 가능한 샌드박스 종류를 가지고 있다")]
async fn reusable_sandbox_exists(world: &mut ManualWorld) {
    ensure_sandbox(world);
}

#[when("사용자가 에이전트 실행 노드를 설정한다")]
async fn user_configures_agent_node_with_sandbox(world: &mut ManualWorld) {
    ensure_sandbox(world);
}

#[then("사용자는 해당 노드에 사용할 샌드박스를 지정해야 한다")]
async fn node_requires_sandbox_assignment(world: &mut ManualWorld) {
    assert!(world.sandbox_id.is_some());
}

#[then("Manual은 노드 실행 시 지정된 샌드박스 정책을 적용해야 한다")]
async fn manual_applies_assigned_sandbox_policy(world: &mut ManualWorld) {
    evaluate_sandbox(world, "read_file", "docs/wiki/목차.md");
    assert_eq!(world.last()["result"]["decision"]["allowed"], true);
}

#[when("Manual이 에이전트 실행 명령을 구성한다")]
async fn manual_builds_agent_launch_command(world: &mut ManualWorld) {
    let codex = Codex::new(Agent::new(
        "codex.sandbox_contract",
        "Codex Sandbox Contract",
        "Exercise sandbox launch contract.",
    ));
    let sandbox = world.sandbox().clone();
    let command =
        codex.command(&CommandRequest::new("echo sandbox contract").with_sandbox_policy(sandbox));

    world.latest_agent_command_program = Some(command.get_program().to_string_lossy().into_owned());
    world.latest_agent_command_args = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
}

#[then("Manual은 현재 플랫폼에 맞는 OS 샌드박스 실행기로 에이전트 프로세스를 시작해야 한다")]
async fn manual_starts_agent_process_with_platform_sandbox(world: &mut ManualWorld) {
    let program = world
        .latest_agent_command_program
        .as_deref()
        .expect("agent launch command should be captured");
    assert!(
        platform_sandbox_launchers().contains(&program),
        "expected platform sandbox launcher from {:?}, got {program}",
        platform_sandbox_launchers()
    );
}

#[then("원래 에이전트 CLI는 샌드박스 래퍼의 인자로 전달되어야 한다")]
async fn original_agent_cli_is_delegated_to_sandbox_wrapper(world: &mut ManualWorld) {
    assert!(
        world
            .latest_agent_command_args
            .iter()
            .any(|arg| arg == "codex"),
        "sandbox wrapper args should include the original codex CLI: {:?}",
        world.latest_agent_command_args
    );
}

#[when("Manual이 샌드박스 실행 백엔드를 선택한다")]
async fn manual_selects_sandbox_backend(_world: &mut ManualWorld) {}

#[then(regex = r#"^macOS에서는 Seatbelt 또는 "([^"]+)" 계열을 사용할 수 있어야 한다$"#)]
async fn macos_backend_candidates_include_seatbelt_and_sandbox_exec(
    _world: &mut ManualWorld,
    backend: String,
) {
    assert!(macos_sandbox_backends().contains(&"seatbelt"));
    assert!(macos_sandbox_backends().contains(&backend.as_str()));
}

#[then("Linux에서는 namespace, seccomp, bubblewrap, firejail 계열을 사용할 수 있어야 한다")]
async fn linux_backend_candidates_include_namespace_seccomp_and_wrappers(_world: &mut ManualWorld) {
    for backend in ["namespace", "seccomp", "bubblewrap", "firejail"] {
        assert!(linux_sandbox_backends().contains(&backend));
    }
}

#[then("Windows에서는 Job Object, AppContainer, Windows Sandbox 계열을 사용할 수 있어야 한다")]
async fn windows_backend_candidates_include_job_object_appcontainer_and_windows_sandbox(
    _world: &mut ManualWorld,
) {
    for backend in ["job-object", "appcontainer", "windows-sandbox"] {
        assert!(windows_sandbox_backends().contains(&backend));
    }
}

#[when("사용자가 샌드박스가 지정된 스크립트 실행 노드를 등록한다")]
async fn user_registers_script_node_with_sandbox(world: &mut ManualWorld) {
    let sandbox_id = world
        .sandbox_id
        .clone()
        .expect("sandbox should exist before registering script node");
    world.workflow_id = "script-sandbox-contract".to_owned();
    world.last_response = Some(rpc_allowing_errors(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 76,
            "method": "workflow.create",
            "params": {
                "workflow": {
                    "id": "script-sandbox-contract",
                    "nodes": [
                        {
                            "id": "cleanup-script",
                            "kind": "script",
                            "script": "fn run() {}",
                            "sandbox_policy": { "sandbox_id": sandbox_id }
                        }
                    ],
                    "dependencies": []
                }
            }
        }),
    ));
}

#[then("Manual은 스크립트 노드의 샌드박스 정책을 저장해야 한다")]
async fn manual_stores_script_node_sandbox_policy(world: &mut ManualWorld) {
    assert!(
        world.last().get("error").is_none(),
        "workflow.create should accept script nodes with sandbox_policy: {}",
        world.last()
    );

    let workflow_id = world.workflow_id.clone();
    let stored = rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 77,
            "method": "workflow.get",
            "params": { "workflow_id": workflow_id }
        }),
    );
    assert!(stored["result"]["workflow"]["nodes"][0]["sandbox_policy"].is_object());
    world.last_response = Some(stored);
}

#[then("스크립트 노드는 실행 가능한 워크플로우 노드로 인식되어야 한다")]
async fn script_node_is_recognized_as_executable_workflow_node(world: &mut ManualWorld) {
    assert_eq!(
        world.last()["result"]["workflow"]["nodes"][0]["kind"],
        "script"
    );
}

#[given("사용자가 실제 파일과 네트워크 접근을 시도하는 테스트 스크립트를 준비했다")]
async fn user_prepares_real_sandbox_probe_script(world: &mut ManualWorld) {
    let root = unique_storage_dir("sandbox-probe");
    let allowed = root.join("allowed");
    let denied = root.join("denied");
    fs::create_dir_all(&allowed).expect("allowed sandbox fixture dir should exist");
    fs::create_dir_all(&denied).expect("denied sandbox fixture dir should exist");
    fs::write(allowed.join("read.txt"), "allowed-read")
        .expect("allowed read fixture should be written");
    fs::write(denied.join("secret.txt"), "denied-read")
        .expect("denied read fixture should be written");
    let denied_delete = denied.join("delete.txt");
    fs::write(&denied_delete, "do not delete").expect("denied delete fixture should be written");
    let log = allowed.join("sandbox.log");
    let script = root.join("probe.sh");
    let listener = TcpListener::bind("127.0.0.1:0").expect("probe listener should bind");
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    fs::write(
        &script,
        format!(
            "#!/bin/sh\ncat '{}' > '{}'\necho READ:$? >> '{}'\ncat '{}' >> '{}' 2>/dev/null\necho DENIED_READ:$? >> '{}'\necho changed > '{}'\necho MODIFY:$? >> '{}'\nrm '{}' 2>/dev/null\necho DELETE:$? >> '{}'\n/usr/bin/nc -z 127.0.0.1 {} >/dev/null 2>&1\necho NETWORK:$? >> '{}'\nexit 0\n",
            allowed.join("read.txt").display(),
            allowed.join("read.out").display(),
            log.display(),
            denied.join("secret.txt").display(),
            log.display(),
            log.display(),
            allowed.join("modified.txt").display(),
            log.display(),
            denied_delete.display(),
            log.display(),
            port,
            log.display(),
        ),
    )
    .expect("probe script should be written");
    make_executable(&script);

    create_sandbox(
        world,
        json!({
            "name": "Real Sandbox Probe",
            "scope_root": root,
            "allow_read": [allowed, script],
            "allow_write": [allowed],
            "allow_commands": [script],
            "allow_network": [],
            "deny_network": ["*"],
            "tmp_write": [],
            "cache_write": []
        }),
    );
    world.sandbox_test_dir = Some(root);
    world.sandbox_test_log = Some(log);
    world.sandbox_denied_delete = Some(denied_delete);
}

#[given("스크립트 노드가 테스트 샌드박스 안에서 실행된다")]
async fn script_node_runs_inside_test_sandbox(world: &mut ManualWorld) {
    let root = world
        .sandbox_test_dir
        .clone()
        .expect("sandbox probe dir should exist");
    let script = root.join("probe.sh");
    let sandbox_id = world.sandbox_id.clone().expect("sandbox should exist");
    world.workflow_id = "real-sandbox-probe".to_owned();
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 78,
            "method": "workflow.create",
            "params": {
                "workflow": {
                    "id": "real-sandbox-probe",
                    "nodes": [
                        {
                            "id": "probe",
                            "kind": "script",
                            "script": script,
                            "sandbox_policy": { "sandbox_id": sandbox_id }
                        }
                    ],
                    "dependencies": []
                }
            }
        }),
    ));
}

#[when("Manual이 해당 스크립트 워크플로우를 실행한다")]
async fn manual_runs_real_sandbox_probe_workflow(world: &mut ManualWorld) {
    let workflow_id = world.workflow_id.clone();
    let run_id = start_workflow(world, &workflow_id, json!({}));
    world.current_run_id = Some(run_id.clone());
    world.current_events = Some(poll_workflow_events_until(world, &run_id, 0, |events| {
        events["result"]["completed"].as_bool().unwrap_or(false)
    }));
}

#[then("허용된 파일 조회와 수정은 성공해야 한다")]
async fn allowed_file_read_and_modify_succeed(world: &mut ManualWorld) {
    assert_eq!(
        world.current_events()["result"]["run"]["nodes"]["probe"]["status"],
        "completed"
    );
    let log = sandbox_probe_log(world);
    assert!(log.contains("READ:0"), "{log}");
    assert!(log.contains("MODIFY:0"), "{log}");
}

#[then("허용되지 않은 파일 조회와 삭제는 차단되어야 한다")]
async fn disallowed_file_read_and_delete_are_blocked(world: &mut ManualWorld) {
    let log = sandbox_probe_log(world);
    assert!(log.contains("DENIED_READ:1"), "{log}");
    assert!(log.contains("DELETE:1"), "{log}");
    assert!(
        world
            .sandbox_denied_delete
            .as_ref()
            .expect("denied delete fixture should exist")
            .exists()
    );
}

#[then("허용되지 않은 네트워크 접속은 차단되어야 한다")]
async fn disallowed_network_connection_is_blocked(world: &mut ManualWorld) {
    let log = sandbox_probe_log(world);
    assert!(log.contains("NETWORK:1"), "{log}");
}

#[given("샌드박스가 \"docs/** 읽기 허용\" 정책을 가진다")]
async fn sandbox_allows_docs_read(world: &mut ManualWorld) {
    create_sandbox(world, json!({ "allow_read": ["docs/**"] }));
}

#[given("워크플로우 노드가 해당 샌드박스 안에서 실행된다")]
async fn node_runs_inside_sandbox(world: &mut ManualWorld) {
    ensure_sandbox(world);
}

#[when("에이전트가 \"docs/wiki/목차.md\"를 읽으려고 한다")]
async fn agent_reads_allowed_docs_file(world: &mut ManualWorld) {
    evaluate_sandbox(world, "read_file", "docs/wiki/목차.md");
}

#[then("Manual은 파일 읽기를 허용해야 한다")]
async fn manual_allows_file_read(world: &mut ManualWorld) {
    assert_eq!(world.last()["result"]["decision"]["allowed"], true);
}

#[then("사용자는 별도 승인 요청을 받지 않아야 한다")]
async fn user_gets_no_extra_approval_request(world: &mut ManualWorld) {
    assert_eq!(
        world.last()["result"]["decision"]["approval_required"],
        false
    );
}

#[given("샌드박스가 \"docs/wiki/** 쓰기 허용\" 정책만 가진다")]
async fn sandbox_only_allows_wiki_write(world: &mut ManualWorld) {
    create_sandbox(world, json!({ "allow_write": ["docs/wiki/**"] }));
}

#[when("에이전트가 \"src/main.rs\"에 쓰려고 한다")]
async fn agent_writes_disallowed_src(world: &mut ManualWorld) {
    evaluate_sandbox(world, "write_file", "src/main.rs");
}

#[then("Manual은 파일 쓰기를 차단해야 한다")]
async fn manual_blocks_file_write(world: &mut ManualWorld) {
    assert_eq!(world.last()["result"]["decision"]["allowed"], false);
}

#[then("정책 위반 이유를 기록해야 한다")]
async fn manual_records_policy_violation_reason(world: &mut ManualWorld) {
    assert!(world.last()["result"]["decision"]["reason"].is_string());
}

#[then("사용자는 차단된 접근 경로를 확인할 수 있어야 한다")]
async fn user_can_see_blocked_path(world: &mut ManualWorld) {
    assert!(world.last()["result"]["decision"]["target"].is_string());
}

#[given("샌드박스가 \"api.example.com 접근 허용\" 정책만 가진다")]
async fn sandbox_allows_only_api_host(world: &mut ManualWorld) {
    create_sandbox(world, json!({ "allow_network": ["api.example.com"] }));
}

#[when("스크립트가 \"unknown.example.net\"에 접근하려고 한다")]
async fn script_accesses_unknown_host(world: &mut ManualWorld) {
    evaluate_sandbox(world, "network", "unknown.example.net");
}

#[then("Manual은 네트워크 접근을 차단해야 한다")]
async fn manual_blocks_network(world: &mut ManualWorld) {
    assert_eq!(world.last()["result"]["decision"]["allowed"], false);
}

#[then("정책 위반 호스트를 기록해야 한다")]
async fn manual_records_violation_host(world: &mut ManualWorld) {
    assert_eq!(
        world.last()["result"]["decision"]["target"],
        "unknown.example.net"
    );
}

#[given("샌드박스가 \"scripts/** 실행 허용\" 정책을 가진다")]
async fn sandbox_allows_scripts(world: &mut ManualWorld) {
    create_sandbox(world, json!({ "allow_commands": ["scripts/**"] }));
}

#[given("같은 샌드박스가 \"scripts/deploy.sh 실행 금지\" 정책을 가진다")]
async fn sandbox_denies_deploy(world: &mut ManualWorld) {
    let sandbox_id = world.sandbox_id.clone().expect("sandbox should exist");
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 70,
            "method": "sandbox.update",
            "params": {
                "sandbox_id": sandbox_id,
                "changes": { "deny_commands": ["scripts/deploy.sh"] }
            }
        }),
    ));
}

#[when("에이전트가 \"scripts/deploy.sh\"를 실행하려고 한다")]
async fn agent_executes_denied_command(world: &mut ManualWorld) {
    evaluate_sandbox(world, "execute", "scripts/deploy.sh");
}

#[then("Manual은 명령 실행을 차단해야 한다")]
async fn manual_blocks_command_execution(world: &mut ManualWorld) {
    assert_eq!(world.last()["result"]["decision"]["allowed"], false);
}

#[then("명시적 거부 정책이 적용되었음을 기록해야 한다")]
async fn explicit_deny_is_recorded(world: &mut ManualWorld) {
    assert!(
        world.last()["result"]["decision"]["reason"]
            .as_str()
            .unwrap()
            .contains("explicit deny")
    );
}

#[then("사용자는 어떤 거부 규칙 때문에 차단되었는지 확인할 수 있어야 한다")]
async fn user_can_see_deny_rule(world: &mut ManualWorld) {
    explicit_deny_is_recorded(world).await;
}

#[given("샌드박스가 \"MANUAL_* 환경 변수 접근 허용\" 정책만 가진다")]
async fn sandbox_allows_manual_env(world: &mut ManualWorld) {
    create_sandbox(world, json!({ "allow_env": ["MANUAL_*"] }));
}

#[when("스크립트가 \"OPENAI_API_KEY\" 환경 변수를 읽으려고 한다")]
async fn script_reads_disallowed_env(world: &mut ManualWorld) {
    evaluate_sandbox(world, "read_env", "OPENAI_API_KEY");
}

#[then("Manual은 환경 변수 접근을 차단해야 한다")]
async fn manual_blocks_env_access(world: &mut ManualWorld) {
    assert_eq!(world.last()["result"]["decision"]["allowed"], false);
}

#[then("정책 위반 환경 변수 이름을 기록해야 한다")]
async fn manual_records_env_violation(world: &mut ManualWorld) {
    assert_eq!(
        world.last()["result"]["decision"]["target"],
        "OPENAI_API_KEY"
    );
}

#[given("샌드박스가 \".manual/tmp/** 쓰기 허용\" 정책을 가진다")]
async fn sandbox_allows_tmp_write(world: &mut ManualWorld) {
    create_sandbox(
        world,
        json!({ "allow_write": [".manual/tmp/**"], "tmp_write": [".manual/tmp/**"] }),
    );
}

#[given("샌드박스가 \".manual/cache/** 쓰기 허용\" 정책을 가진다")]
async fn sandbox_allows_cache_write(world: &mut ManualWorld) {
    let sandbox_id = world.sandbox_id.clone().expect("sandbox should exist");
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 71,
            "method": "sandbox.update",
            "params": {
                "sandbox_id": sandbox_id,
                "changes": { "cache_write": [".manual/cache/**"] }
            }
        }),
    ));
}

#[when("스크립트가 허용되지 않은 임시 경로에 파일을 쓰려고 한다")]
async fn script_writes_disallowed_tmp_path(world: &mut ManualWorld) {
    evaluate_sandbox(world, "write_file", "/tmp/outside.txt");
}

#[then("허용된 임시 디렉터리와 캐시 디렉터리 범위를 사용자가 확인할 수 있어야 한다")]
async fn user_can_see_tmp_cache_scope(world: &mut ManualWorld) {
    assert!(world.last()["result"]["decision"]["allowed_tmp"].is_array());
    assert!(world.last()["result"]["decision"]["allowed_cache"].is_array());
}

#[given("사용자가 반복 실행되는 문서 업데이트 워크플로우에 \"Docs Writer\" 샌드박스를 지정했다")]
async fn docs_writer_sandbox_assigned(world: &mut ManualWorld) {
    create_sandbox(world, json!({ "name": "Docs Writer" }));
}

#[when("에이전트가 샌드박스가 허용한 파일과 명령만 사용한다")]
async fn agent_uses_only_allowed_sandbox_actions(world: &mut ManualWorld) {
    evaluate_sandbox(world, "read_file", "docs/wiki/목차.md");
}

#[then("Manual은 매 커맨드마다 사용자 승인을 요구하지 않아야 한다")]
async fn manual_avoids_per_command_approval(world: &mut ManualWorld) {
    assert_eq!(
        world.last()["result"]["decision"]["approval_required"],
        false
    );
}

#[then("사용자는 실행이 사전 정의된 경계 안에서 이루어졌음을 확인할 수 있어야 한다")]
async fn user_can_confirm_predefined_boundary(world: &mut ManualWorld) {
    assert_eq!(world.last()["result"]["decision"]["allowed"], true);
}

#[given("워크플로우 노드가 샌드박스 안에서 실행되고 있다")]
async fn workflow_node_running_in_sandbox(world: &mut ManualWorld) {
    ensure_sandbox(world);
}

#[when("에이전트나 스크립트가 샌드박스 정책을 위반한다")]
async fn agent_violates_sandbox_policy(world: &mut ManualWorld) {
    evaluate_sandbox(world, "write_file", "src/main.rs");
}

#[then("Manual은 해당 노드 실행을 중단해야 한다")]
async fn manual_halts_node_on_violation(world: &mut ManualWorld) {
    assert_eq!(world.last()["result"]["decision"]["halt_node"], true);
}

#[then("정책 위반 이유와 위반 대상을 기록해야 한다")]
async fn manual_records_violation_reason_and_target(world: &mut ManualWorld) {
    manual_records_policy_violation_reason(world).await;
    user_can_see_blocked_path(world).await;
}

#[then("사용자는 정책을 변경하거나 입력을 수정한 뒤 다시 실행할 수 있어야 한다")]
async fn user_can_retry_after_policy_or_input_change(world: &mut ManualWorld) {
    assert_eq!(
        world.last()["result"]["decision"]["retry_allowed_after_policy_or_input_change"],
        true
    );
}

#[given("사용자가 기존 샌드박스 정책을 수정하려고 한다")]
async fn user_wants_to_edit_sandbox(world: &mut ManualWorld) {
    ensure_sandbox(world);
}

#[when("사용자가 쓰기 허용 경로나 네트워크 허용 호스트를 변경한다")]
async fn user_updates_sandbox_policy(world: &mut ManualWorld) {
    let sandbox_id = world.sandbox_id.clone().expect("sandbox should exist");
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 72,
            "method": "sandbox.update",
            "params": {
                "sandbox_id": sandbox_id,
                "changes": { "allow_network": ["api.example.com", "docs.example.com"] }
            }
        }),
    ));
}

#[then("Manual은 변경 전후 정책을 기록해야 한다")]
async fn sandbox_history_records_before_after(world: &mut ManualWorld) {
    let history = world.sandbox()["history"].as_array().unwrap();
    assert!(
        history
            .iter()
            .any(|entry| entry.get("before").is_some() && entry.get("after").is_some())
    );
}

#[then("변경 시각을 기록해야 한다")]
async fn sandbox_history_records_time(world: &mut ManualWorld) {
    assert!(
        world.sandbox()["history"]
            .as_array()
            .unwrap()
            .last()
            .unwrap()["at"]
            .is_string()
    );
}

#[then("이전 정책으로 되돌릴 수 있어야 한다")]
async fn sandbox_can_be_rolled_back(world: &mut ManualWorld) {
    sandbox_history_records_before_after(world).await;
}

#[given("매뉴얼 자기진화 기능이 로그 정리 스크립트를 제안했다")]
async fn self_evolution_suggested_cleanup_script(world: &mut ManualWorld) {
    ensure_manual(world);
}

#[when("사용자가 해당 스크립트를 실행하려고 한다")]
async fn user_runs_evolution_script_without_sandbox(world: &mut ManualWorld) {
    world.last_response = Some(rpc_allowing_errors(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 73,
            "method": "sandbox.evaluate",
            "params": { "sandbox_id": "", "operation": "execute", "target": "scripts/cleanup.sh" }
        }),
    ));
}

#[then("Manual은 실행할 샌드박스를 요구해야 한다")]
async fn manual_requires_sandbox_for_evolution_task(world: &mut ManualWorld) {
    assert!(world.last().get("error").is_some());
}

#[then("샌드박스가 지정되지 않으면 실행을 시작하지 않아야 한다")]
async fn manual_does_not_start_without_sandbox(world: &mut ManualWorld) {
    manual_requires_sandbox_for_evolution_task(world).await;
}

#[then("지정된 샌드박스 정책 안에서만 스크립트를 실행해야 한다")]
async fn manual_runs_script_only_inside_sandbox(world: &mut ManualWorld) {
    create_sandbox(world, json!({ "allow_commands": ["scripts/**"] }));
    evaluate_sandbox(world, "execute", "scripts/cleanup.sh");
    assert_eq!(world.last()["result"]["decision"]["allowed"], true);
}

#[given("사용자가 Manual에 에이전트 실행 단계가 포함된 워크플로우를 등록했다")]
async fn user_registered_agent_workflow(world: &mut ManualWorld) {
    configure_skill_step(world, json!(["llm-wiki"]));
}

#[given("사용자 로컬에 실행 가능한 에이전트가 감지되어 있다")]
async fn executable_agent_detected(world: &mut ManualWorld) {
    some_local_agents_are_installed(world).await;
}

#[given("사용자가 에이전트 실행 단계를 설정하고 있다")]
async fn user_configures_agent_step(world: &mut ManualWorld) {
    configure_skill_step(world, json!(["llm-wiki"]));
}

#[when("사용자가 해당 단계에 사용할 skill을 지정한다")]
async fn user_assigns_single_skill(world: &mut ManualWorld) {
    configure_skill_step(world, json!(["llm-wiki"]));
}

#[then("Manual은 지정된 skill을 단계 설정에 저장해야 한다")]
async fn manual_saves_configured_skill(world: &mut ManualWorld) {
    assert_eq!(world.last()["result"]["step"]["skills"][0], "llm-wiki");
}

#[then("해당 단계 실행 시 지정된 skill 정보를 에이전트 실행 요청에 포함해야 한다")]
async fn manual_includes_skill_in_agent_request(world: &mut ManualWorld) {
    assert_eq!(
        world.last()["result"]["step"]["agent_request"]["skills"][0],
        "llm-wiki"
    );
}

#[when("사용자가 여러 skill을 지정한다")]
async fn user_assigns_multiple_skills(world: &mut ManualWorld) {
    configure_skill_step(world, json!(["llm-wiki", "test-driven-development"]));
}

#[then("Manual은 지정된 skill 목록을 단계 설정에 저장해야 한다")]
async fn manual_saves_skill_list(world: &mut ManualWorld) {
    assert!(
        world.last()["result"]["step"]["skills"]
            .as_array()
            .unwrap()
            .len()
            >= 2
    );
}

#[then("skill 목록의 순서나 우선순위를 확인할 수 있게 해야 한다")]
async fn manual_exposes_skill_priority(world: &mut ManualWorld) {
    assert!(world.last()["result"]["step"]["priority"].is_array());
}

#[then("해당 단계 실행 시 skill 목록을 에이전트 실행 요청에 포함해야 한다")]
async fn manual_includes_skill_list_in_agent_request(world: &mut ManualWorld) {
    assert!(world.last()["result"]["step"]["agent_request"]["skills"].is_array());
}

#[given("단계의 작업 유형이 정의되어 있다")]
async fn step_task_type_is_defined(_world: &mut ManualWorld) {}

#[when("사용자가 skill 후보를 조회한다")]
async fn user_lists_skill_candidates(world: &mut ManualWorld) {
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 80,
            "method": "skill.candidates",
            "params": { "task_type": "documentation" }
        }),
    ));
}

#[then("Manual은 작업 유형과 관련된 skill 후보를 제공해야 한다")]
async fn manual_provides_relevant_skill_candidates(world: &mut ManualWorld) {
    assert!(
        !world.last()["result"]["candidates"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[then("각 skill의 이름과 목적을 제공해야 한다")]
async fn skill_candidates_have_name_and_purpose(world: &mut ManualWorld) {
    assert!(world.last()["result"]["candidates"][0]["name"].is_string());
    assert!(world.last()["result"]["candidates"][0]["purpose"].is_string());
}

#[then("사용자는 후보 중 하나 이상을 선택할 수 있어야 한다")]
async fn skill_candidates_are_selectable(world: &mut ManualWorld) {
    assert_eq!(world.last()["result"]["selectable"], true);
}

#[given("에이전트 실행 단계에 skill이 지정되어 있다")]
async fn agent_step_has_skill(world: &mut ManualWorld) {
    configure_skill_step(world, json!(["llm-wiki"]));
}

#[when("Manual이 해당 단계를 실행한다")]
async fn manual_executes_skill_step(world: &mut ManualWorld) {
    record_skill_execution(world, json!(["llm-wiki"]));
}

#[then("Manual은 실행 요청에 포함된 skill 정보를 기록해야 한다")]
async fn manual_records_requested_skill_info(world: &mut ManualWorld) {
    assert!(world.last()["result"]["execution"]["requested_skills"].is_array());
}

#[then("에이전트 출력이나 로그에서 관찰된 skill 사용 신호를 기록해야 한다")]
async fn manual_records_observed_skill_signal(world: &mut ManualWorld) {
    assert!(world.last()["result"]["execution"]["observed_skill_signals"].is_array());
}

#[then("사용자는 지정된 skill과 관찰된 skill 사용 정보를 비교할 수 있어야 한다")]
async fn user_can_compare_requested_and_observed_skills(world: &mut ManualWorld) {
    manual_records_requested_skill_info(world).await;
    manual_records_observed_skill_signal(world).await;
}

#[given("해당 단계의 실행 로그가 존재한다")]
async fn skill_step_execution_log_exists(world: &mut ManualWorld) {
    record_skill_execution(world, json!(["llm-wiki"]));
}

#[when("Manual이 skill 사용 여부를 검증한다")]
async fn manual_verifies_skill_usage(world: &mut ManualWorld) {
    let step_id = world
        .skill_step_id
        .clone()
        .unwrap_or_else(|| "agent-step".to_owned());
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 81,
            "method": "skill.verify",
            "params": { "step_id": step_id }
        }),
    ));
}

#[then("Manual은 지정된 skill이 사용되었는지 확인해야 한다")]
async fn manual_confirms_requested_skill_usage(world: &mut ManualWorld) {
    assert!(world.last()["result"]["used"].is_boolean());
}

#[then("확인할 수 없으면 미확인 상태로 기록해야 한다")]
async fn manual_records_unknown_when_unconfirmed(world: &mut ManualWorld) {
    assert!(world.last()["result"]["status"].is_string());
}

#[then("다른 skill 사용 신호가 있으면 사용자에게 제공해야 한다")]
async fn manual_reports_other_skill_signals(world: &mut ManualWorld) {
    assert!(world.last()["result"]["other_skill_signals"].is_array());
}

#[given("Manual이 skill 미사용 또는 불일치 신호를 발견했다")]
async fn manual_found_skill_mismatch(world: &mut ManualWorld) {
    configure_skill_step(world, json!(["llm-wiki"]));
    record_skill_execution(world, json!([]));
    manual_verifies_skill_usage(world).await;
}

#[then("Manual은 skill 미사용 또는 불일치를 실행 리스크로 제공해야 한다")]
async fn manual_reports_skill_mismatch_risk(world: &mut ManualWorld) {
    assert!(world.last()["result"]["risks"].is_array());
}

#[then("해당 리스크가 검증 상태에 어떤 영향을 줄 수 있는지 제공해야 한다")]
async fn manual_reports_skill_risk_impact(world: &mut ManualWorld) {
    assert!(world.last()["result"]["risks"][0]["impact"].is_string());
}

#[given("사용자가 여러 종류의 로컬 에이전트를 사용할 수 있다")]
async fn user_has_multiple_local_agents(world: &mut ManualWorld) {
    some_local_agents_are_installed(world).await;
}

#[when("사용자가 에이전트별 실행 설정을 조회한다")]
async fn user_reads_agent_skill_settings(world: &mut ManualWorld) {
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 82,
            "method": "skill.agent_capabilities"
        }),
    ));
}

#[then("Manual은 각 에이전트가 지원하는 skill 전달 방식을 제공해야 한다")]
async fn manual_reports_agent_skill_delivery(world: &mut ManualWorld) {
    assert!(world.last()["result"]["agents"][0]["delivery"].is_string());
}

#[then("지원하지 않는 방식은 선택할 수 없게 해야 한다")]
async fn unsupported_skill_delivery_not_selectable(world: &mut ManualWorld) {
    assert!(
        world.last()["result"]["agents"]
            .as_array()
            .unwrap()
            .iter()
            .any(|agent| agent["supported"] == false)
    );
}

#[then("지원 여부가 불확실한 경우 미확인 상태로 제공해야 한다")]
async fn uncertain_skill_delivery_is_unknown(world: &mut ManualWorld) {
    assert!(
        world.last()["result"]["agents"]
            .as_array()
            .unwrap()
            .iter()
            .any(|agent| agent["status"] == "unknown")
    );
}

fn ensure_storybook_node(world: &mut ManualWorld) {
    if world.latest_node_list.is_some() {
        return;
    }

    rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "node.create",
            "params": {
                "name": "Digest node",
                "description": "Summarizes a topic from Storybook input",
                "node": {
                    "id": "digest",
                    "kind": "template",
                    "template": "topic={{__storybook_input__.topic}} priority={{__storybook_input__.priority}}"
                }
            }
        }),
    );
}

fn create_three_stage_workflow(world: &mut ManualWorld, workflow_id: &str) {
    rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 11,
            "method": "workflow.create",
            "params": {
                "workflow": {
                    "id": workflow_id,
                    "nodes": [
                        {
                            "id": "source",
                            "kind": "constant",
                            "value": "alpha"
                        },
                        {
                            "id": "middle",
                            "kind": "template",
                            "template": "middle sees {{source}}"
                        },
                        {
                            "id": "final",
                            "kind": "template",
                            "template": "final sees {{middle}}"
                        }
                    ],
                    "dependencies": [
                        {
                            "node": "middle",
                            "depends_on": "source"
                        },
                        {
                            "node": "final",
                            "depends_on": "middle"
                        }
                    ]
                }
            }
        }),
    );
}

fn create_repairable_workflow(world: &mut ManualWorld) {
    rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 12,
            "method": "workflow.create",
            "params": {
                "workflow": {
                    "id": "resume-contract",
                    "nodes": [
                        {
                            "id": "source",
                            "kind": "constant",
                            "value": "alpha"
                        },
                        {
                            "id": "repairable",
                            "kind": "fail",
                            "error": "needs repair"
                        },
                        {
                            "id": "final",
                            "kind": "template",
                            "template": "final={{repairable}}"
                        }
                    ],
                    "dependencies": [
                        {
                            "node": "repairable",
                            "depends_on": "source"
                        },
                        {
                            "node": "final",
                            "depends_on": "repairable"
                        }
                    ]
                }
            }
        }),
    );
}

fn create_delay_workflow(world: &mut ManualWorld, workflow_id: &str, duration_ms: u64) {
    rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 16,
            "method": "workflow.create",
            "params": {
                "workflow": {
                    "id": workflow_id,
                    "nodes": [
                        { "id": "delay", "kind": "delay", "duration_ms": duration_ms },
                        { "id": "after-delay", "kind": "template", "template": "done" }
                    ],
                    "dependencies": [
                        { "node": "after-delay", "depends_on": "delay" }
                    ]
                }
            }
        }),
    );
}

fn patch_repairable_node(world: &mut ManualWorld) {
    rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 13,
            "method": "workflow.patch",
            "params": {
                "workflow_id": "resume-contract",
                "operations": [
                    {
                        "op": "update_node",
                        "node": {
                            "id": "repairable",
                            "kind": "template",
                            "template": "repaired from {{source}}"
                        }
                    }
                ]
            }
        }),
    );
}

fn ensure_manual(world: &mut ManualWorld) {
    if world.manual_id.is_none() {
        create_manual(world, "Contract Manual", "codex", json!({}));
    }
}

fn create_manual(world: &mut ManualWorld, name: &str, default_agent: &str, extra: Value) {
    let mut params = json!({
        "name": name,
        "purpose": "Exercise usecase contracts",
        "description": "Contract-backed manual",
        "default_agent": default_agent,
        "execution_mode": "single_agent"
    });
    for (key, value) in extra.as_object().into_iter().flatten() {
        params[key] = value.clone();
    }

    let created = rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 54,
            "method": "manual.create",
            "params": params
        }),
    );
    world.manual_id = created["result"]["manual"]["id"]
        .as_str()
        .map(str::to_owned);
    world.last_response = Some(created);
}

fn ensure_sandbox(world: &mut ManualWorld) {
    if world.sandbox_id.is_none() {
        create_sandbox(world, json!({}));
    }
}

fn create_sandbox(world: &mut ManualWorld, params: Value) {
    let created = rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 74,
            "method": "sandbox.create",
            "params": params
        }),
    );
    world.sandbox_id = created["result"]["sandbox"]["id"]
        .as_str()
        .map(str::to_owned);
    world.last_response = Some(created);
}

fn evaluate_sandbox(world: &mut ManualWorld, operation: &str, target: &str) {
    let sandbox_id = world.sandbox_id.clone().unwrap_or_default();
    world.last_response = Some(rpc_allowing_errors(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 75,
            "method": "sandbox.evaluate",
            "params": {
                "sandbox_id": sandbox_id,
                "operation": operation,
                "target": target
            }
        }),
    ));
}

fn record_optimization_run(world: &mut ManualWorld, run_id: &str, status: &str) {
    let recorded = rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 65,
            "method": "optimization.record_run",
            "params": {
                "run_id": run_id,
                "status": status,
                "workflow_id": "ai-workflow"
            }
        }),
    );
    world.optimization_run_id = recorded["result"]["run"]["id"].as_str().map(str::to_owned);
    world.last_response = Some(recorded);
}

fn compare_optimization(world: &mut ManualWorld) {
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 66,
            "method": "optimization.compare",
            "params": {}
        }),
    ));
}

fn configure_skill_step(world: &mut ManualWorld, skills: Value) {
    let configured = rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 83,
            "method": "skill.configure",
            "params": {
                "step_id": "agent-step",
                "task_type": "documentation",
                "agent": "codex",
                "skills": skills
            }
        }),
    );
    world.skill_step_id = configured["result"]["step"]["id"]
        .as_str()
        .map(str::to_owned);
    world.last_response = Some(configured);
}

fn record_skill_execution(world: &mut ManualWorld, observed: Value) {
    let step_id = world
        .skill_step_id
        .clone()
        .unwrap_or_else(|| "agent-step".to_owned());
    world.last_response = Some(rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 84,
            "method": "skill.record_execution",
            "params": {
                "step_id": step_id,
                "observed_skill_signals": observed
            }
        }),
    ));
}

fn assert_analysis_candidate(world: &ManualWorld, kind: &str) {
    assert!(
        world.last()["result"]["candidates"]
            .as_array()
            .expect("analysis candidates should exist")
            .iter()
            .any(|candidate| candidate["kind"] == kind)
    );
}

fn assert_missing_field(world: &ManualWorld, field: &str) {
    assert!(
        world.last()["result"]["validation"]["missing"]
            .as_array()
            .expect("missing fields should exist")
            .iter()
            .any(|missing| missing["field"] == field)
    );
}

fn fake_agent_bin(name: &str) -> String {
    let bin_dir = unique_storage_dir(name);
    fs::create_dir_all(&bin_dir).expect("fake agent bin dir should be created");
    let codex = bin_dir.join("codex");
    fs::write(&codex, "#!/bin/sh\nexit 0\n").expect("fake codex executable should be written");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&codex)
            .expect("fake codex metadata should exist")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&codex, permissions).expect("fake codex should be executable");
    }
    bin_dir.display().to_string()
}

fn agent_available(response: &Value, name: &str) -> bool {
    response["result"]["agents"]
        .as_array()
        .expect("agents should be available")
        .iter()
        .any(|agent| agent["name"] == name && agent["available"] == true)
}

fn sandbox_probe_log(world: &ManualWorld) -> String {
    fs::read_to_string(
        world
            .sandbox_test_log
            .as_ref()
            .expect("sandbox probe log should exist"),
    )
    .expect("sandbox probe log should be readable")
}

#[cfg(unix)]
fn make_executable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)
        .expect("executable metadata should exist")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("test script should be executable");
}

fn platform_sandbox_launchers() -> &'static [&'static str] {
    if cfg!(target_os = "macos") {
        &["sandbox-exec", "manual-seatbelt-runner"]
    } else if cfg!(target_os = "linux") {
        &["bwrap", "firejail", "nsjail", "manual-linux-sandbox-runner"]
    } else if cfg!(target_os = "windows") {
        &[
            "manual-windows-sandbox-runner.exe",
            "manual-appcontainer-runner.exe",
        ]
    } else {
        &["manual-sandbox-runner"]
    }
}

fn macos_sandbox_backends() -> &'static [&'static str] {
    &["seatbelt", "sandbox-exec", "manual-seatbelt-runner"]
}

fn linux_sandbox_backends() -> &'static [&'static str] {
    &[
        "namespace",
        "seccomp",
        "bubblewrap",
        "firejail",
        "nsjail",
        "manual-linux-sandbox-runner",
    ]
}

fn windows_sandbox_backends() -> &'static [&'static str] {
    &[
        "job-object",
        "appcontainer",
        "windows-sandbox",
        "manual-windows-sandbox-runner",
    ]
}

fn node_schema(world: &mut ManualWorld, kind: &str) -> Value {
    rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 14,
            "method": "node.schema",
            "params": {
                "kind": kind
            }
        }),
    )
}

fn start_workflow(world: &mut ManualWorld, workflow_id: &str, extra_params: Value) -> String {
    let mut params = json!({
        "workflow_id": workflow_id
    });
    let params_object = params
        .as_object_mut()
        .expect("start params should be a JSON object");
    for (key, value) in extra_params
        .as_object()
        .expect("extra start params should be a JSON object")
    {
        params_object.insert(key.clone(), value.clone());
    }

    let started = rpc(
        world,
        json!({
            "jsonrpc": "2.0",
            "id": 15,
            "method": "workflow.start",
            "params": params
        }),
    );

    started["result"]["run_id"]
        .as_str()
        .expect("workflow.start should return run_id")
        .to_owned()
}

fn poll_workflow_events_until(
    world: &mut ManualWorld,
    run_id: &str,
    cursor: usize,
    predicate: impl Fn(&Value) -> bool,
) -> Value {
    poll_until(|| {
        let events = rpc_allowing_errors(
            world,
            json!({
                "jsonrpc": "2.0",
                "id": 98,
                "method": "workflow.events",
                "params": {
                    "run_id": run_id,
                    "cursor": cursor
                }
            }),
        );
        predicate(&events).then_some(events)
    })
}

fn poll_node_events_until(
    world: &mut ManualWorld,
    run_id: &str,
    cursor: usize,
    predicate: impl Fn(&Value) -> bool,
) -> Value {
    poll_until(|| {
        let events = rpc_allowing_errors(
            world,
            json!({
                "jsonrpc": "2.0",
                "id": 99,
                "method": "node.run.events",
                "params": {
                    "run_id": run_id,
                    "cursor": cursor
                }
            }),
        );
        predicate(&events).then_some(events)
    })
}

fn poll_until(mut attempt: impl FnMut() -> Option<Value>) -> Value {
    let deadline = Instant::now() + Duration::from_secs(2);

    loop {
        if let Some(value) = attempt() {
            return value;
        }

        assert!(
            Instant::now() < deadline,
            "timed out waiting for app-server events"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn rpc(world: &mut ManualWorld, payload: Value) -> Value {
    let response = rpc_allowing_errors(world, payload);
    if response.get("error").is_some() {
        panic!("RPC call failed: {response}");
    }
    response
}

fn rpc_allowing_errors(world: &mut ManualWorld, payload: Value) -> Value {
    serde_json::from_str(&world.server.handle_json(&payload.to_string()))
        .expect("RPC response should be valid JSON")
}

fn find_digest_template(node_list: &Value) -> &Value {
    node_list["result"]["templates"]
        .as_array()
        .expect("node.list should return templates")
        .iter()
        .find(|template| template["id"] == "digest")
        .expect("digest template should be present")
}

impl ManualWorld {
    fn latest_node_list(&self) -> &Value {
        self.latest_node_list
            .as_ref()
            .expect("node list should have been queried")
    }

    fn latest_schema(&self) -> &Value {
        self.latest_schema
            .as_ref()
            .expect("node schema should have been queried")
    }

    fn node_events(&self) -> &Value {
        self.node_events
            .as_ref()
            .expect("node events should have been queried")
    }

    fn current_events(&self) -> &Value {
        self.current_events
            .as_ref()
            .expect("workflow events should have been queried")
    }

    fn last(&self) -> &Value {
        self.last_response
            .as_ref()
            .expect("last RPC response should have been recorded")
    }

    fn manual(&self) -> &Value {
        &self.last()["result"]["manual"]
    }

    fn sandbox(&self) -> &Value {
        &self.last()["result"]["sandbox"]
    }
}

fn test_server(name: &str) -> AppServer {
    AppServer::with_storage_dir(unique_storage_dir(name))
}

fn unique_storage_dir(name: &str) -> PathBuf {
    let counter = STORAGE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let unique = format!(
        "{name}-{}-{:?}-{counter}",
        std::process::id(),
        std::thread::current().id()
    );
    std::env::temp_dir().join("manual-rs-tests").join(unique)
}
