// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "ClaudeTray",
    platforms: [.macOS("14.4")],
    targets: [
        .executableTarget(
            name: "ClaudeTray",
            path: "Sources/ClaudeTray",
            swiftSettings: [.enableExperimentalFeature("StrictConcurrency")]
        )
    ]
)
