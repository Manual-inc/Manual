#!/usr/bin/env bash
# Why this exists: docs/wiki/architecture/cli-release-distribution.md defines the
# GitHub Release archive contract and the curl|bash installation flow for Manual.
set -euo pipefail

repo_slug="${MANUAL_INSTALL_REPO:-Manual-inc/Manual}"
version="${MANUAL_INSTALL_VERSION:-}"
bin_dir="${MANUAL_INSTALL_BIN_DIR:-${HOME}/.local/bin}"
base_url="${MANUAL_INSTALL_BASE_URL:-}"

usage() {
  cat <<'EOF'
Install Manual from GitHub Releases.

Usage:
  install.sh [--version <tag>] [--bin-dir <path>] [--base-url <url>] [--help]

Environment overrides:
  MANUAL_INSTALL_PLATFORM
  MANUAL_INSTALL_VERSION
  MANUAL_INSTALL_BIN_DIR
  MANUAL_INSTALL_BASE_URL
  MANUAL_INSTALL_REPO
EOF
}

log() {
  printf '%s\n' "$*"
}

fail() {
  printf 'manual install error: %s\n' "$*" >&2
  exit 1
}

download_file() {
  local url="$1"
  local output="$2"

  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$url" -o "$output"
    return
  fi

  if command -v wget >/dev/null 2>&1; then
    wget -qO "$output" "$url"
    return
  fi

  fail "curl or wget is required"
}

detect_platform() {
  local os arch

  if [[ -n "${MANUAL_INSTALL_PLATFORM:-}" ]]; then
    printf '%s\n' "${MANUAL_INSTALL_PLATFORM}"
    return
  fi

  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
    Linux) os="linux" ;;
    Darwin) os="darwin" ;;
    *)
      fail "unsupported operating system: ${os}"
      ;;
  esac

  case "$arch" in
    x86_64|amd64) arch="x86_64" ;;
    arm64|aarch64) arch="aarch64" ;;
    *)
      fail "unsupported architecture: ${arch}"
      ;;
  esac

  if [[ "$os" == "linux" && "$arch" == "aarch64" ]]; then
    fail "linux-aarch64 releases are not published yet"
  fi

  printf '%s-%s\n' "$os" "$arch"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)
      [[ $# -ge 2 ]] || fail "--version requires a value"
      version="$2"
      shift 2
      ;;
    --bin-dir)
      [[ $# -ge 2 ]] || fail "--bin-dir requires a value"
      bin_dir="$2"
      shift 2
      ;;
    --base-url)
      [[ $# -ge 2 ]] || fail "--base-url requires a value"
      base_url="$2"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      fail "unknown argument: $1"
      ;;
  esac
done

platform="$(detect_platform)"
archive_name="manual-${platform}.tar.gz"

if [[ -z "$base_url" ]]; then
  if [[ -n "$version" ]]; then
    base_url="https://github.com/${repo_slug}/releases/download/${version}"
  else
    base_url="https://github.com/${repo_slug}/releases/latest/download"
  fi
fi

work_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$work_dir"
}
trap cleanup EXIT

archive_path="${work_dir}/${archive_name}"
extract_dir="${work_dir}/extract"
mkdir -p "$extract_dir" "$bin_dir"

log "Downloading ${archive_name} from ${base_url}"
download_file "${base_url}/${archive_name}" "$archive_path"

tar -xzf "$archive_path" -C "$extract_dir"

[[ -f "${extract_dir}/manual" ]] || fail "archive did not contain manual"
[[ -f "${extract_dir}/manual-app-server" ]] || fail "archive did not contain manual-app-server"

install -m 0755 "${extract_dir}/manual" "${bin_dir}/manual"
install -m 0755 "${extract_dir}/manual-app-server" "${bin_dir}/manual-app-server"

log "Installed manual to ${bin_dir}/manual"
log "Installed manual-app-server to ${bin_dir}/manual-app-server"

case ":${PATH}:" in
  *":${bin_dir}:"*) ;;
  *)
    log "Add ${bin_dir} to PATH to run 'manual' directly."
    ;;
esac
