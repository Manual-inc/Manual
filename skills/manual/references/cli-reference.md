# manual CLI 전체 명령어 참조

소스: `app/cli/src/main.rs`

## 연결 설정

### 우선순위

CLI 시작 시 다음 순서로 서버 연결을 결정한다:

1. `--server-bin <PATH>` 지정 시 → 해당 바이너리를 stdio 모드로 직접 실행
2. `--server-url <URL>` + `--auth-token <TOKEN>` 지정 시 → HTTP 서버로 직접 연결
3. (기본) 데몬 모드 → discovery 파일로 기존 서버 재사용, 없으면 자동 시작

### 옵션 및 환경변수

| 옵션 | 환경변수 | 설명 |
|------|----------|------|
| `--server-bin <PATH>` | `MANUAL_APP_SERVER_BIN` | app-server 바이너리 경로 |
| `--server-url <URL>` | `MANUAL_APP_SERVER_URL` | HTTP 서버 URL (`http://127.0.0.1:PORT`만 허용) |
| `--auth-token <TOKEN>` | `MANUAL_APP_SERVER_TOKEN` | Bearer 인증 토큰 (URL과 함께 필수) |
| `--discovery-file <PATH>` | `MANUAL_APP_SERVER_DISCOVERY` | discovery JSON 파일 경로 |

기본 discovery 파일 위치 (macOS): `~/.manual/app-server.json`

`HOME`이 없으면: `$TMPDIR/manual-app-server.json`

### discovery 파일 형식

```json
{
  "url": "http://127.0.0.1:PORT",
  "auth_token": "TOKEN"
}
```

### 기본 서버 바이너리 탐색 경로

`--server-bin` / `MANUAL_APP_SERVER_BIN` 모두 없으면 다음 순서로 탐색:
- `./manual-rs/target/debug/manual-app-server`
- `../manual-rs/target/debug/manual-app-server`
- `../../manual-rs/target/debug/manual-app-server`

---

## `doctor`

로컬 연결 상태를 비파괴적으로 점검한다. 다음 항목을 함께 보여준다:

- app-server 바이너리 존재 여부와 경로
- discovery 파일 상태와 경로
- 현재 서버 URL / auth token 유무
- health 상태
- 바로 이어서 실행할 `Next steps`

```bash
manual doctor
```

대표적인 안내:

- 바이너리가 없으면 `cargo build --manifest-path manual-rs/Cargo.toml -p app-server --bin manual-app-server`
- discovery 파일이 깨졌거나 stale하면 discovery 경로를 안내하고 `manual demo optimization` 재실행을 권장
- 상태가 healthy이면 바로 `manual demo optimization`로 제품의 핵심 가치 경로를 이어 준다

---

## workflow 명령어

### `workflow starter code-review`

데모 다음에 가장 빨리 실제 저장소용 workflow를 만들기 위한 starter preset.

```bash
manual workflow starter code-review --run
manual workflow starter code-review --repo /path/to/repo --workflow-id repo-review
manual workflow starter code-review --agent codex
```

현재 동작:

- git repository 여부를 확인한다
- 로컬에서 사용 가능한 `codex`, `claude`, `pi` 중 하나를 자동 선택한다
- `collect_diff` script 노드와 `review` agent 노드가 있는 workflow를 생성한다
- `--run`이면 workflow 실행, optimization follow-through, review output까지 이어서 보여준다

옵션:

| 옵션 | 설명 |
|------|------|
| `--repo <PATH>` | starter workflow를 만들 대상 저장소 경로 (기본값: 현재 디렉터리) |
| `--workflow-id <ID>` | 저장할 workflow ID (기본값: `starter-code-review`) |
| `--agent <NAME>` | 사용할 agent를 직접 지정 (`codex`, `claude`, `pi`) |
| `--model <MODEL>` | review agent 노드에 넣을 model override |
| `--run` | 생성 후 즉시 workflow를 실행 |

이 명령은 raw app-server 메서드가 아니라 CLI-side preset이며, 내부적으로 `agent.list`, `workflow.create`, 필요 시 `workflow.start` + `workflow.events`를 사용한다.

---

### `workflow create <WORKFLOW_JSON>`

JSON 파일을 읽어 `workflow.create` RPC를 호출한다.

```bash
manual workflow create my-workflow.json
# → { "workflow_id": "...", "node_count": 2 }
```

워크플로우 ID는 JSON의 `id` 필드로부터 가져온다 (서버가 자동 생성하지 않는다).

### `workflow get <workflow_id>`

워크플로우 정의를 반환한다.

```bash
manual workflow get lead-review
# → { "workflow": { "id": "...", "nodes": [...], "dependencies": [...] } }
```

### `workflow list`

저장된 워크플로우 목록.

```bash
manual workflow list
# → { "workflows": [ { "workflow_id": "...", "node_count": N }, ... ] }
```

### `workflow update <workflow_id> <WORKFLOW_JSON>`

기존 워크플로우를 JSON 파일 내용으로 전체 교체한다 (부분 수정 아님).

```bash
manual workflow update lead-review updated.json
```

부분 수정이 필요하면 rpc로 `workflow.patch`를 호출한다 (아래 참조).

### `workflow delete <workflow_id>`

워크플로우 삭제.

```bash
manual workflow delete lead-review
# → { "workflow_id": "...", "deleted": true }
```

---

### `workflow start <workflow_id>`

워크플로우 실행을 백그라운드에서 시작하고 `run_id`를 즉시 반환한다. 완료를 기다리지 않는다.

| 옵션 | 설명 |
|------|------|
| `--start-node <NODE_ID>` | 이 노드부터 실행 시작 |
| `--resume-from-failure` | 이전 실행의 첫 번째 실패 노드부터 재시작 |
| `--inputs <PATH>` | 노드 입력 오버라이드 JSON 파일 (node_id → 값) |
| `--mode auto\|step` | 실행 방식 (기본: `auto`) |
| `--resume-run-id <RUN_ID>` | 이전 run ID (재시작용) |

```bash
manual workflow start lead-review
manual workflow start lead-review --start-node review --inputs overrides.json
manual workflow start lead-review --mode step
manual workflow start lead-review --resume-from-failure --resume-run-id run-prev
```

### `workflow run <workflow_id>`

`start`와 동일하게 실행을 시작하고, `events --watch`로 완료될 때까지 폴링한다.

`start`의 모든 옵션 + `--interval-ms <MS>` (기본 100).

```bash
manual workflow run lead-review
manual workflow run lead-review --mode step
manual workflow run lead-review --interval-ms 500
```

---

### `workflow events <run_id>`

실행 이벤트를 조회한다.

| 옵션 | 기본값 | 설명 |
|------|--------|------|
| `--cursor <N>` | 0 | N번 이벤트부터 조회 |
| `--watch` | false | `completed=true`가 될 때까지 반복 폴링 |
| `--interval-ms <MS>` | 100 | watch 폴링 간격 |

```bash
manual workflow events run-xyz
manual workflow events run-xyz --watch
manual workflow events run-xyz --cursor 5
```

응답 예시:
```json
{
  "events": [
    { "sequence": 0, "type": "workflow_started", "workflow_id": "...", "run_id": "..." },
    { "sequence": 1, "type": "node_started",   "node_id": "collect_diff", "run_id": "..." },
    { "sequence": 2, "type": "node_completed", "node_id": "collect_diff", "result": "...", "run_id": "..." }
  ],
  "next_cursor": 3,
  "completed": false,
  "run": {
    "status": "running",
    "paused": false,
    "first_failed_node": null,
    "resumable": false,
    "nodes": {
      "collect_diff": { "status": "completed", "result": "..." },
      "review":       { "status": "pending",   "result": null }
    }
  }
}
```

---

### `workflow stop <run_id>`

실행 중인 워크플로우를 취소한다. 현재 실행 중인 노드 이후에는 새 노드가 시작되지 않는다. 응답: `{ "cancelled": true }`.

```bash
manual workflow stop run-xyz
```

이후 이벤트 조회 시 `run.status: "cancelled"`로 표시된다.

---

### `workflow resume <run_id>`

paused/실패 상태의 실행을 재개한다.

| 옵션 | 설명 |
|------|------|
| `--start-node <NODE_ID>` | 이 노드부터 재개 |
| `--resume-from-failure` | 첫 번째 실패 노드부터 재개 |
| `--inputs <PATH>` | 노드 입력 오버라이드 JSON |
| `--mode auto\|step` | 실행 방식 |

> `resume`은 `--resume-run-id` 옵션을 받지 않는다 (run_id가 위치 인자로 이미 들어가므로).

```bash
manual workflow resume run-xyz
manual workflow resume run-xyz --resume-from-failure
manual workflow resume run-xyz --start-node review --inputs fix.json
```

---

## rpc 명령어

CLI가 래핑하지 않은 RPC 메서드를 직접 호출한다.

```bash
manual rpc <METHOD> [PARAMS_JSON]
```

- `PARAMS_JSON`이 없으면 params=null로 호출
- `PARAMS_JSON`이 `-`이면 stdin에서 JSON 읽음

```bash
manual rpc workflow.list
manual rpc workflow.get params.json
echo '{"workflow_id":"lead-review"}' | manual rpc workflow.get -
```

### CLI가 아직 래핑하지 않은 주요 RPC

- `workflow.patch` — 노드/의존성 부분 수정 (add_node, update_node, delete_node, add_dependency, update_dependency 등 op 지원)
  ```json
  {
    "workflow_id": "lead-review",
    "operations": [
      { "op": "update_node", "node": { "id": "review", "kind": "template", "template": "..." } },
      { "op": "add_dependency", "dependency": { "node": "publish", "depends_on": "review" } },
      { "op": "delete_node", "node_id": "old_node" }
    ]
  }
  ```

전체 메서드 목록은 `manual-rs/crates/app-server/src/lib.rs`의 `handle_json` 라우팅 참조.

---

## 자주 쓰는 패턴

### 워크플로우 작성 → 실행 → 확인

```bash
# 1. JSON 작성
cat > workflow.json <<'EOF'
{
  "id": "demo",
  "nodes": [
    { "id": "msg", "kind": "constant", "value": "hello" },
    { "id": "out", "kind": "template", "template": "say: {{msg}}" }
  ],
  "dependencies": [ { "node": "out", "depends_on": "msg" } ]
}
EOF

# 2. 생성
manual workflow create workflow.json

# 3. 실행 + Watch
manual workflow run demo
```

### 실패 디버그 → 재시작

```bash
# 1. 이전 실행에서 실패 확인
manual workflow events $OLD_RUN_ID
#  → run.first_failed_node = "review", run.resumable = true

# 2. 입력 오버라이드 작성
cat > fix.json <<'EOF'
{ "review": { "diff_summary": "수정된 요약" } }
EOF

# 3. 같은 run을 재개하거나
manual workflow resume $OLD_RUN_ID --resume-from-failure --inputs fix.json

# 또는 새 run으로 재시작
manual workflow start demo --resume-from-failure --resume-run-id $OLD_RUN_ID --inputs fix.json
```

### Step 모드 단계별 검토

```bash
# 백그라운드로 시작
manual workflow start demo --mode step
#  → 첫 노드 실행 후 paused

# 이벤트 확인
manual workflow events $RUN_ID
#  → run.paused = true

# 다음 노드 진행
manual workflow resume $RUN_ID
# 반복
```
