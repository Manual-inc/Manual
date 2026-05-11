---
title: Manual Wiki Overview
type: overview
tags: [overview, manual]
sources: [wiki/sources/2026-05-11-manual-product-direction.md]
date_created: 2026-05-11
date_updated: 2026-05-11
---

# Manual Wiki Overview

Manual은 "실행 가능한 매뉴얼"을 목표로 하는 로컬-first 에이전트 워크플로우 도구다. 스크립트 노드와 에이전트 노드를 조합해 코드 리뷰, 분해-실행-검증 루프, 테스트/분석 자동화 같은 복합 작업을 실행한다.

현재 방향은 앱 서버를 중심에 두고 CLI와 네이티브 클라이언트가 동일한 워크플로우와 실행 상태를 공유하는 구조다. 로컬 MVP에서는 Codex CLI, Claude CLI 같은 도구를 비대화형으로 호출하고, 결과 스트림과 로그를 UI에서 관찰한다. 자세한 구조는 [[architecture/manual-app-architecture|Manual 앱 아키텍처]]에 정리되어 있다.

2026-05-11 회의에서 단기 MVP는 [[features/partial-run-and-restart|부분 실행과 재시작]], [[features/node-storybook|노드 Storybook]], [[features/token-cost-observability|토큰 비용 관측]]으로 좁혀졌다. 이 세 가지는 Manual을 단순 그래프 뷰어가 아니라 사용자가 에이전트 자동화를 통제하고 개선하는 도구로 보여준다.

제품 원칙으로는 [[features/Adaptive-Compute|Adaptive Compute]]가 추가됐다. 이는 결정적으로 처리 가능한 과제는 스크립트로, 낮은 위험의 비결정적 과제는 로컬 LLM 또는 저비용 모델로, 핵심적이고 고심이 필요한 비결정적 과제는 상위 LLM으로 라우팅하면서 워크플로우 비용을 지속적으로 낮추는 방향이다.

후속 방향으로는 [[features/agent-skill-routing|에이전트 스킬 지정]]과 [[architecture/agent-sandboxing|에이전트 샌드박스]]가 있다. 스킬 지정은 에이전트가 어떤 작업 규칙을 따라야 하는지 워크플로우에 명시하는 기능이고, 샌드박스는 로컬 에이전트 실행의 파일/네트워크 접근을 제한하는 안전장치다.

## 현재 핵심 질문

- 노드 단위 실행과 재시작을 어떤 UX로 제공할 것인가?
- 노드 Storybook의 입력/출력 schema를 어디에 저장하고 어떻게 공유할 것인가?
- provider와 CLI별 토큰/비용 정보를 어떤 공통 형식으로 수집할 것인가?
- Adaptive Compute 라우팅 정책을 노드 metadata와 실행 기록에 어떻게 표현할 것인가?
- 그래프 편집 CRUD는 MVP 이후 어느 시점에 넣을 것인가?
