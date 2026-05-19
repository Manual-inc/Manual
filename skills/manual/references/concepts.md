# Manual 핵심 개념 참조

이 문서는 두 종류의 정보를 구분해 다룬다:
- ✅ **현재 구현됨** — `manual-rs` 코드와 CLI에 실제로 존재하는 기능
- ⏳ **유스케이스 비전** — `docs/usecase/*.feature` 및 위키에 정의되었으나 현재 미구현

## 목차
1. [Manual System (전체 비전)](#manual-system-전체-비전)
2. [Workflow & Node (구현됨)](#workflow--node-구현됨)
3. [Dependencies & Template 참조 (구현됨)](#dependencies--template-참조-구현됨)
4. [Node Kinds (구현됨)](#node-kinds-구현됨)
5. [Sandbox Policy (구현됨)](#sandbox-policy-구현됨)
6. [Partial Execution & Step Mode (구현됨)](#partial-execution--step-mode-구현됨)
7. [Token Budget & Verification (유스케이스)](#token-budget--verification-유스케이스)
8. [Agent Skill Routing (유스케이스)](#agent-skill-routing-유스케이스)
9. [Adaptive Compute & Self-Evolution (유스케이스)](#adaptive-compute--self-evolution-유스케이스)

---

## Manual System (전체 비전)

Manual System은 AI 에이전트를 실제 업무에 도입할 때 발생하는 두 가지 핵심 문제를 해결하기 위한 작업 체계다.

**토큰 부족** — 고성능 LLM은 비싸고 토큰이 제한적이다. 모든 단계에 최고 모델을 쓰거나 과도한 컨텍스트를 전달하면 비용이 폭발한다.

**신뢰성 부족** — LLM은 확률적으로 동작하므로 검증 체계 없이는 오류 추적이 어렵고 사람이 결과를 재검토해야 한다.

세 가지 축으로 해결을 시도한다:
- **실행 구조** — 단계를 나눠 각 노드의 책임을 명확히 한다 (Workflow & Node)
- **실행 안전성** — 샌드박스로 파일·명령·네트워크 접근 경계를 제한한다
- **검증 절차** — 산출물 기준을 강제해 오류를 조기에 발견한다 (유스케이스 단계)

---

## Workflow & Node (구현됨)

**Workflow**는 사용자가 지정한 ID로 식별되는 노드 그래프다.

`WorkflowDefinition` (`manual-rs/crates/manual-worflow/src/lib.rs`):
```rust
pub struct WorkflowDefinition {
    pub id: String,
    pub nodes: Vec<NodeDefinition>,
    pub dependencies: Vec<DependencyDefinition>,
}
```

**핵심 특징:**
- ID는 사용자가 직접 지정한다 (서버 자동 생성 아님)
- `workflow.create` 응답은 `{ workflow_id, node_count }`
- 동일 ID로 `create`를 다시 호출하면 거부됨
- `workflow.update`는 전체 교체, `workflow.patch`는 부분 수정

**Node**는 워크플로우의 단일 실행 단계다.

```rust
pub struct NodeDefinition {
    pub id: String,
    pub kind: NodeKind,        // 노드 종류
    pub value: Value,           // constant 전용
    pub template: String,        // template 전용
    pub duration_ms: u64,        // delay 전용
    pub error: String,           // fail 전용
    pub prompt: String,          // claude/codex/pi 전용
    pub model: Option<String>,
    pub cwd: Option<String>,
    pub extra_args: Vec<String>,
    pub script: String,          // script 전용
    pub sandbox_policy: Value,   // script 전용 (필수)
}
```

해당 `kind`에 사용되지 않는 필드는 빈 문자열/0/null로 직렬화된다.

---

## Dependencies & Template 참조 (구현됨)

### Dependencies

의존성은 **노드 외부**의 별도 배열이다.

```json
{
  "dependencies": [
    { "node": "review", "depends_on": "collect_diff" },
    { "node": "review", "depends_on": "context" }
  ]
}
```

같은 `node`에 여러 `depends_on`을 추가하면 모두 입력이 된다. 사이클은 허용되지 않는다.

### Template 참조 문법

`template` 노드와 `claude/codex/pi` 노드의 `prompt`에서 상위 노드 결과를 참조할 수 있다.

| 문법 | 동작 |
|------|------|
| `{{node_id}}` | 상위 노드 결과 전체를 문자열화해서 삽입 |
| `{{node_id.field}}` | 상위 노드 결과가 객체일 때 특정 필드 참조 |

예:
```json
{
  "id": "summary", "kind": "template",
  "template": "leads: {{payload.qualified}} / {{payload.total}}"
}
```

`payload` 노드의 결과가 `{ "qualified": 42, "total": 128 }`이면 → `"leads: 42 / 128"`.

스칼라 상위 결과도 `{{node_id}}`로 참조 가능 (예: `"next action: {{recommendation}}"`).

---

## Node Kinds (구현됨)

전체 노드 종류는 `manual-rs/crates/manual-node/src/lib.rs`의 `node_schema()` 함수에서 확인 가능.

### `constant`
정적 값을 그대로 출력. 워크플로우 입력 또는 테스트 데이터로 사용.
```json
{ "id": "lead_payload", "kind": "constant", "value": { "lead_count": 128 } }
```

### `template`
`{{...}}` 치환으로 텍스트 생성.
```json
{ "id": "summary", "kind": "template",
  "template": "leads: {{lead_payload.lead_count}}" }
```

### `delay`
지정 시간 대기 후 `null` 반환. 테스트·디버그용.
```json
{ "id": "pause", "kind": "delay", "duration_ms": 500 }
```

### `fail`
항상 실패. 오류 처리 시나리오 테스트용.
```json
{ "id": "boom", "kind": "fail", "error": "intentional" }
```

### `claude` / `codex` / `pi`
로컬 에이전트 CLI를 비대화형으로 호출. 출력은 `{ status_code, stdout, stderr }` 형태.
```json
{ "id": "review", "kind": "claude",
  "prompt": "리뷰: {{diff}}",
  "model": "claude-sonnet-4-6",
  "cwd": "/path/to/repo",
  "extra_args": ["--allowedTools", "Read,Bash"] }
```

### `script`
샌드박스 환경에서 셸 스크립트 실행. `sandbox_policy` 필수.
```json
{ "id": "collect", "kind": "script",
  "script": "git status --porcelain",
  "sandbox_policy": { "allow_paths": ["."], "deny_network": true } }
```

---

## Sandbox Policy (구현됨)

`script` 노드 실행 시 파일·명령·네트워크 접근 경계를 강제한다. `sandbox_policy`가 `null`이면 `script node requires sandbox_policy` 오류로 실행이 거부된다.

`sandbox_policy`는 임의 JSON 객체이며 `manual-sandbox` 크레이트가 해석한다.

### 백엔드

- **macOS**: `sandbox-exec` (Apple Seatbelt) — 기본 백엔드
- **Linux**: 미구현 (현재 fallback)
- **Windows**: 미구현 (job-object/appcontainer/windows-sandbox 검토 중)

### 정책 키 (구현 진행 중, 정확한 목록은 코드 참조)

- `allow_paths` / `deny_paths` — 파일 접근 화이트/블랙리스트
- `allow_commands` / `deny_commands` — 명령 패턴
- `tmp_write` — `/tmp` 쓰기 허용 (boolean)
- `cache_write` — 캐시 디렉토리 쓰기 허용
- `deny_network` — 네트워크 차단

정책 평가는 `evaluate(sandbox, operation, target)` 함수에서 처리. 명령 실행은 `run_sandboxed(sandbox, program, args)`로 호출된다.

### 왜 필수인가

`docs/wiki/problems/실행-안전성-부족.md`에서 정리한 문제처럼, 에이전트나 스크립트가 사전 합의 없이 임의의 파일·명령·네트워크에 접근하면 실행 안전성이 떨어진다. Manual은 이 경계를 명시적으로 선언하게 강제한다.

---

## Partial Execution & Step Mode (구현됨)

워크플로우 전체를 처음부터 다시 실행할 필요 없이 필요한 부분만 실행할 수 있다.

### 특정 노드부터 실행
```bash
manual workflow start <id> --start-node review
```
의존 노드는 이전 실행 결과나 `--inputs` 오버라이드로 충당.

### 실패 지점부터 재시작

이벤트 응답에 포함된 `run.first_failed_node`와 `run.resumable`이 신호다.

```bash
# 새 run으로 재시작
manual workflow start <id> --resume-from-failure --resume-run-id <previous_run>

# 같은 run을 그대로 재개
manual workflow resume <run_id> --resume-from-failure
```

### Auto vs Step 모드

- `--mode auto` (기본): 전체 자동 실행
- `--mode step`: 노드 하나 실행 후 `paused: true` 로 일시 정지. `workflow.resume` 호출 시 다음 노드 한 개만 진행.

step 모드 동작 흐름:
```
start (mode=step) → 1st 노드 완료 → paused
resume            → 2nd 노드 완료 → paused
resume            → 3rd 노드 완료 → completed
```

### 입력 오버라이드

```json
// overrides.json
{ "review": { "diff_summary": "..." } }
```
```bash
manual workflow start <id> --inputs overrides.json
```

해당 node_id가 실행될 때 평소의 상위 노드 결과 대신 오버라이드 값이 입력으로 사용된다.

---

## Token Budget & Verification (유스케이스)

> ⏳ 현재 워크플로우 정의에는 아직 포함되지 않음. `docs/usecase/매뉴얼-관리.feature`, `docs/wiki/systems/매뉴얼-최적화-기능.md`에 정의된 미래 비전.

### Token Budget (계획)
- 노드별 토큰 상한선
- 워크플로우 활성화 시 누락 검증
- 초과 시 경고 또는 중단

### Verification Policy (계획)
- 산출물 필수 항목 (`required_outputs`)
- 검증 기준 (정확도, 커버리지)
- 측정 지표 세 가지: 요구사항 충족도, 검증 통과율, 남은 리스크

### Optimization Report (계획)
```text
Token Usage      82,400 tokens
Verification     73% checked
Time             14m 20s

Main Issue
Implementation step used 61% of all tokens.

Recommendation
Preprocess file discovery with a script before calling the agent.
Pass only changed files and relevant summaries into implementation.
Expected token reduction: 25-35%.
```

---

## Agent Skill Routing (유스케이스)

> ⏳ `docs/usecase/에이전트-스킬-지정.feature` 참고. 현재 워크플로우 노드 스키마에는 `skills` 필드가 없다.

계획된 동작:
- 에이전트 노드에 `skills` 배열 지정
- 실행 요청에 스킬 정보 포함
- 실행 로그에서 스킬 사용 신호 관찰
- 미사용·불일치 시 실행 리스크로 표시

현재 우회 방법: `claude`/`codex` 노드의 `extra_args`에 직접 `--skill <name>` 같은 인자를 넣는다.

---

## Adaptive Compute & Self-Evolution (유스케이스)

### Adaptive Compute (제품 원칙)

작업 성격에 따라 실행 주체를 결정한다:
```
결정론적 반복 작업     →  script 노드 (토큰 0)
낮은 위험 비결정적 작업 →  저비용 모델 (haiku, local LLM)
핵심 추론·판단 작업   →  상위 LLM (opus, sonnet)
```

전처리 패턴(Preprocessing): 에이전트 호출 전 script로 컨텍스트를 압축한다. 이는 **현재 구현된 노드 종류만으로도 가능**하다.

```json
{
  "id": "preprocess", "kind": "script",
  "script": "git diff HEAD~1 | head -200",
  "sandbox_policy": { "allow_paths": ["."], "deny_network": true }
},
{
  "id": "review", "kind": "claude",
  "prompt": "다음 diff를 리뷰하라:\n{{preprocess}}",
  "model": "claude-haiku-4-5"
}
```

### Self-Evolution (유스케이스)

> ⏳ `docs/usecase/매뉴얼-자기진화-기능.feature` 참고.

계획된 동작:
1. 최적화 측정이 실행 데이터 수집 (토큰·시간·검증율)
2. Diagnose가 병목 식별
3. Recommend가 개선안 제안
4. 사용자 승인 또는 검증 가드레일 후 워크플로우에 반영

---

## 추가 참고 자료

- 워크플로우 코드: `manual-rs/crates/manual-worflow/src/lib.rs`
- 노드 스키마: `manual-rs/crates/manual-node/src/lib.rs`
- 샌드박스: `manual-rs/crates/manual-sandbox/src/lib.rs`
- JSON-RPC 동작 예시: `manual-rs/crates/app-server/tests/json_rpc_workflow.rs`
- CLI: `app/cli/src/main.rs`, `app/cli/tests/cli.rs`
- 유스케이스: `docs/usecase/*.feature`
- 위키 시스템 페이지: `docs/wiki/systems/*.md`
