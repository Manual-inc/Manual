#!/usr/bin/env bash
# Why this exists: docs/wiki/architecture/cli-release-distribution.md requires a
# release-tagged install.sh asset to default to its own release archive.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
work_dir="$(mktemp -d)"
release_dir="${work_dir}/release"
bin_dir="${work_dir}/bin"
tag="${1:-v-test}"
platform="${2:-darwin-aarch64}"

cleanup() {
  rm -rf "${work_dir}"
}
trap cleanup EXIT

mkdir -p "${release_dir}/package" "${bin_dir}"

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

rendered_install="${work_dir}/install.sh"
bash "${repo_root}/scripts/render-install.sh" "${tag}" "${rendered_install}"

MANUAL_INSTALL_BASE_URL="file://${release_dir}" \
MANUAL_INSTALL_PLATFORM="${platform}" \
MANUAL_INSTALL_BIN_DIR="${bin_dir}" \
  bash "${rendered_install}"

test -x "${bin_dir}/manual"
test -x "${bin_dir}/manual-app-server"
"${bin_dir}/manual" | grep -q "manual-smoke"
