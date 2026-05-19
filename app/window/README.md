# Manual Window

Windows native shell for the Manual optimization workflow.

Current scope:

- workflow run summary
- execution timeline
- optimization metrics
- recommendations and derived-measurement notice

Build this directory on Windows with the .NET SDK and Windows App SDK workload installed.

On macOS, `dotnet build` can restore the project but cannot execute the Windows XAML compiler. For this repository, use `bash scripts/test-window-ui-smoke.sh` to validate that the XAML shell remains well-formed and still exposes the expected optimization-focused surface.
