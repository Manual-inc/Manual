---
title: Manual Wiki Log
type: log
tags: [log, manual]
sources: []
date_created: 2026-05-11
date_updated: 2026-05-11
---

# Manual Wiki Log

## [2026-05-11] setup | Manual project wiki

- Summary: Manual 프로젝트의 제품/아키텍처/회의 결정사항을 관리하는 llm-wiki 구조를 생성했다.
- Pages created: `AGENTS.md`, `wiki/index.md`, `wiki/log.md`, `wiki/overview.md`
- Pages updated: none

## [2026-05-11] ingest | 2026-05-11 Manual 제품 방향 회의

- Summary: Manual 데모, 앱 서버 기반 아키텍처, MVP 우선순위, 부분 실행/노드 Storybook/토큰 비용 관측 논의를 인입했다.
- Pages created: `raw/meetings/2026-05-11-manual-product-direction.md`, `wiki/sources/2026-05-11-manual-product-direction.md`, `wiki/meetings/2026-05-11-manual-product-direction.md`, `wiki/decisions/2026-05-11-mvp-scope.md`, `wiki/features/partial-run-and-restart.md`, `wiki/features/node-storybook.md`, `wiki/features/token-cost-observability.md`, `wiki/features/agent-skill-routing.md`, `wiki/architecture/manual-app-architecture.md`, `wiki/architecture/agent-sandboxing.md`
- Pages updated: `wiki/index.md`, `wiki/overview.md`, `wiki/log.md`

## [2026-05-11] update | Adaptive Compute 개념 추가

- Summary: 워크플로우를 지속적으로 개선해 결정적 과제는 스크립트로, 낮은 위험의 비결정적 과제는 저비용 모델로, 핵심 판단 과제는 상위 LLM으로 라우팅하는 Adaptive Compute 개념을 정리했다.
- Pages created: `wiki/features/Adaptive-Compute.md`
- Pages updated: `wiki/index.md`, `wiki/overview.md`, `wiki/features/token-cost-observability.md`, `wiki/log.md`

## [2026-05-11] update | Adaptive Compute 로그 기반 노드 분리 보강

- Summary: 에이전트 노드의 실행 로그에서 반복적으로 관찰되는 결정적 작업 과정을 별도 스크립트/규칙 기반 노드로 분리하는 방법을 추가했다.
- Pages created: none
- Pages updated: `wiki/features/Adaptive-Compute.md`, `wiki/log.md`
