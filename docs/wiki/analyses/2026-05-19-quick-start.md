---
title: 2026-05-19 Quick Start
type: analysis
tags: [quick-start, onboarding, cli, native-clients]
sources: []
date_created: 2026-05-19
date_updated: 2026-05-19
---

# 2026-05-19 Quick Start

이 문서는 Manual을 처음 접한 사람이 가장 빠르게 제품 가치를 체감하는 경로를 정리한다.

## 설치

최신 릴리스 CLI 설치:

```bash
curl -fsSL https://github.com/Manual-inc/Manual/releases/latest/download/install.sh | bash
```

특정 릴리스 설치:

```bash
TAG=v0.x.y
curl -fsSL "https://github.com/Manual-inc/Manual/releases/download/${TAG}/install.sh" | bash
```

## 가장 빠른 데모

처음 연결 상태를 먼저 확인하고 싶다면:

```bash
manual doctor
```

이 명령은 현재 상태뿐 아니라 다음 행동도 함께 알려준다. 건강한 상태라면 바로 `manual demo optimization`를 권장하고, 누락이나 stale discovery가 있으면 복구 힌트를 먼저 출력한다.

설치된 CLI가 있다면:

```bash
manual demo optimization
```

저장소 안에서는:

```bash
bash scripts/demo-optimization.sh
```

이 데모는 다음을 한 번에 보여준다.

1. workflow 실행
2. human-readable workflow events
3. optimization report
4. optimization analysis

## 데모 다음 첫 실제 워크플로우

데모 다음에는 JSON을 직접 쓰지 않고 starter preset으로 첫 workflow를 만드는 경로를 권장한다.

```bash
manual workflow starter
manual workflow starter --repo .
manual workflow starter --repo . --run
manual workflow starter --run
manual workflow starter code-review --run
manual workflow starter test-plan --run
```

첫 명령은 starter catalog를 보여준다. 두 번째 명령은 현재 저장소 변경 유형을 보고 맞는 starter를 추천한다. 세 번째 명령은 그 추천 starter를 바로 실행한다. 네 번째 명령은 마지막으로 기억된 저장소 기준으로 추천 starter를 다시 실행한다. catalog에는 recent starter history와 `manual workflow run <workflow_id> --human` rerun 힌트도 함께 보인다. 각 preset에는 언제 쓰기 좋은지와 실행 뒤 어떤 결과를 얻게 되는지도 같이 표시된다. 저장소가 해석되면 추천 이유, 기대 결과, 그리고 어떤 changed file을 보고 그 추천이 나왔는지도 즉시 보여 준다. recent starter 목록도 가능한 경우 왜 그 starter가 잘 맞았는지, 기대 결과, 마지막 결과 preview를 함께 보여 준다. 이후 일반 `workflow run` rerun으로 결과가 바뀌어도 shared recent history는 그 최신 output을 다시 반영하고, CLI rerun 출력도 `Starter Outcome` block을 다시 보여 줘 follow-through를 잃지 않게 한다. rerun 없이 저장된 summary만 다시 보고 싶으면 `manual workflow starter-outcome <workflow_id>`를 쓰면 되고, 최신 starter 결과 하나만 바로 보고 싶으면 `manual workflow starter-outcome --latest`를 쓰면 된다. 두 형태 모두 `--copy`를 붙이면 clipboard로 바로 보낼 수 있다. 실행이 끝나면 CLI는 `Starter Outcome` block을 추가로 보여 줘 workflow ID, 재실행 명령, 핵심 결과를 바로 공유하거나 복붙하기 쉽게 만든다. `code-review`는 correctness review를, `change-summary`는 사람이 읽을 변경 요약을, `test-plan`은 자동/수동 검증 계획을 만든다. 모두 로컬 git repository를 확인하고, 사용 가능한 local agent를 선택해 workflow를 생성한 뒤 실행까지 이어 줄 수 있다. review 입력은 changed file summary와 bounded patch preview로 제한해 첫 실행이 과도하게 무거워지지 않게 한다. 자세한 배경은 [[workflow-starters|워크플로우 스타터]]를 본다.

## 소스에서 빌드

CLI 빌드:

```bash
cargo build --manifest-path app/cli/Cargo.toml --bin manual
```

app-server 빌드:

```bash
cargo build --manifest-path manual-rs/Cargo.toml -p app-server --bin manual-app-server
```

빌드한 CLI로 데모 실행:

```bash
app/cli/target/debug/manual demo optimization
```

## 네이티브 표면

macOS 앱 실행:

```bash
bash app/mac/script/build_and_run.sh
```

앱 안에서는 sidebar quick-start card에서 starter catalog를 보고 저장소를 고른 뒤 starter workflow를 바로 생성하고 실행할 수 있다. `Recommended Starter…` 버튼은 docs-only/code-without-tests/other diff 규칙을 미리 설명해 왜 특정 starter가 선택될지 실행 전에 이해할 수 있게 한다. 각 starter 카드도 언제 쓰기 좋은지와 어떤 결과를 얻게 되는지 같이 보여 준다. 마지막으로 쓴 저장소는 기억되어 `Run Recommended Again`으로 다시 실행할 수 있고, `Last repository` 카드에서 지금 다시 누르면 어떤 starter가 추천되는지와 그 이유, 기대 결과, changed file 힌트도 먼저 볼 수 있다. recent starter 목록도 가능한 경우 이유, 기대 결과, 마지막 결과 preview를 같이 보여 주므로 exact preset rerun을 더 안심하고 고를 수 있고, 결과가 있으면 `Copy Summary`로 바로 공유할 수 있다. 실행이 시작되면 bottom panel이 `Output` 탭으로 열려 결과를 바로 확인할 수 있고, starter workflow일 때는 `Starter Outcome` summary와 `Copy Summary` action도 보여 준다.

Windows shell:

- 제품 shell은 `app/window/`에 있다.
- 현재 preview shell은 `manual doctor` -> `manual demo optimization` -> `manual workflow starter` -> review output 순서를 static UI narrative로 먼저 보여주고, `code-review`, `change-summary`, `test-plan` starter 옵션을 함께 노출한다.
- 이 macOS 환경에서는 `bash scripts/test-window-ui-smoke.sh`로 XAML 구조 회귀를 검증한다.
- 실제 WinUI runtime wiring은 Windows 환경에서 마무리해야 한다.

## 권장 확인 명령

```bash
bash scripts/test-demo-smoke.sh
bash scripts/test-window-ui-smoke.sh
cargo run --manifest-path docs/test/Cargo.toml
swift run --package-path app/mac manual-cucumber
```
