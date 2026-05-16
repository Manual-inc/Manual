---
title: 에이전트 샌드박스
type: architecture
tags: [sandbox, security, permissions, agent]
sources: [2026-05-11-manual-product-direction.md]
date_created: 2026-05-11
date_updated: 2026-05-16
---

# 에이전트 샌드박스

출처: [2026-05-11 Manual 제품 방향 회의](../sources/2026-05-11-manual-product-direction.md)

## 개념

에이전트나 스크립트 노드가 접근할 수 있는 파일, 디렉터리, 네트워크, host를 제한하는 실행 정책이다. 로컬 에이전트가 실제 저장소와 시스템에서 작업하기 때문에 Manual의 신뢰성과 안전성을 높이는 핵심 기능이다.

## 논의된 예시

- 특정 디렉터리만 보이게 한다.
- 특정 host만 조회 가능하게 한다.
- DNS 또는 네트워크 접근을 제한한다.
- 코드 리뷰 노드는 CVE 사이트처럼 필요한 외부 정보원만 접근하게 한다.

## 제품 내 위치

샌드박스는 중요하지만 MVP 발표의 첫 범위에서는 고급 옵션으로 둘 수 있다. 현재 우선순위는 [[partial-run-and-restart|부분 실행과 재시작]], [[node-storybook|노드 Storybook]], [[token-cost-observability|토큰 비용 관측]]이다.

## 열린 질문

- sandbox policy를 워크플로우 단위로 둘지 노드 단위로 둘지 정해야 한다.
- macOS, Windows, Linux의 정책 표현을 공통 schema로 추상화해야 한다.
- 실패 시 UI가 권한 오류와 일반 실행 오류를 어떻게 구분해 보여줄지 정해야 한다.

## 관련 페이지

- [[manual-app-architecture|Manual 앱 아키텍처]]
- [[2026-05-11-mvp-scope|2026-05-11 MVP 범위 결정]]
- [[agent-skill-routing|에이전트 스킬 지정]]
