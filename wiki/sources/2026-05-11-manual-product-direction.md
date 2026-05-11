---
title: 2026-05-11 Manual 제품 방향 회의
type: source
tags: [meeting, manual, mvp, workflow, agent]
sources: [raw/meetings/2026-05-11-manual-product-direction.md]
date_created: 2026-05-11
date_updated: 2026-05-11
---

# 2026-05-11 Manual 제품 방향 회의

원자료: [Manual 제품 방향 회의 원자료](../../raw/meetings/2026-05-11-manual-product-direction.md)

## 요약

Manual은 "실행 가능한 매뉴얼"을 목표로 하는 로컬-first 에이전트 워크플로우 도구로 정리됐다. 핵심 모델은 스크립트 노드와 에이전트 노드를 조합해 코드 리뷰 같은 복합 작업을 실행하고, 앱 서버를 통해 CLI와 네이티브 UI가 동일한 워크플로우/실행 상태를 공유하는 구조다.

이번 회의의 주요 결론은 MVP를 그래프 CRUD 전체로 넓히기보다, [[features/partial-run-and-restart|부분 실행과 재시작]], [[features/node-storybook|노드 Storybook]], [[features/token-cost-observability|토큰 비용 관측]]에 집중하자는 것이다. [[architecture/agent-sandboxing|에이전트 샌드박스]]와 [[features/agent-skill-routing|에이전트 스킬 지정]]은 중요하지만 후속 또는 고급 옵션으로 분리할 수 있다.

## 핵심 기여

- Manual의 현재 데모 범위와 [[architecture/manual-app-architecture|Manual 앱 아키텍처]]를 명확히 정리했다.
- [[decisions/2026-05-11-mvp-scope|2026-05-11 MVP 범위 결정]]을 통해 단기 개발 우선순위를 좁혔다.
- 자동화와 사용자 통제를 함께 지원하는 제품 방향을 정리했다.
- 범용 워크플로우 도구가 아니라 로컬 에이전트 실행/스킬/샌드박스 중심 도구로 포지셔닝했다.
- 비용 최적화를 위해 스크립트, 로컬 모델, 고급 에이전트를 단계별로 조합하는 전략을 제시했다.

## 이 소스가 반영한 페이지

- [[meetings/2026-05-11-manual-product-direction|2026-05-11 Manual 제품 방향 회의]]
- [[decisions/2026-05-11-mvp-scope|2026-05-11 MVP 범위 결정]]
- [[architecture/manual-app-architecture|Manual 앱 아키텍처]]
- [[features/partial-run-and-restart|부분 실행과 재시작]]
- [[features/node-storybook|노드 Storybook]]
- [[features/token-cost-observability|토큰 비용 관측]]
- [[features/agent-skill-routing|에이전트 스킬 지정]]
- [[architecture/agent-sandboxing|에이전트 샌드박스]]
