// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "ZagChat",
    platforms: [.macOS(.v13)],
    dependencies: [
        .package(name: "Zag", path: "../../bindings/swift"),
    ],
    targets: [
        .executableTarget(
            name: "ZagChat",
            dependencies: [.product(name: "Zag", package: "Zag")],
            path: "Sources/ZagChat"
        ),
    ]
)
