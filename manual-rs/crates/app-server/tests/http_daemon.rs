use app_server::{AppServer, HttpServerConfig, serve_http_listener};
use serde_json::{Value, json};
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[test]
fn http_json_rpc_requires_local_auth_token_and_uses_single_server_state() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = AppServer::with_storage_dir(unique_storage_dir("http-rpc"));

    let handle = thread::spawn(move || {
        serve_http_listener(
            listener,
            server,
            HttpServerConfig {
                auth_token: "test-token".to_owned(),
            },
        )
        .unwrap();
    });

    let unauthorized = http_post_rpc(
        address,
        None,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "workflow.list",
            "params": {}
        }),
    );
    assert!(unauthorized.starts_with("HTTP/1.1 401 Unauthorized"));

    let create = http_post_rpc(
        address,
        Some("test-token"),
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "workflow.create",
            "params": {
                "workflow": {
                    "id": "daemon-shared",
                    "nodes": [
                        {
                            "id": "message",
                            "kind": "template",
                            "template": "hello"
                        }
                    ]
                }
            }
        }),
    );
    let create_body = response_json_body(&create);
    assert_eq!(create_body["result"]["workflow_id"], "daemon-shared");

    let list = http_post_rpc(
        address,
        Some("test-token"),
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "workflow.list",
            "params": {}
        }),
    );
    let list_body = response_json_body(&list);
    assert_eq!(
        list_body["result"]["workflows"],
        json!([
            {
                "workflow_id": "daemon-shared",
                "node_count": 1
            }
        ])
    );

    drop(handle);
}

#[test]
fn http_sse_streams_workflow_and_run_changes_from_shared_server() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = AppServer::with_storage_dir(unique_storage_dir("http-events"));

    thread::spawn(move || {
        serve_http_listener(
            listener,
            server,
            HttpServerConfig {
                auth_token: "event-token".to_owned(),
            },
        )
        .unwrap();
    });

    let mut events = TcpStream::connect(address).unwrap();
    events
        .write_all(
            b"GET /events?token=event-token HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        )
        .unwrap();

    let create = http_post_rpc(
        address,
        Some("event-token"),
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "workflow.create",
            "params": {
                "workflow": {
                    "id": "streamed",
                    "nodes": [
                        {
                            "id": "message",
                            "kind": "template",
                            "template": "hello"
                        }
                    ]
                }
            }
        }),
    );
    assert_eq!(
        response_json_body(&create)["result"]["workflow_id"],
        "streamed"
    );

    let start = http_post_rpc(
        address,
        Some("event-token"),
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "workflow.start",
            "params": {
                "workflow_id": "streamed"
            }
        }),
    );
    assert_eq!(response_json_body(&start)["result"]["run_id"], "run-1");

    events
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut buffer = String::new();
    events.read_to_string(&mut buffer).ok();

    assert!(buffer.starts_with("HTTP/1.1 200 OK"));
    assert!(buffer.contains("event: workflow_changed"));
    assert!(buffer.contains(r#""workflow_id":"streamed""#));
    assert!(buffer.contains("event: run_changed"));
    assert!(buffer.contains(r#""run_id":"run-1""#));
}

fn http_post_rpc(address: std::net::SocketAddr, token: Option<&str>, payload: Value) -> String {
    let body = payload.to_string();
    let auth = token
        .map(|token| format!("Authorization: Bearer {token}\r\n"))
        .unwrap_or_default();
    let request = format!(
        "POST /rpc HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{auth}\r\n{body}",
        body.len()
    );

    let mut stream = TcpStream::connect(address).unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    stream.shutdown(std::net::Shutdown::Write).unwrap();

    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
}

fn response_json_body(response: &str) -> Value {
    let (_, body) = response.split_once("\r\n\r\n").unwrap();
    serde_json::from_str(body).unwrap()
}

fn unique_storage_dir(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("{name}-{unique}"));
    fs::create_dir_all(&path).unwrap();
    path
}
