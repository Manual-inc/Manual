import Foundation

// See docs/wiki/architecture/cli-release-distribution.md: mac-side launchers
// must follow the shared `manual-app-server` binary naming contract.
public func defaultManualAppServerBinaryPath(repositoryRootPath: String) -> String {
    URL(fileURLWithPath: repositoryRootPath, isDirectory: true)
        .appendingPathComponent("manual-rs/target/debug/manual-app-server")
        .path
}

public func defaultManualAppServerBinaryURL(repositoryRoot: URL) -> URL {
    URL(fileURLWithPath: defaultManualAppServerBinaryPath(repositoryRootPath: repositoryRoot.path))
}
