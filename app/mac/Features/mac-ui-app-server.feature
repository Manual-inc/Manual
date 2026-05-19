# language: ko
# wiki: docs/wiki/architecture/manual-app-architecture.md
# wiki: docs/wiki/systems/기능-계약-테스트.md

기능: mac UI intent app-server 실행
  Manual macOS 앱 사용자로서
  나는 화면 메뉴에서 워크플로우 실행을 시작하고 싶다
  그래서 SwiftUI 앱이 app-server를 통해 실제 실행을 시작함을 확인할 수 있다

  시나리오: UI intent에서 예제 워크플로우를 실행한다
    조건 mac UI intent가 테스트 app-server discovery를 사용한다
    만일 사용자가 UI에서 예제 워크플로우 실행을 선택한다
    그러면 app-server에는 UI가 시작한 workflow run이 생성되어야 한다
    그리고 run 이벤트는 workflow_started를 포함해야 한다

  시나리오: UI intent 실행 후 optimization report를 확인한다
    조건 mac UI intent가 테스트 app-server discovery를 사용한다
    만일 사용자가 UI에서 예제 워크플로우 실행을 선택한다
    그러면 app-server에는 UI가 시작한 workflow run이 생성되어야 한다
    그리고 UI workflow 완료 후 optimization report가 준비되어야 한다
    그리고 optimization report는 derived 측정 근거를 포함해야 한다

  시나리오: UI intent에서 code review starter workflow를 실행한다
    조건 mac UI intent가 테스트 app-server discovery를 사용한다
    만일 사용자가 UI에서 code review starter 실행을 선택한다
    그러면 app-server에는 UI가 시작한 workflow run이 생성되어야 한다
    그리고 UI starter workflow는 code review 단계와 diff 수집 단계를 가져야 한다

  시나리오: UI intent에서 추천 starter workflow를 실행한다
    조건 mac UI intent가 테스트 app-server discovery를 사용한다
    만일 사용자가 UI에서 추천 starter 실행을 선택한다
    그러면 app-server에는 UI가 시작한 workflow run이 생성되어야 한다
    그리고 UI recommended starter workflow는 summary 단계와 diff 수집 단계를 가져야 한다

  시나리오: UI intent에서 recent starter rerun을 선택한다
    조건 mac UI intent가 테스트 app-server discovery를 사용한다
    그리고 사용자가 UI에서 code review starter 실행을 선택한다
    만일 사용자가 UI에서 recent starter rerun을 선택한다
    그러면 UI는 shared recent starter history를 조회해야 한다
    그리고 app-server에는 UI가 시작한 workflow run이 생성되어야 한다
    그리고 UI starter workflow는 code review 단계와 diff 수집 단계를 가져야 한다
