---
title: Manual CLI 릴리스 배포
type: architecture
tags: [architecture, cli, release, installer, github-release]
sources: []
date_created: 2026-05-18
date_updated: 2026-05-18
---

# Manual CLI 릴리스 배포

## 개요

Manual CLI는 GitHub Release 아카이브를 통해 배포되고, 사용자는 루트 `install.sh`를 `curl | bash` 형태로 실행해 설치할 수 있다. 설치 계약의 핵심은 CLI 바이너리 `manual`과 로컬 app-server 바이너리 `manual-app-server`를 항상 함께 배포하는 것이다.

## 배포 계약

- GitHub Release는 플랫폼별 `manual-<platform>.tar.gz` 아카이브를 제공한다.
- 각 아카이브는 `manual`과 `manual-app-server` 두 바이너리를 함께 포함해야 한다.
- `install.sh`는 사용자 플랫폼에 맞는 아카이브를 선택해 다운로드하고, 기본적으로 `~/.local/bin` 아래에 두 바이너리를 설치한다.
- 설치된 `manual`은 자신과 같은 디렉터리에 있는 `manual-app-server`를 우선 탐색한다.
- 사용자는 `MANUAL_INSTALL_PLATFORM`, `MANUAL_INSTALL_VERSION`, `MANUAL_INSTALL_BIN_DIR`, `MANUAL_INSTALL_BASE_URL`로 설치 동작을 덮어쓸 수 있다.
- 런타임에서는 `MANUAL_APP_SERVER_BIN`, `MANUAL_APP_SERVER_DISCOVERY`, `MANUAL_RS_WORKFLOW_DIR` 환경 변수로 서버 실행/상태 경로를 추가로 제어할 수 있다.

## 파이프라인 구조

- 브랜치 CI는 릴리스 아카이브를 패키징하고 `install.sh` smoke test를 실행해 설치 계약을 검증한다.
- 태그 기반 release workflow는 같은 패키징 스크립트로 플랫폼별 아카이브를 만들고 GitHub Release에 업로드한다.
- 태그 이름에 `-`가 포함되면 prerelease로 게시해 시험 릴리스를 만들 수 있다.

## 관련 페이지

- [[manual-app-architecture|Manual 앱 아키텍처]]
- [[문서-연결-코드]]
