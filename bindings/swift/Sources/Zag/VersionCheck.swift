import Foundation

/// CLI version detection and compatibility checking for zag bindings.
public enum VersionCheck {

    /// A parsed semver tuple.
    public struct SemVer: Comparable {
        public let major: Int
        public let minor: Int
        public let patch: Int

        public static func < (lhs: SemVer, rhs: SemVer) -> Bool {
            if lhs.major != rhs.major { return lhs.major < rhs.major }
            if lhs.minor != rhs.minor { return lhs.minor < rhs.minor }
            return lhs.patch < rhs.patch
        }
    }

    /// A feature requirement with method name, minimum version, and whether it is set.
    public struct Requirement {
        public let method: String
        public let version: String
        public let isSet: Bool

        public init(method: String, version: String, isSet: Bool) {
            self.method = method
            self.version = version
            self.isSet = isSet
        }
    }

    /// Cached detected versions keyed by binary path.
    private static var versionCache: [String: String] = [:]
    private static let cacheLock = NSLock()

    /// Parse a semver string like "0.6.0" into a numeric tuple.
    public static func parseSemver(_ version: String) throws -> SemVer {
        let trimmed = version.trimmingCharacters(in: .whitespaces)
        let parts = trimmed.split(separator: ".").map(String.init)
        guard parts.count == 3,
              let major = Int(parts[0]),
              let minor = Int(parts[1]),
              let patch = Int(parts[2]) else {
            throw ZagError(
                message: "Could not parse version \"\(version)\": expected format \"X.Y.Z\"",
                exitCode: nil,
                stderr: "")
        }
        return SemVer(major: major, minor: minor, patch: patch)
    }

    #if os(macOS) || os(Linux)
    /// Detect the CLI version by running `{bin} --version`. Cached per binary path.
    public static func detectVersion(bin: String) async throws -> String {
        cacheLock.lock()
        if let cached = versionCache[bin] {
            cacheLock.unlock()
            return cached
        }
        cacheLock.unlock()

        let process = Process()
        let pipe = Pipe()

        if bin.contains("/") {
            process.executableURL = URL(fileURLWithPath: bin)
        } else {
            process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
            process.arguments = [bin, "--version"]
        }
        if process.arguments == nil {
            process.arguments = ["--version"]
        }
        process.standardOutput = pipe
        process.standardError = FileHandle.nullDevice

        do {
            try process.run()
        } catch {
            throw ZagError(
                message: "Could not detect zag CLI version: failed to run '\(bin) --version'. " +
                    "Ensure zag is installed and on your PATH, or set ZAG_BIN. (\(error.localizedDescription))",
                exitCode: nil,
                stderr: "")
        }

        process.waitUntilExit()

        guard process.terminationStatus == 0 else {
            throw ZagError(
                message: "Could not detect zag CLI version: '\(bin) --version' exited with code \(process.terminationStatus)",
                exitCode: Int(process.terminationStatus),
                stderr: "")
        }

        let data = pipe.fileHandleForReading.readDataToEndOfFile()
        let output = (String(data: data, encoding: .utf8) ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
        let parts = output.split(separator: " ")
        let versionStr = parts.last.map(String.init) ?? ""

        // Validate it parses
        _ = try parseSemver(versionStr)

        cacheLock.lock()
        versionCache[bin] = versionStr
        cacheLock.unlock()

        return versionStr
    }

    /// Check that the installed CLI version satisfies all configured requirements.
    public static func check(bin: String, requirements: [Requirement]) async throws {
        let active = requirements.filter(\.isSet)
        guard !active.isEmpty else { return }

        let detected = try await detectVersion(bin: bin)
        let detectedSv = try parseSemver(detected)

        let failures = try active.filter { try detectedSv < parseSemver($0.version) }
        guard !failures.isEmpty else { return }

        if failures.count == 1 {
            let f = failures[0]
            throw ZagError(
                message: "\(f.method) requires zag CLI >= \(f.version), " +
                    "but the installed version is \(detected). Please update the zag binary.",
                exitCode: nil,
                stderr: "")
        }

        let lines = failures.map { "  - \($0.method) requires >= \($0.version)" }.joined(separator: "\n")
        throw ZagError(
            message: "The following methods require a newer zag CLI version:\n\(lines)\n" +
                "Installed version: \(detected). Please update the zag binary.",
            exitCode: nil,
            stderr: "")
    }
    #endif

    /// Inject a version into the cache for testing.
    internal static func setVersionForTesting(bin: String, version: String) {
        cacheLock.lock()
        versionCache[bin] = version
        cacheLock.unlock()
    }

    /// Clear the version cache for testing.
    internal static func clearVersionCache() {
        cacheLock.lock()
        versionCache.removeAll()
        cacheLock.unlock()
    }
}
