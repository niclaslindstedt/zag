#if os(macOS) || os(Linux)
import Foundation

/// Provider and model discovery functions for zag.
///
/// Available on macOS and Linux only (requires Foundation.Process).
public enum ZagDiscover {

    /// List all available provider names.
    ///
    /// - Parameter bin: Path to the zag binary (defaults to `ZAG_BIN` env or `"zag"`).
    /// - Returns: An array of provider name strings.
    public static func listProviders(bin: String? = nil) async throws -> [String] {
        let caps = try await getAllCapabilities(bin: bin)
        return caps.map { $0.provider }
    }

    /// Get capability declarations for a specific provider.
    ///
    /// - Parameters:
    ///   - provider: Provider name (e.g. `"claude"`, `"codex"`, `"gemini"`, `"copilot"`, `"ollama"`).
    ///   - bin: Path to the zag binary (defaults to `ZAG_BIN` env or `"zag"`).
    /// - Returns: The provider's capability declaration.
    public static func getCapability(
        provider: String,
        bin: String? = nil
    ) async throws -> ProviderCapability {
        let b = bin ?? ZagProcess.defaultBin
        return try await discoverExec(bin: b, args: ["-p", provider])
    }

    /// Get capability declarations for all providers.
    ///
    /// - Parameter bin: Path to the zag binary (defaults to `ZAG_BIN` env or `"zag"`).
    /// - Returns: An array of capability declarations, one per provider.
    public static func getAllCapabilities(
        bin: String? = nil
    ) async throws -> [ProviderCapability] {
        let b = bin ?? ZagProcess.defaultBin
        return try await discoverExec(bin: b, args: [])
    }

    /// Resolve a model alias for a given provider.
    ///
    /// Size aliases (`small`/`s`, `medium`/`m`/`default`, `large`/`l`/`max`) are
    /// resolved to the provider-specific model. Non-alias names pass through unchanged.
    ///
    /// - Parameters:
    ///   - provider: Provider name.
    ///   - model: Model name or alias to resolve.
    ///   - bin: Path to the zag binary (defaults to `ZAG_BIN` env or `"zag"`).
    /// - Returns: The resolved model information.
    public static func resolveModel(
        provider: String,
        model: String,
        bin: String? = nil
    ) async throws -> ResolvedModel {
        let b = bin ?? ZagProcess.defaultBin
        return try await discoverExec(bin: b, args: ["-p", provider, "--resolve", model])
    }

    // MARK: - Private

    private static func discoverExec<T: Decodable>(
        bin: String,
        args: [String]
    ) async throws -> T {
        let fullArgs = ["discover"] + args + ["--json"]
        let (stdout, stderr, exitCode) = try await collectOutput(bin: bin, args: fullArgs)

        if exitCode != 0 {
            let detail = stderr.isEmpty ? stdout : stderr
            throw ZagError(
                message: "zag exited with code \(exitCode): \(detail)",
                exitCode: Int(exitCode),
                stderr: stderr)
        }

        guard let data = stdout.data(using: .utf8) else {
            throw ZagError(
                message: "Failed to read zag output as UTF-8",
                exitCode: Int(exitCode),
                stderr: stderr)
        }

        do {
            return try JSONDecoder().decode(T.self, from: data)
        } catch {
            let preview = String(stdout.prefix(200))
            throw ZagError(
                message: "Failed to parse zag JSON output: \(preview)",
                exitCode: Int(exitCode),
                stderr: stderr)
        }
    }

    private static func collectOutput(
        bin: String, args: [String]
    ) async throws -> (stdout: String, stderr: String, exitCode: Int32) {
        let (executableURL, fullArgs) = resolveCommand(bin: bin, args: args)

        let process = Process()
        process.executableURL = executableURL
        process.arguments = fullArgs

        let stdoutPipe = Pipe()
        let stderrPipe = Pipe()
        process.standardOutput = stdoutPipe
        process.standardError = stderrPipe
        process.standardInput = FileHandle.nullDevice

        do {
            try process.run()
        } catch {
            throw ZagError(
                message: "Failed to start '\(bin)': \(error)",
                exitCode: nil, stderr: "")
        }

        let stdoutData = stdoutPipe.fileHandleForReading.readDataToEndOfFile()
        let stderrData = stderrPipe.fileHandleForReading.readDataToEndOfFile()

        process.waitUntilExit()

        let stdout = String(data: stdoutData, encoding: .utf8) ?? ""
        let stderr = String(data: stderrData, encoding: .utf8) ?? ""

        return (stdout, stderr, process.terminationStatus)
    }

    /// Resolve binary to an executable URL and adjusted arguments.
    private static func resolveCommand(bin: String, args: [String]) -> (URL, [String]) {
        if bin.contains("/") {
            return (URL(fileURLWithPath: bin), args)
        }
        return (URL(fileURLWithPath: "/usr/bin/env"), [bin] + args)
    }
}
#endif
