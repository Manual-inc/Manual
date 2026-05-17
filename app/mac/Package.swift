// swift-tools-version: 6.1

import PackageDescription

let testingFrameworksPath = "/Library/Developer/CommandLineTools/Library/Developer/Frameworks"

let package = Package(
    name: "ManualMac",
    platforms: [
        .macOS(.v15)
    ],
    products: [
        .executable(name: "ManualMac", targets: ["ManualMacRun"]),
        .executable(name: "manual-cucumber", targets: ["ManualCucumberRun"]),
        .library(name: "ManualCucumber", targets: ["ManualCucumber"]),
    ],
    targets: [
        .target(
            name: "ManualMacApp",
            resources: [
                .process("Resources")
            ]
        ),
        .executableTarget(
            name: "ManualMacRun",
            dependencies: ["ManualMacApp"],
            path: "Sources/ManualMacRun"
        ),
        .target(
            name: "ManualCucumber",
            dependencies: ["ManualMacApp"],
            path: "Sources/ManualCucumber"
        ),
        .executableTarget(
            name: "ManualCucumberRun",
            dependencies: ["ManualCucumber"],
            path: "Sources/ManualCucumberRun"
        ),
        .testTarget(
            name: "ManualMacAppTests",
            dependencies: ["ManualMacApp", "ManualCucumber"],
            path: "Tests/ManualMacAppTests",
            swiftSettings: [
                .unsafeFlags(["-F", testingFrameworksPath])
            ],
            linkerSettings: [
                .unsafeFlags([
                    "-F", testingFrameworksPath,
                    "-Xlinker", "-rpath",
                    "-Xlinker", testingFrameworksPath,
                    "-framework", "Testing",
                ])
            ]
        ),
    ]
)
