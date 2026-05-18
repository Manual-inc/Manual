---
title: Manual 앱 아키텍처
type: architecture
tags: [architecture, app-server, cli, native-client, agent]
sources: [2026-05-11-manual-product-direction.md]
date_created: 2026-05-11
date_updated: 2026-05-18
---

# Manual 앱 아키텍처

출처: [2026-05-11 Manual 제품 방향 회의](../sources/2026-05-11-manual-product-direction.md)

## 개요

Manual은 프로젝트 앱 서버를 중심에 두고 CLI와 네이티브 클라이언트가 동일한 워크플로우와 실행 상태를 공유하는 구조로 논의됐다. 로컬-first MVP에서는 Codex CLI, Claude CLI 같은 로컬 에이전트 실행기를 비대화형으로 호출하고, 결과를 앱 서버를 통해 UI에 반영한다.

## 구성 요소

- 앱 서버: 워크플로우, 노드, 실행 상태, 로그를 관리한다.
- CLI: `workflow`, `node`, `manual`, `optimization`, `sandbox`, `skill`, `agent` 전용 서브커맨드로 app-server의 전체 JSON-RPC 표면을 보낸다. 자세한 명령 구조는 [[manual-cli-command-surface|Manual CLI app-server 명령 표면]]에 정리한다.
- macOS 앱: 워크플로우 그래프와 실행 로그를 보여준다.
- Windows/macOS/Linux 클라이언트: 장기적으로 각 플랫폼 네이티브 클라이언트가 가능하다.
- 에이전트 실행기: Codex, Claude, Pi, Hermes/Homecode 등 로컬 CLI 또는 실행 프로세스를 호출한다.
- 스크립트 실행기: clone, cd, 테스트 실행, 정적 분석 같은 deterministic 단계를 맡는다.

## 통신 방식

- 현재는 MCP 대신 JSON API를 사용한다.
- JSON API가 MVP에서는 더 단순하다.
- 필요하면 나중에 MCP로 바꿀 수 있다.

## 로컬 상태 저장

- CLI discovery 파일과 app-server 상태 저장 기본 경로는 `~/.manual/` 아래에 둔다.
- 기본 discovery 파일은 `~/.manual/app-server.json`을 사용한다.
- app-server는 `~/.manual/` 아래에서 워크플로우, 실행 기록, 노드 실행 기록 같은 파일 기반 상태를 공유한다.
- `MANUAL_APP_SERVER_DISCOVERY`와 `MANUAL_RS_WORKFLOW_DIR` 환경 변수로 기본 경로를 덮어쓸 수 있다.

## 실행 흐름 예시

1. CLI 또는 UI가 앱 서버에 워크플로우 실행을 요청한다.
2. 앱 서버가 스크립트 노드와 에이전트 노드를 순서 또는 병렬 조건에 따라 실행한다.
3. 에이전트 노드는 로컬 CLI를 비대화형으로 호출한다.
4. 실행 로그와 결과 스트림이 앱 서버에 쌓인다.
5. UI가 앱 서버에서 상태를 받아 그래프와 타임라인에 반영한다.

## 관련 페이지

- [[2026-05-11-manual-product-direction|2026-05-11 Manual 제품 방향 회의]]
- [[cli-release-distribution|Manual CLI 릴리스 배포]]
- [[manual-cli-command-surface|Manual CLI app-server 명령 표면]]
- [[partial-run-and-restart|부분 실행과 재시작]]
- [[node-storybook|노드 Storybook]]
- [[token-cost-observability|토큰 비용 관측]]
- [[agent-sandboxing|에이전트 샌드박스]]
- [[agent-skill-routing|에이전트 스킬 지정]]
