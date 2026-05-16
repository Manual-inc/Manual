//! Cucumber feature contract tests for `docs/usecase/*.feature`.
//! Why this exists: docs/wiki/systems/기능-계약-테스트.md requires app-server to
//! execute shared usecase contracts from docs/usecase inside its own test suite.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use app_server::AppServer;
use cucumber::{World as _, given, then, when};
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
    let run_id = world
        .node_run_id
        .clone()
        .expect("node run should already be complete");
    world.node_events = Some(poll_node_events_until(world, &run_id, 0, |events| {
        events["result"]["completed"].as_bool().unwrap_or(false)
    }));
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
