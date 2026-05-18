#!/usr/bin/env bash
# Why this exists: docs/wiki/architecture/cli-release-distribution.md defines the
# release archive and installer contract that this smoke test protects.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
install_script="${repo_root}/install.sh"
work_dir="$(mktemp -d)"
release_dir="${work_dir}/release"
bin_dir="${work_dir}/bin"
default_platform() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "${os}" in
    Linux) os="linux" ;;
    Darwin) os="darwin" ;;
    *) os="$(printf '%s' "${os}" | tr '[:upper:]' '[:lower:]')" ;;
  esac

  case "${arch}" in
    x86_64|amd64) arch="x86_64" ;;
    arm64|aarch64) arch="aarch64" ;;
  esac

  printf '%s-%s\n' "${os}" "${arch}"
}

platform="${1:-$(default_platform)}"
archive_path="${2:-}"

cleanup() {
  rm -rf "${work_dir}"
}
trap cleanup EXIT

mkdir -p "${release_dir}/package" "${bin_dir}"

if [[ -n "${archive_path}" ]]; then
  cp "${archive_path}" "${release_dir}/manual-${platform}.tar.gz"
else
  cat > "${release_dir}/package/manual" <<'EOF'
#!/usr/bin/env bash
echo manual-smoke
EOF

  cat > "${release_dir}/package/manual-app-server" <<'EOF'
#!/usr/bin/env bash
echo manual-app-server-smoke
EOF

  chmod +x "${release_dir}/package/manual" "${release_dir}/package/manual-app-server"
  tar -C "${release_dir}/package" -czf "${release_dir}/manual-${platform}.tar.gz" manual manual-app-server
fi

MANUAL_INSTALL_BASE_URL="file://${release_dir}" \
MANUAL_INSTALL_BIN_DIR="${bin_dir}" \
MANUAL_INSTALL_VERSION="test" \
  bash "${install_script}"

test -x "${bin_dir}/manual"
test -x "${bin_dir}/manual-app-server"

if [[ -n "${archive_path}" ]]; then
  "${bin_dir}/manual" --help >/dev/null
else
  "${bin_dir}/manual" | grep -q "manual-smoke"
fi
