---
title: 에이전트 스킬 지정
type: feature
tags: [agent, skill, routing, workflow]
sources: [2026-05-11-manual-product-direction.md]
date_created: 2026-05-11
date_updated: 2026-05-16
---

# 에이전트 스킬 지정

출처: [2026-05-11 Manual 제품 방향 회의](../sources/2026-05-11-manual-product-direction.md)

## 개념

에이전트 노드에 사용할 skill을 명시하고, 해당 에이전트가 의도한 방식으로 작업하는지 관찰하는 기능이다. Manual을 범용 워크플로우 도구가 아니라 에이전트 작업 운영 도구로 차별화하는 요소다.

## 예시

- 코드 리뷰 노드에 code review skill을 지정한다.
- 실행 계획 노드에 planning skill을 지정한다.
- 검증 노드에 verification skill을 지정한다.
- 하나의 에이전트 노드에 여러 skill을 지정한다.

## 기대 효과

- 에이전트가 어떤 작업 규칙을 따라야 하는지 워크플로우 안에 명시할 수 있다.
- 에이전트가 skill을 무시하거나 잘못된 skill을 사용하는지 관찰할 수 있다.
- 팀의 작업 루틴을 노드 설정으로 재사용할 수 있다.

## 열린 질문

- skill 지정이 프롬프트 수준인지, 실행기에서 강제되는 설정인지 정해야 한다.
- 실행 로그에서 skill 사용 여부를 어떻게 검증할지 정해야 한다.
- Claude, Codex, Pi 등 에이전트별 skill 전달 방식이 다를 수 있다.

## 관련 페이지

- [[node-storybook|노드 Storybook]]
- [[manual-app-architecture|Manual 앱 아키텍처]]
- [[2026-05-11-mvp-scope|2026-05-11 MVP 범위 결정]]
