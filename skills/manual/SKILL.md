---
name: manual
description: >
  Expert guide for the Manual project — an AI agent workflow management system
  that solves token scarcity and reliability problems in real-world AI adoption.
  Use this skill whenever the user asks about Manual, manual-cli, workflows,
  workflow nodes, sandbox policies, partial execution, agent skill routing, or
  the Manual app architecture. Also trigger when the user wants to create, run,
  debug, stop, or resume a workflow; design a workflow JSON definition;
  understand how the CLI connects to the app server; or interpret execution
  events output. Even if the user just says "manual", "manual-cli", or
  "워크플로우" in this project context, use this skill proactively.
---

# Manual Skill

Manual은 AI 에이전트 워크플로우 관리 시스템으로, 두 가지 핵심 문제를 해결한다:
- **토큰 부족** — 제한된 토큰을 작업 중요도에 따라 적재적소에 배분
- **신뢰성 부족** — LLM의 확률적 오류를 시스템 검증과 실행 제약으로 보완

## 아키텍처 개요

```
CLI (manual-cli) ──JSON-RPC──▶ App Server ──▶ 에이전트 실행기 (Claude / Codex / Pi)
macOS App ────────JSON-RPC──▶ App Server ──▶ 스크립트 실행기 (sandboxed)
```

- **App Server** (`manual-rs/crates/app-server`) — 워크플로우·노드·실행 상태·로그를 관리하는 JSON-RPC 서버
- **CLI** (`app/cli/src/main.rs`) — `manual-cli` 바이너리
- **로컬-first** — 에이전트는 로컬 CLI 프로세스로 비대화형 호출됨

자세한 내용은 `references/cli-reference.md`, `references/concepts.md`를 참고한다.

---

## 현재 MVP 범위에 대한 안내

이 스킬은 두 가지 정보를 구분해 다룬다:

- **현재 구현됨** — `manual-rs` 코드와 CLI에 실제로 존재하는 기능
- **유스케이스 비전** — `docs/usecase/*.feature`에 정의된 향후 목표 (현재 미구현)

워크플로우 JSON 스키마와 CLI 명령은 모두 현재 구현 기준이다. `token_budget`, `verification`, `skills` 같은 필드는 유스케이스 문서에 등장하지만 현재 워크플로우 정의에는 아직 포함되어 있지 않다.

---

## CLI 빠른 참조

### 서버 연결 방식 (택일)

```bash
# 1. 데몬 자동 관리 (기본, 권장)
manual-cli workflow list

# 2. 서버 바이너리 직접 지정 (stdio 모드)
manual-cli --server-bin ./manual-rs/target/debug/app-server workflow list

# 3. 이미 실행 중인 서버에 HTTP 연결
manual-cli --server-url http://127.0.0.1:PORT --auth-token TOKEN workflow list
```

환경변수: `MANUAL_APP_SERVER_BIN`, `MANUAL_APP_SERVER_URL`, `MANUAL_APP_SERVER_TOKEN`, `MANUAL_APP_SERVER_DISCOVERY`

기본 discovery 파일: `~/Library/Application Support/Manual/app-server.json`

### 워크플로우 CRUD

```bash
manual-cli workflow create workflow.json     # JSON 파일로 워크플로우 생성
manual-cli workflow list                     # 저장된 워크플로우 목록
manual-cli workflow get <workflow_id>        # 단일 워크플로우 조회
manual-cli workflow update <workflow_id> updated.json  # 전체 교체
manual-cli workflow delete <workflow_id>     # 삭제
```

> 워크플로우 ID는 JSON의 `id` 필드로 사용자가 직접 지정한다. CLI/서버가 자동 생성하지 않는다.

### 실행 및 모니터링

```bash
# 실행 시작 (run_id 출력 후 즉시 종료, 백그라운드 실행)
manual-cli workflow start <workflow_id>

# 실행 시작 + 완료까지 이벤트 스트림 폴링
manual-cli workflow run <workflow_id>

# 단계별 수동 진행 (각 노드 완료 후 paused; resume으로 진행)
manual-cli workflow start <workflow_id> --mode step

# 특정 노드부터 실행
manual-cli workflow start <workflow_id> --start-node <node_id>

# 이전 실행의 첫 실패 노드부터 재시작
manual-cli workflow start <workflow_id> --resume-from-failure --resume-run-id <previous_run_id>

# 노드 입력 오버라이드
manual-cli workflow start <workflow_id> --inputs overrides.json

# 이벤트 조회
manual-cli workflow events <run_id>                  # 한 번 조회
manual-cli workflow events <run_id> --watch          # 완료까지 폴링
manual-cli workflow events <run_id> --cursor 5       # 5번부터 조회

# 실행 중단 / 재개
manual-cli workflow stop <run_id>
manual-cli workflow resume <run_id>
manual-cli workflow resume <run_id> --resume-from-failure
```

### 로우 RPC (CLI가 래핑하지 않는 메서드 호출용)

```bash
manual-cli rpc workflow.list
manual-cli rpc workflow.patch params.json   # 부분 수정 (CLI 미래핑)
echo '{"workflow_id":"x"}' | manual-cli rpc workflow.get -
```

---

## 워크플로우 JSON 구조 (실제 스키마)

**최상위:**
```json
{
  "id": "<사용자가 지정하는 워크플로우 ID>",
  "nodes": [ ... NodeDefinition ... ],
  "dependencies": [ { "node": "<from>", "depends_on": "<to>" }, ... ]
}
```

`dependencies`는 **노드 외부**의 별도 배열이다. 각 항목은 `node`가 `depends_on`을 입력으로 받음을 의미한다.

### 노드 종류 (NodeKind)

| kind | 용도 | 핵심 필드 |
|------|------|-----------|
| `constant` | 정적 값 출력 | `value: any` |
| `template` | 텍스트 템플릿 렌더링. `{{node_id}}`, `{{node_id.field}}` 참조 가능 | `template: string` |
| `delay` | 시간 지연 (테스트·디버그용) | `duration_ms: number` |
| `fail` | 강제 실패 (테스트용) | `error: string` |
| `claude` | Claude CLI 비대화형 호출 | `prompt`, `model?`, `cwd?`, `extra_args?` |
| `codex` | Codex CLI 호출 | `prompt`, `model?`, `cwd?`, `extra_args?` |
| `pi` | Pi 에이전트 호출 | `prompt`, `model?`, `cwd?`, `extra_args?` |
| `script` | 샌드박스 환경에서 스크립트 실행 | `script`, `sandbox_policy` (필수) |

### 노드 예시 (실제 동작하는 스키마)

```json
// 상수 값
{ "id": "lead_payload", "kind": "constant", "value": { "lead_count": 128 } }

// 템플릿 (상위 노드 결과 참조)
{ "id": "summary", "kind": "template",
  "template": "qualified: {{lead_payload.qualified_count}}" }

// Claude 에이전트
{ "id": "review", "kind": "claude",
  "prompt": "다음 diff를 리뷰하라:\n{{collect_diff}}",
  "model": "claude-sonnet-4-6",
  "cwd": "/path/to/repo" }

// 샌드박스 스크립트
{ "id": "collect_diff", "kind": "script",
  "script": "git diff HEAD~1 --name-only",
  "sandbox_policy": { "allow_paths": ["/repo"], "tmp_write": true } }

// 의도적 실패 (테스트용)
{ "id": "boom", "kind": "fail", "error": "intentional failure" }
```

### 완전한 예시: 코드 리뷰 워크플로우

```json
{
  "id": "code-review",
  "nodes": [
    {
      "id": "collect_diff",
      "kind": "script",
      "script": "git diff HEAD~1",
      "sandbox_policy": { "allow_paths": ["."], "deny_network": true }
    },
    {
      "id": "review",
      "kind": "claude",
      "prompt": "다음 변경사항을 리뷰하라:\n{{collect_diff}}",
      "model": "claude-sonnet-4-6"
    }
  ],
  "dependencies": [
    { "node": "review", "depends_on": "collect_diff" }
  ]
}
```

### 샌드박스 정책 (`sandbox_policy`)

`script` 노드 전용 필수 필드. 임의의 JSON 객체이며 `manual-sandbox` 크레이트가 해석한다. macOS에서는 Apple Seatbelt(`sandbox-exec`)를 백엔드로 사용한다.

`sandbox_policy`에 들어갈 수 있는 키 (구현 진행 중):
- `allow_paths` / `deny_paths` — 파일 접근 화이트/블랙리스트
- `allow_commands` / `deny_commands` — 명령 패턴
- `tmp_write` — `/tmp` 쓰기 허용 여부 (boolean)
- `cache_write` — 캐시 디렉토리 쓰기 허용 여부
- `deny_network` — 네트워크 차단

> 정책 키의 확정 목록은 `manual-rs/crates/manual-sandbox/src/lib.rs`를 참조한다.

---

## 이벤트 출력 형식

`workflow events`는 다음 형태의 JSON을 반환한다.

```json
{
  "events": [
    { "run_id": "...", "sequence": 0, "type": "workflow_started", "workflow_id": "..." },
    { "run_id": "...", "sequence": 1, "type": "node_started", "node_id": "collect_diff" },
    { "run_id": "...", "sequence": 2, "type": "node_completed", "node_id": "collect_diff", "result": "..." },
    { "run_id": "...", "sequence": 3, "type": "node_failed", "node_id": "review", "error": "..." },
    { "run_id": "...", "sequence": 4, "type": "workflow_failed", "workflow_id": "...", "error": "..." }
  ],
  "next_cursor": 5,
  "completed": true,
  "run": {
    "status": "failed",
    "first_failed_node": "review",
    "resumable": true,
    "paused": false,
    "nodes": {
      "collect_diff": { "status": "completed", "result": "..." },
      "review":       { "status": "failed",    "error":  "..." }
    }
  }
}
```

이벤트 타입: `workflow_started`, `node_started`, `node_completed`, `node_failed`, `workflow_completed`, `workflow_failed`.

`run.first_failed_node`와 `run.resumable`을 보면 실패 지점에서 재시작 가능 여부를 알 수 있다.

---

## 주요 워크플로우 패턴

### 1. 실패 지점에서 재시작

```bash
# 1. 이벤트로 실패 노드 확인
manual-cli workflow events <run_id>
#  → "first_failed_node": "review", "resumable": true

# 2. 새 실행으로 실패 지점부터 재시작
manual-cli workflow start <workflow_id> \
  --resume-from-failure \
  --resume-run-id <previous_run_id>

# 또는 같은 run을 그대로 재개
manual-cli workflow resume <run_id> --resume-from-failure
```

### 2. 단계별 수동 진행 (Step Mode)

```bash
# Step 모드로 시작 → 첫 노드 완료 후 paused
manual-cli workflow start <workflow_id> --mode step

# 이벤트로 상태 확인
manual-cli workflow events <run_id>
#  → "paused": true

# 다음 노드 승인 (한 단계씩)
manual-cli workflow resume <run_id>
```

### 3. 중간 노드부터 입력 오버라이드해 실행

```bash
# overrides.json
# { "review": { "diff_summary": "수동 작성 텍스트" } }

manual-cli workflow start <workflow_id> \
  --start-node review \
  --inputs overrides.json
```

### 4. Adaptive Compute 패턴

비용을 줄이는 가장 효과적인 방법은 결정적 처리를 `script` 노드로 분리해 에이전트에게 작은 컨텍스트만 넘기는 것이다.

```json
{
  "id": "adaptive-review",
  "nodes": [
    { "id": "preprocess", "kind": "script",
      "script": "git diff HEAD~1 | head -200",
      "sandbox_policy": { "allow_paths": ["."], "deny_network": true } },
    { "id": "review", "kind": "claude",
      "prompt": "다음 diff를 리뷰하라:\n{{preprocess}}",
      "model": "claude-haiku-4-5" }
  ],
  "dependencies": [
    { "node": "review", "depends_on": "preprocess" }
  ]
}
```

---

## 핵심 개념 빠른 참조

| 개념 | 요약 | 구현 상태 |
|------|------|-----------|
| **Workflow** | 사용자 지정 ID로 식별되는 노드 그래프 | ✅ 구현됨 |
| **Node** | `kind` 필드로 종류 구분, 단일 출력 | ✅ 구현됨 |
| **Dependency** | 노드 간 입력 의존 (별도 배열) | ✅ 구현됨 |
| **Template 참조** | `{{node_id}}` / `{{node_id.field}}` | ✅ 구현됨 |
| **Sandbox Policy** | `script` 노드 필수, 임의 JSON | ✅ 구현됨 (macOS: Seatbelt) |
| **Partial Execution** | `--start-node`로 부분 실행 | ✅ 구현됨 |
| **Resume from Failure** | `--resume-from-failure` | ✅ 구현됨 |
| **Step Mode** | `mode: "step"` + `resume` | ✅ 구현됨 |
| **Stop** | `workflow.stop` 으로 cancel | ✅ 구현됨 |
| **Token Budget / Verification** | 노드별 토큰 한도, 검증 정책 | ⏳ 유스케이스만 정의됨 |
| **Agent Skill Routing** | `skills` 필드로 에이전트 스킬 지정 | ⏳ 유스케이스만 정의됨 |
| **Self-Evolution** | 실행 데이터 기반 워크플로우 개선 | ⏳ 유스케이스만 정의됨 |

자세한 개념은 `references/concepts.md` 참고.

---

## 문서 위치

- 워크플로우 정의 코드: `manual-rs/crates/manual-worflow/src/lib.rs`
- 노드 종류 스키마: `manual-rs/crates/manual-node/src/lib.rs` (`node_schema()` 참조)
- JSON-RPC 테스트: `manual-rs/crates/app-server/tests/json_rpc_workflow.rs`
- 샌드박스: `manual-rs/crates/manual-sandbox/src/lib.rs`
- CLI 소스: `app/cli/src/main.rs`
- 아키텍처 위키: `docs/wiki/architecture/manual-app-architecture.md`
- 유스케이스 (미래 비전): `docs/usecase/*.feature`
