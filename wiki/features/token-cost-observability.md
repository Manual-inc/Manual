---
title: 토큰 비용 관측
type: feature
tags: [token, cost, observability, optimization, mvp]
sources: [wiki/sources/2026-05-11-manual-product-direction.md]
date_created: 2026-05-11
date_updated: 2026-05-11
---

# 토큰 비용 관측

출처: [2026-05-11 Manual 제품 방향 회의](../sources/2026-05-11-manual-product-direction.md)

## 개념

워크플로우 실행 run/job마다 토큰 사용량과 비용을 기록하고, 노드별 비용 구조를 볼 수 있게 하는 기능이다. Manual이 단순 자동화 도구가 아니라 에이전트 작업을 비용 관점에서도 개선하는 도구가 되게 한다.

## 필요한 지표

- run/job별 전체 토큰 사용량.
- 노드별 입력/출력 토큰 사용량.
- 에이전트/provider/model별 비용.
- 실패한 실행과 성공한 실행의 비용 비교.
- 워크플로우 수정 전후 비용 비교.

## 제품 가치

- 고급 모델에 모든 단계를 맡기는 경우와, 스크립트/로컬 모델/고급 모델을 섞는 경우를 비교할 수 있다.
- 비용이 큰 노드를 찾아 프롬프트, 입력 요약, 모델 배분을 개선할 수 있다.
- "이 워크플로우의 토큰 비용을 줄여줘" 같은 에이전트 기반 최적화 루프를 만들 수 있다.

## 비용 절감 전략

- 정적 분석이나 단순 의존성 추적은 스크립트로 처리한다.
- 간단한 요약/분류는 로컬 모델에 맡긴다.
- 높은 판단력이 필요한 단계만 Claude/Codex 같은 고급 에이전트에 맡긴다.
- 앞단 결과를 요약해 전달해 컨텍스트 크기를 줄인다.

## 관련 페이지

- [[decisions/2026-05-11-mvp-scope|2026-05-11 MVP 범위 결정]]
- [[features/partial-run-and-restart|부분 실행과 재시작]]
- [[architecture/manual-app-architecture|Manual 앱 아키텍처]]
