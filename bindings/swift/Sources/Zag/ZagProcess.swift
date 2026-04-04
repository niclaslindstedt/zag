import Foundation

/// Subprocess helpers for invoking the zag CLI.
public enum ZagProcess {

    /// Default zag binary path from `ZAG_BIN` env or `"zag"`.
    public static var defaultBin: String {
        ProcessInfo.processInfo.environment["ZAG_BIN"] ?? "zag"
    }

    // MARK: - exec

    /// Run zag and return parsed `AgentOutput`.
    public static func exec(bin: String, args: [String]) async throws -> AgentOutput {
        let (stdout, stderr, exitCode) = try await collectOutput(bin: bin, args: args)

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
            return try JSONDecoder.zag.decode(AgentOutput.self, from: data)
        } catch {
            let preview = String(stdout.prefix(200))
            throw ZagError(
                message: "Failed to parse zag JSON output: \(preview)",
                exitCode: Int(exitCode),
                stderr: stderr)
        }
    }

    // MARK: - stream

    /// Run zag in streaming mode, yielding `Event` objects from NDJSON.
    public static func stream(bin: String, args: [String]) -> AsyncThrowingStream<Event, Error> {
        let (executableURL, fullArgs) = resolveCommand(bin: bin, args: args)

        return AsyncThrowingStream { continuation in
            let task = Task {
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
                    continuation.finish(throwing: ZagError(
                        message: "Failed to start '\(bin)': \(error)",
                        exitCode: nil, stderr: ""))
                    return
                }

                let handle = stdoutPipe.fileHandleForReading
                for try await line in handle.bytes.lines {
                    let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
                    if trimmed.isEmpty { continue }
                    guard let data = trimmed.data(using: .utf8) else { continue }
                    do {
                        let event = try JSONDecoder.zag.decode(Event.self, from: data)
                        continuation.yield(event)
                    } catch {
                        continue
                    }
                }

                process.waitUntilExit()

                if process.terminationStatus != 0 {
                    let stderrData = stderrPipe.fileHandleForReading.readDataToEndOfFile()
                    let stderr = String(data: stderrData, encoding: .utf8) ?? ""
                    continuation.finish(throwing: ZagError(
                        message: "zag exited with code \(process.terminationStatus)",
                        exitCode: Int(process.terminationStatus),
                        stderr: stderr))
                } else {
                    continuation.finish()
                }
            }

            continuation.onTermination = { _ in
                task.cancel()
            }
        }
    }

    // MARK: - startStreamingProcess

    /// Start a streaming process with piped stdin and stdout.
    public static func startStreamingProcess(bin: String, args: [String]) throws -> StreamingSession {
        let (executableURL, fullArgs) = resolveCommand(bin: bin, args: args)

        let process = Process()
        process.executableURL = executableURL
        process.arguments = fullArgs

        let stdinPipe = Pipe()
        let stdoutPipe = Pipe()
        let stderrPipe = Pipe()
        process.standardInput = stdinPipe
        process.standardOutput = stdoutPipe
        process.standardError = stderrPipe

        do {
            try process.run()
        } catch {
            throw ZagError(
                message: "Failed to start '\(bin)': \(error)",
                exitCode: nil, stderr: "")
        }

        return StreamingSession(
            process: process,
            stdinPipe: stdinPipe,
            stdoutPipe: stdoutPipe,
            stderrPipe: stderrPipe)
    }

    // MARK: - run (interactive)

    /// Run zag interactively with inherited stdio.
    public static func runInteractive(bin: String, args: [String]) async throws {
        let (executableURL, fullArgs) = resolveCommand(bin: bin, args: args)

        let process = Process()
        process.executableURL = executableURL
        process.arguments = fullArgs

        do {
            try process.run()
        } catch {
            throw ZagError(
                message: "Failed to start '\(bin)': \(error)",
                exitCode: nil, stderr: "")
        }

        process.waitUntilExit()

        if process.terminationStatus != 0 {
            throw ZagError(
                message: "zag exited with code \(process.terminationStatus)",
                exitCode: Int(process.terminationStatus),
                stderr: "")
        }
    }

    // MARK: - Private helpers

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
    /// If `bin` contains a `/`, it's treated as a direct path.
    /// Otherwise, `/usr/bin/env` is used to locate it via PATH.
    private static func resolveCommand(bin: String, args: [String]) -> (URL, [String]) {
        if bin.contains("/") {
            return (URL(fileURLWithPath: bin), args)
        }
        return (URL(fileURLWithPath: "/usr/bin/env"), [bin] + args)
    }
}

// MARK: - StreamingSession

/// A live streaming session with piped stdin and stdout.
public final class StreamingSession {
    private let process: Process
    private let stdinPipe: Pipe
    private let stdoutPipe: Pipe
    private let stderrPipe: Pipe

    init(process: Process, stdinPipe: Pipe, stdoutPipe: Pipe, stderrPipe: Pipe) {
        self.process = process
        self.stdinPipe = stdinPipe
        self.stdoutPipe = stdoutPipe
        self.stderrPipe = stderrPipe
    }

    /// Send a raw NDJSON line to the agent's stdin.
    public func send(_ message: String) throws {
        guard let data = (message + "\n").data(using: .utf8) else { return }
        stdinPipe.fileHandleForWriting.write(data)
    }

    /// Send a user message to the agent.
    public func sendUserMessage(_ content: String) throws {
        let payload: [String: String] = ["type": "user_message", "content": content]
        let data = try JSONSerialization.data(withJSONObject: payload)
        guard let json = String(data: data, encoding: .utf8) else { return }
        try send(json)
    }

    /// Close stdin to signal no more input.
    public func closeInput() {
        stdinPipe.fileHandleForWriting.closeFile()
    }

    /// Async stream of parsed `Event` objects from stdout.
    public var events: AsyncThrowingStream<Event, Error> {
        let handle = stdoutPipe.fileHandleForReading
        return AsyncThrowingStream { continuation in
            Task {
                for try await line in handle.bytes.lines {
                    let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
                    if trimmed.isEmpty { continue }
                    guard let data = trimmed.data(using: .utf8) else { continue }
                    do {
                        let event = try JSONDecoder.zag.decode(Event.self, from: data)
                        continuation.yield(event)
                    } catch {
                        continue
                    }
                }
                continuation.finish()
            }
        }
    }

    /// Wait for the process to exit. Throws `ZagError` on non-zero exit.
    public func wait() async throws {
        closeInput()
        process.waitUntilExit()

        if process.terminationStatus != 0 {
            let stderrData = stderrPipe.fileHandleForReading.readDataToEndOfFile()
            let stderr = String(data: stderrData, encoding: .utf8) ?? ""
            throw ZagError(
                message: "zag exited with code \(process.terminationStatus)",
                exitCode: Int(process.terminationStatus),
                stderr: stderr)
        }
    }
}
