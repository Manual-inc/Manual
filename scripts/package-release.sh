#!/usr/bin/env bash
# Why this exists: docs/wiki/architecture/cli-release-distribution.md defines the
# release artifact layout that pairs the Manual CLI with its colocated app-server.
set -euo pipefail

platform="${1:-}"
dist_dir="${2:-dist}"

usage() {
  cat <<'EOF'
Package a release archive for the current runner platform.

Usage:
  scripts/package-release.sh <platform> [dist-dir]

Supported platforms:
  linux-x86_64
  darwin-x86_64
  darwin-aarch64
EOF
}

fail() {
  printf 'release packaging error: %s\n' "$*" >&2
  exit 1
}

if [[ -z "${platform}" ]]; then
  usage
  fail "platform is required"
fi

case "${platform}" in
  linux-x86_64|darwin-x86_64|darwin-aarch64) ;;
  *)
    usage
    fail "unsupported platform: ${platform}"
    ;;
esac

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
mkdir -p "${repo_root}/${dist_dir}"

(
  cd "${repo_root}"
  cargo build --manifest-path app/cli/Cargo.toml --release --bin manual
  cargo build --manifest-path manual-rs/Cargo.toml -p app-server --release --bin manual-app-server
)

cli_bin="${repo_root}/app/cli/target/release/manual"
server_bin="${repo_root}/manual-rs/target/release/manual-app-server"
archive_path="${repo_root}/${dist_dir}/manual-${platform}.tar.gz"
stage_dir="$(mktemp -d)"

cleanup() {
  rm -rf "${stage_dir}"
}
trap cleanup EXIT

[[ -x "${cli_bin}" ]] || fail "missing built CLI binary at ${cli_bin}"
[[ -x "${server_bin}" ]] || fail "missing built app-server binary at ${server_bin}"

cp "${cli_bin}" "${stage_dir}/manual"
cp "${server_bin}" "${stage_dir}/manual-app-server"
tar -C "${stage_dir}" -czf "${archive_path}" manual manual-app-server

printf 'packaged %s\n' "${archive_path}"
