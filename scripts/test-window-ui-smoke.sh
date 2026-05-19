#!/usr/bin/env bash
# Why this exists: docs/wiki/architecture/manual-app-architecture.md treats
# Windows as a first-class native surface even when this environment can't run WinUI.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
window_xaml="${repo_root}/app/window/MainWindow.xaml"

xmllint --noout "${window_xaml}"
grep -q "Optimization" "${window_xaml}"
grep -q "Regression Risk" "${window_xaml}"
grep -q "Recommendations" "${window_xaml}"
