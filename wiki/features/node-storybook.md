---
title: 노드 Storybook
type: feature
tags: [node, test-harness, ux, mvp]
sources: [wiki/sources/2026-05-11-manual-product-direction.md]
date_created: 2026-05-11
date_updated: 2026-05-11
---

# 노드 Storybook

출처: [2026-05-11 Manual 제품 방향 회의](../sources/2026-05-11-manual-product-direction.md)

## 개념

프론트엔드 Storybook처럼 노드 하나를 독립적으로 열고, 임의 입력을 넣어 실행 결과와 내부 과정을 확인하는 기능이다. 워크플로우 전체를 만들기 전에 노드 단위의 동작을 검증할 수 있게 한다.

## 필요한 동작

- 노드를 독립 화면에서 실행한다.
- 입력 파라미터를 임의로 구성한다.
- 실행 로그, 중간 결과, 최종 출력을 확인한다.
- 노드의 입력/출력 schema를 보여준다.
- 등록된 노드 목록을 관리한다.
- 워크플로우는 등록된 노드를 조합해 구성한다.

## 기대 효과

- 노드가 재사용 가능한 실행 단위라는 모델이 명확해진다.
- 부분 실행과 결합해 실패 지점 디버깅이 쉬워진다.
- 노드별 테스트 케이스를 축적할 수 있다.
- UI 편집 기능 전체를 만들기 전에도 제품 차별점을 보여줄 수 있다.

## 구현 사항

- 노드 registry를 파일 기반으로 둔다.
- app server 에서 노드 스토리북을 위한 스키마, 메서드를 정의하고 이를 공용으로 사용한다.
- 스크립트 노드와 에이전트 노드의 Storybook UI는 공통적인 부분은 동일하게 가져가되 차집합 부분은 개별로 UI를 가져간다.

## 관련 페이지

- [[features/partial-run-and-restart|부분 실행과 재시작]]
- [[architecture/manual-app-architecture|Manual 앱 아키텍처]]
- [[features/agent-skill-routing|에이전트 스킬 지정]]
