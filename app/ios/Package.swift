// swift-tools-version: 6.2

import PackageDescription

let package = Package(
    name: "ManualIOS",
    platforms: [
        .iOS(.v26)
    ],
    products: [
        .library(name: "ManualIOS", targets: ["ManualIOSApp"])
    ],
    targets: [
        .target(name: "ManualIOSApp")
    ]
)

