#!/usr/bin/env bash
# Why this exists: docs/wiki/architecture/manual-app-architecture.md treats
# Windows as a first-class native surface even when this environment can't run WinUI.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
window_xaml="${repo_root}/app/window/MainWindow.xaml"

xmllint --noout "${window_xaml}"
grep -q "Quick Start Path" "${window_xaml}"
grep -q "manual doctor" "${window_xaml}"
grep -q "manual workflow starter code-review --run" "${window_xaml}"
grep -q "Review Output" "${window_xaml}"
grep -q "Regression Risk" "${window_xaml}"
grep -q "Optimization" "${window_xaml}"
