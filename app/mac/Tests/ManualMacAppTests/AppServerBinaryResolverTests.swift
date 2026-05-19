import Testing
@testable import ManualMacApp

@Suite("AppServer Binary Resolver")
struct AppServerBinaryResolverTests {
    @Test func debugBinaryPathUsesManualAppServerName() {
        let path = defaultManualAppServerBinaryPath(repositoryRootPath: "/tmp/manual-repo")

        #expect(path.hasSuffix("/manual-rs/target/debug/manual-app-server"))
    }
}
