#!/usr/bin/env bash
# Why this exists: docs/wiki/analyses/2026-05-19-demo-flow.md defines the
# one-command product demo path that should prove Manual's core value quickly.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
demo_script="${repo_root}/scripts/demo-optimization.sh"
output_file="$(mktemp)"
cleanup() {
  rm -f "${output_file}"
}
trap cleanup EXIT

bash "${demo_script}" >"${output_file}"

grep -q "Workflow Events" "${output_file}"
grep -q "Optimization Report" "${output_file}"
grep -q "Optimization Analysis" "${output_file}"
grep -q "Digest step used most tokens" "${output_file}"
