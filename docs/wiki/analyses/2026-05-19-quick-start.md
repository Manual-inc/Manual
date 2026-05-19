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
manual workflow starter code-review --run
```

이 명령은 로컬 git repository를 확인하고, 사용 가능한 local agent를 선택해 code review workflow를 생성한 뒤 즉시 실행까지 이어 준다. review 입력은 changed file summary와 bounded patch preview로 제한해 첫 실행이 과도하게 무거워지지 않게 한다. 자세한 배경은 [[workflow-starters|워크플로우 스타터]]를 본다.

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

Windows shell:

- 제품 shell은 `app/window/`에 있다.
- 이 macOS 환경에서는 `bash scripts/test-window-ui-smoke.sh`로 XAML 구조 회귀를 검증한다.
- 실제 WinUI runtime wiring은 Windows 환경에서 마무리해야 한다.

## 권장 확인 명령

```bash
bash scripts/test-demo-smoke.sh
bash scripts/test-window-ui-smoke.sh
cargo run --manifest-path docs/test/Cargo.toml
swift run --package-path app/mac manual-cucumber
```
