# Manual Window

Windows native preview shell for the Manual onboarding and optimization workflow.

Current scope:

- quick-start path (`manual doctor` -> `manual demo optimization` -> `manual workflow starter`)
- starter options preview (`code-review`, `change-summary`, `test-plan`)
- workflow run summary
- execution timeline
- review output preview
- optimization metrics and derived-measurement notice

Build this directory on Windows with the .NET SDK and Windows App SDK workload installed.

On macOS, `dotnet build` can restore the project but cannot execute the Windows XAML compiler. For this repository, use `bash scripts/test-window-ui-smoke.sh` to validate that the XAML shell remains well-formed and still exposes the expected quick-start + optimization-focused surface.
