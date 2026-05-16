//! 서버 재시작 후 부분 실행 상태 복구 시나리오 테스트
use app_server::AppServer;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static STORAGE_COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_storage_dir(name: &str) -> PathBuf {
    let counter = STORAGE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let unique = format!(
        "{name}-{}-{:?}-{counter}",
        std::process::id(),
        std::thread::current().id()
    );
    std::env::temp_dir()
        .join("manual-rs-tests")
        .join(unique)
}

fn poll_events_until(
    server: &AppServer,
    run_id: &str,
    cursor: usize,
    predicate: impl Fn(&Value) -> bool,
) -> Value {
    let deadline = Instant::now() + Duration::from_secs(2);

    loop {
        let events: Value = serde_json::from_str(
            &server.handle_json(
                &json!({
                    "jsonrpc": "2.0",
                    "id": 99,
                    "method": "workflow.events",
                    "params": {
                        "run_id": run_id,
                        "cursor": cursor
                    }
                })
                .to_string(),
            ),
        )
        .unwrap();

        if predicate(&events) {
            return events;
        }

        assert!(Instant::now() < deadline, "timed out waiting for events");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn failure_run_can_be_resumed_after_server_restart() {
    let storage = unique_storage_dir("restart-recovery");

    // ── 인스턴스 A: fail 워크플로우 실행 ──
    let run_id = {
        let server_a = AppServer::with_storage_dir(&storage);

        server_a.handle_json(
            &json!({
                "jsonrpc": "2.0", "id": 1,
                "method": "workflow.create",
                "params": {
                    "workflow": {
                        "id": "resume-wf",
                        "nodes": [
                            {"id": "A", "kind": "constant", "value": "ok"},
                            {"id": "B", "kind": "fail", "error": "intentional"},
                        ],
                        "dependencies": [{"node": "B", "depends_on": "A"}]
                    }
                }
            })
            .to_string(),
        );

        let started: Value = serde_json::from_str(
            &server_a.handle_json(
                &json!({
                    "jsonrpc": "2.0", "id": 2,
                    "method": "workflow.start",
                    "params": {"workflow_id": "resume-wf"}
                })
                .to_string(),
            ),
        )
        .unwrap();
        let run_id = started["result"]["run_id"].as_str().unwrap().to_owned();

        // 실행 완료 대기
        let result = poll_events_until(&server_a, &run_id, 0, |events| {
            events["result"]["completed"].as_bool().unwrap_or(false)
        });
        assert_eq!(result["result"]["run"]["status"], "failed", "A에서 실패 확인");

        run_id
    }; // server_a drop

    // ── 인스턴스 B: 같은 storage 디렉토리로 재시작 ──
    {
        let server_b = AppServer::with_storage_dir(&storage);

        // 이전 run의 이벤트가 복구되었는지 확인
        let result: Value = serde_json::from_str(
            &server_b.handle_json(
                &json!({
                    "jsonrpc": "2.0", "id": 3,
                    "method": "workflow.events",
                    "params": {"run_id": run_id, "cursor": 0}
                })
                .to_string(),
            ),
        )
        .unwrap();
        assert_eq!(result["result"]["run"]["status"], "failed", "B에서 실패 상태 복구 확인");
        assert_eq!(result["result"]["run"]["resumable"], true);
        assert_eq!(result["result"]["run"]["first_failed_node"], "B");

        // resume_from_failure=true 로 새 실행 (B를 제외한 A는 skip)
        // 주의: B는 여전히 fail이므로 새 실행도 실패하지만 A는 skip되어야 함
        let new_start: Value = serde_json::from_str(
            &server_b.handle_json(
                &json!({
                    "jsonrpc": "2.0", "id": 4,
                    "method": "workflow.start",
                    "params": {
                        "workflow_id": "resume-wf",
                        "resume_run_id": run_id,
                        "resume_from_failure": true,
                    }
                })
                .to_string(),
            ),
        )
        .unwrap();
        let new_run_id = new_start["result"]["run_id"].as_str().unwrap().to_owned();

        let new_result = poll_events_until(&server_b, &new_run_id, 0, |events| {
            events["result"]["completed"].as_bool().unwrap_or(false)
        });

        // A는 skip, B는 재실행 후 실패
        let events = new_result["result"]["events"].as_array().unwrap();
        let node_skipped: Vec<&str> = events
            .iter()
            .filter(|e| e["type"] == "node_skipped")
            .filter_map(|e| e["node_id"].as_str())
            .collect();
        assert!(
            node_skipped.contains(&"A"),
            "A는 skip 되어야 함: {node_skipped:?}"
        );
    }
}
