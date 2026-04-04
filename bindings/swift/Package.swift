// swift-tools-version: 5.9

import PackageDescription

let package = Package(
    name: "Zag",
    platforms: [
        .macOS(.v13),
    ],
    products: [
        .library(name: "Zag", targets: ["Zag"]),
    ],
    targets: [
        .target(
            name: "Zag",
            path: "Sources/Zag"
        ),
        .testTarget(
            name: "ZagTests",
            dependencies: ["Zag"],
            path: "Tests/ZagTests"
        ),
    ]
)
