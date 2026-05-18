#!/usr/bin/env bash
# Why this exists: docs/wiki/architecture/cli-release-distribution.md requires a
# release-specific install.sh asset whose default archive source matches its tag.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
template="${repo_root}/install.sh"
version="${1:-}"
output_path="${2:-}"

if [[ -z "${version}" ]]; then
  echo "usage: scripts/render-install.sh <tag> [output-path]" >&2
  exit 1
fi

escaped_version="$(printf '%s' "${version}" | sed 's/[\/&]/\\&/g')"

if [[ -n "${output_path}" ]]; then
  sed "s/^release_default_version=\"__MANUAL_RELEASE_VERSION__\"$/release_default_version=\"${escaped_version}\"/" "${template}" > "${output_path}"
  chmod +x "${output_path}"
else
  sed "s/^release_default_version=\"__MANUAL_RELEASE_VERSION__\"$/release_default_version=\"${escaped_version}\"/" "${template}"
fi
