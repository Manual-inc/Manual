#!/usr/bin/env bash
# Why this exists: docs/wiki/analyses/2026-05-19-demo-flow.md defines the
# fastest way to feel Manual's workflow + optimization value end-to-end.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manual_bin="${repo_root}/app/cli/target/debug/manual"
app_server_bin="${repo_root}/manual-rs/target/debug/manual-app-server"

ensure_binaries() {
  cargo build --quiet --manifest-path "${repo_root}/app/cli/Cargo.toml" --bin manual
  cargo build --quiet --manifest-path "${repo_root}/manual-rs/Cargo.toml" -p app-server --bin manual-app-server
}

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "${tmp_dir}"
}
trap cleanup EXIT

ensure_binaries

export MANUAL_APP_SERVER_BIN="${app_server_bin}"
export MANUAL_APP_SERVER_DISCOVERY="${tmp_dir}/app-server.json"
export MANUAL_RS_WORKFLOW_DIR="${tmp_dir}/state"

"${manual_bin}" demo optimization
