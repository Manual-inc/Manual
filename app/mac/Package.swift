// swift-tools-version: 6.1

import PackageDescription

let package = Package(
    name: "ManualMac",
    platforms: [
        .macOS(.v15)
    ],
    products: [
        .executable(name: "ManualMac", targets: ["ManualMacApp"])
    ],
    targets: [
        .executableTarget(
            name: "ManualMacApp",
            resources: [
                .process("Resources")
            ]
        )
    ]
)
