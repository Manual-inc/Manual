---
title: 워크플로우 스타터
type: feature
tags: [workflow, starter, onboarding, cli, mac, windows]
sources: []
date_created: 2026-05-19
date_updated: 2026-05-19
---

# 워크플로우 스타터

워크플로우 스타터는 [[2026-05-19-demo-flow|데모 경로]] 다음 단계에서 사용자가 JSON 스키마를 먼저 배우지 않고도 자신의 저장소에서 첫 Manual workflow를 만들 수 있게 하는 진입점이다.

현재 starter catalog에는 `code-review`, `change-summary`, `test-plan` preset이 있으며, CLI와 mac app quick-start action 모두 이 catalog를 기반으로 starter를 생성할 수 있다.
Windows preview shell도 같은 starter 경로를 static onboarding narrative로 반영한다.

## 목적

- 데모에서 느낀 제품 가치를 실제 저장소의 첫 성공으로 이어 준다.
- `workflow.create`용 JSON을 직접 쓰기 전에 runnable workflow shape를 제공한다.
- 로컬에서 사용 가능한 에이전트를 자동 선택해 setup friction을 줄인다.
- 생성된 workflow를 그대로 수정하거나 재사용할 수 있게 한다.

## 현재 동작

현재 starter path는 다음을 수행한다.

1. 대상 저장소가 git repository인지 확인한다.
2. `codex`, `claude`, `pi` 순서로 로컬 가용 에이전트를 탐색한다.
3. repo diff를 수집하는 `collect_diff` script 노드와 review agent 노드를 가진 workflow를 생성한다.
4. `collect_diff`는 changed file summary와 bounded patch preview를 만들어 큰 저장소에서도 첫 review 입력이 과도하게 커지지 않게 한다.
5. workflow ID, repo 경로, 선택된 agent, 다음 실행 명령을 사람이 읽기 쉬운 형태로 출력한다.
6. `--run`이 붙으면 starter workflow를 바로 실행하고 optimization follow-through 뒤에 review output까지 보여준다.

## 현재 preset

추천 명령:

```bash
manual workflow starter --repo . --run
manual workflow starter code-review --run
manual workflow starter change-summary --run
manual workflow starter test-plan --run
```

CLI에서는 `manual workflow starter`만 실행해 available preset catalog를 먼저 볼 수 있다.
저장소 안에서 `manual workflow starter --repo .`를 실행하면 changed file 유형을 바탕으로 지금 가장 맞는 starter를 같이 추천한다.
저장소 안에서 `manual workflow starter --repo . --run`를 실행하면 추천된 starter를 즉시 실행한다.

mac app에서는 sidebar quick-start card에 starter catalog가 보이고, `Recommended Starter…` 버튼으로 현재 저장소 변경 유형을 보고 맞는 starter를 바로 고를 수 있다. 개별 starter를 눌러 직접 실행할 수도 있다. 실행 중에는 bottom panel이 `Output` 탭으로 열려 결과가 completion 뒤 바로 보이게 한다.

### `code-review`

- `collect_diff` — 현재 working tree diff 또는 최근 commit diff를 수집하되, file summary와 잘린 patch preview만 넘긴다.
- `review` — 로컬에서 사용 가능한 agent가 diff를 검토한다.

### `change-summary`

- `collect_diff` — 현재 working tree diff 또는 최근 commit diff를 같은 bounded preview 방식으로 수집한다.
- `summary` — 로컬에서 사용 가능한 agent가 변경 내용을 사람이 읽는 짧은 업데이트로 요약한다.

### `test-plan`

- `collect_diff` — 현재 working tree diff 또는 최근 commit diff를 같은 bounded preview 방식으로 수집한다.
- `test_plan` — 로컬에서 사용 가능한 agent가 자동/수동 검증 항목을 함께 적은 짧은 test plan을 만든다.

## 기대 효과

- 사용자는 `manual demo optimization` 다음에 바로 자기 저장소에서 첫 workflow를 돌릴 수 있다.
- 생성된 workflow definition이 app-server에 저장되므로 이후에는 `manual workflow run <workflow_id> --human`로 반복 실행하면 된다.
- starter output이 다음 행동을 명확히 제시해 onboarding이 문서에만 의존하지 않게 된다.

## 관련 페이지

- [[2026-05-19-quick-start|2026-05-19 Quick Start]]
- [[2026-05-19-demo-flow|2026-05-19 Demo Flow]]
- [[manual-cli-command-surface|Manual CLI app-server 명령 표면]]
- [[node-storybook|노드 Storybook]]
- [[agent-skill-routing|에이전트 스킬 지정]]
