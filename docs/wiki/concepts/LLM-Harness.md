---
title: LLM Harness
type: concept
tags: [llm, harness, tools, environment, control]
sources: [문제-정의.md]
date_created: 2026-05-16
date_updated: 2026-05-16
---

# LLM Harness

LLM harness는 LLM을 실제 작업 환경에서 다루기 위한 실행 기반 전체를 뜻한다.

## Definition

LLM harness는 모델이 어떤 도구를 사용할 수 있는지, 어떤 환경에서 실행되는지, 어떤 권한과 제약을 갖는지, 컨텍스트가 어떻게 주입되는지, 실행이 어떻게 제어되는지를 포괄한다.

따라서 harness는 단순한 검증 절차나 작업 규칙이 아니다. 그것은 LLM을 둘러싼 실행 외피이며, [[매뉴얼-시스템]]이 실제 업무 문제를 해결하기 위해 활용할 수 있는 기반 계층이다.

## Examples

- 도구 호출 인터페이스
- 파일 시스템, 셸, 브라우저 같은 실행 환경
- 권한, 샌드박스, 승인 정책
- 컨텍스트 주입과 메모리 관리
- 모델 호출 방식과 토큰 예산 제어
- 실행 로그와 관찰 가능성

[[샌드박스-기능]]은 LLM harness가 제공할 수 있는 실행 제어의 구체적 예다. 파일 읽기/쓰기, 명령 실행, 네트워크 접근 범위를 정책으로 제한해 에이전트와 스크립트가 허용된 경계 안에서만 동작하게 한다.

## Relationship to Manual System

[[매뉴얼-시스템]]은 [[LLM-Harness]]보다 넓은 개념이다. Manual System은 토큰 효율성과 신뢰성을 목표로 하는 작업 체계이며, LLM harness는 그 시스템을 가능하게 하는 실행 기반 중 하나다.

## Boundary

LLM harness가 "LLM을 어떻게 다룰 것인가"에 가깝다면, [[매뉴얼-시스템]]은 "LLM을 사용해 어떻게 신뢰할 수 있는 업무 결과를 만들 것인가"에 가깝다.
