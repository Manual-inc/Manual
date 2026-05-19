---
title: Manual CLI app-server 명령 표면
type: architecture
tags: [architecture, cli, app-server, json-rpc]
sources: []
date_created: 2026-05-18
date_updated: 2026-05-19
---

# Manual CLI app-server 명령 표면

`app/cli/`의 `manual` 바이너리는 로컬 `manual-app-server` JSON-RPC API를 사람이 쓰기 쉬운 전용 서브커맨드로 노출한다. 목표는 제품 기능의 대부분을 raw `rpc` 문자열 호출 뒤에 숨기지 않고, CLI만으로도 워크플로우, 노드, 매뉴얼, 최적화, 샌드박스, 스킬 라우팅 기능을 전부 다룰 수 있게 만드는 것이다.

이 페이지는 [[manual-app-architecture|Manual 앱 아키텍처]]에서 설명한 “앱 서버를 중심으로 CLI와 네이티브 클라이언트가 같은 상태를 공유하는 구조”를 CLI 관점에서 구체화한다.

## 명령 그룹

- `manual workflow ...` — `workflow.*` 11개 메서드 전부를 다룬다. 생성/조회/수정/삭제/패치, 실행/중단/재개, 이벤트 조회, registry 기반 조합을 포함한다.
- `manual node ...` — `node.*` 11개 메서드 전부를 다룬다. 노드 템플릿 등록과 수정, schema 조회, 독립 실행, 실행 로그, 테스트 케이스 저장/검증을 포함한다. 이 표면은 [[node-storybook|노드 Storybook]] 기능과 직접 연결된다.
- `manual manual ...` — `manual.*` 9개 메서드 전부를 다룬다. Manual 레코드 생성, 목록 조회, 상태 전환, 복제, 버전 조회를 포함한다.
- `manual optimization ...` — `optimization.*` 4개 메서드 전부를 다룬다. 최적화 실행 기록, 분석, 비교, 리포트를 포함하며 [[token-cost-observability|토큰 비용 관측]]과 연결된다.
- `manual demo optimization` — built-in 데모 workflow를 생성, 실행, 최적화 리포트/분석 표시까지 한 번에 수행한다. 제품의 핵심 가치를 가장 짧게 체감하는 진입점이다.
- `manual sandbox ...` — `sandbox.*` 5개 메서드 전부를 다룬다. 샌드박스 생성/조회/수정, 정책 평가를 포함하며 [[agent-sandboxing|에이전트 샌드박스]]와 [[샌드박스-기능]]을 CLI에서 바로 다룰 수 있게 한다.
- `manual skill ...` — `skill.*` 5개 메서드 전부를 다룬다. 스킬 단계 구성, 후보 추천, 실행 기록, 사용 검증, 에이전트 capability 힌트를 포함하며 [[agent-skill-routing|에이전트 스킬 지정]]과 연결된다.
- `manual agent list ...` — `agent.list` 메서드를 노출한다.
- `manual rpc ...` — 미래 메서드 추가나 디버깅을 위한 raw JSON-RPC fallback이다. 전용 명령 표면이 기본이고 `rpc`는 보조 수단이다.

## 입력 규칙

- 식별자 중심 메서드는 `workflow_id`, `node_id`, `run_id`, `manual_id`, `sandbox_id`, `step_id`를 CLI 인자로 직접 받는다.
- 중첩 구조가 큰 payload는 JSON 파일 경로를 받는다. 예를 들어 워크플로우 정의, 노드 정의, 매뉴얼 생성 payload, 샌드박스 생성 payload, 스킬 단계 payload는 파일로 넣는다.
- 부분 업데이트는 `--changes`, `--execution`, `--inputs`, `--params` 같은 옵션으로 JSON 파일을 추가할 수 있다.
- `--params`는 app-server가 object payload를 기대하지만 CLI에서 모든 세부 필드를 별도 옵션으로 열지 않은 메서드에 대한 escape hatch다.
- `manual optimization analyze --human`, `compare --human`, `report --human`은 같은 JSON-RPC 결과를 사람이 읽기 쉬운 텍스트 요약으로 렌더링한다. 기본 출력은 계속 JSON이고, human 출력은 measurement provenance도 함께 보여준다.
- `manual workflow events --human`과 `manual workflow run --human`은 workflow 이벤트와 완료된 run의 optimization report, optimization analysis를 한 흐름의 텍스트 출력으로 렌더링한다.

## 검증 규칙

- `app/cli/tests/cli.rs`는 fake app-server를 띄우고 각 전용 커맨드가 기대한 JSON-RPC 메서드 이름과 파라미터를 보내는지 검증한다.
- 같은 테스트 파일은 `manual-rs/crates/app-server/src/lib.rs`의 현재 dispatch 목록을 읽어 전용 CLI가 따라가야 하는 메서드 집합이 정확히 46개인지 확인한다.
- 따라서 CLI 회귀 테스트가 녹색이면, 적어도 현재 app-server dispatch 표면과 CLI 전용 명령 표면 사이의 누락은 없는 상태여야 한다.

## 관련 페이지

- [[manual-app-architecture|Manual 앱 아키텍처]]
- [[node-storybook|노드 Storybook]]
- [[token-cost-observability|토큰 비용 관측]]
- [[agent-skill-routing|에이전트 스킬 지정]]
- [[agent-sandboxing|에이전트 샌드박스]]
