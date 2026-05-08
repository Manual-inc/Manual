// swift-tools-version: 6.2

import PackageDescription

let package = Package(
    name: "ManualMac",
    platforms: [
        .macOS(.v26)
    ],
    products: [
        .executable(name: "ManualMac", targets: ["ManualMacApp"])
    ],
    targets: [
        .executableTarget(name: "ManualMacApp")
    ]
)

