import Foundation

// MARK: - FeatureSupport

/// Whether a provider supports a given feature and whether support is native.
public struct FeatureSupport: Codable, Equatable, Sendable {
    public let supported: Bool
    public let native: Bool

    public init(supported: Bool, native: Bool) {
        self.supported = supported
        self.native = native
    }
}

// MARK: - SessionLogSupport

/// Session log support with optional completeness level.
public struct SessionLogSupport: Codable, Equatable, Sendable {
    public let supported: Bool
    public let native: Bool
    public let completeness: String?

    public init(supported: Bool, native: Bool, completeness: String? = nil) {
        self.supported = supported
        self.native = native
        self.completeness = completeness
    }
}

// MARK: - SizeMappings

/// Size alias mappings (small/medium/large to actual model names).
public struct SizeMappings: Codable, Equatable, Sendable {
    public let small: String
    public let medium: String
    public let large: String

    public init(small: String, medium: String, large: String) {
        self.small = small
        self.medium = medium
        self.large = large
    }
}

// MARK: - Features

/// All feature flags for a provider.
public struct Features: Codable, Equatable, Sendable {
    public let interactive: FeatureSupport
    public let nonInteractive: FeatureSupport
    public let resume: FeatureSupport
    public let resumeWithPrompt: FeatureSupport
    public let sessionLogs: SessionLogSupport
    public let jsonOutput: FeatureSupport
    public let streamJson: FeatureSupport
    public let jsonSchema: FeatureSupport
    public let inputFormat: FeatureSupport
    public let streamingInput: FeatureSupport
    public let worktree: FeatureSupport
    public let sandbox: FeatureSupport
    public let systemPrompt: FeatureSupport
    public let autoApprove: FeatureSupport
    public let review: FeatureSupport
    public let addDirs: FeatureSupport
    public let maxTurns: FeatureSupport

    private enum CodingKeys: String, CodingKey {
        case interactive
        case nonInteractive = "non_interactive"
        case resume
        case resumeWithPrompt = "resume_with_prompt"
        case sessionLogs = "session_logs"
        case jsonOutput = "json_output"
        case streamJson = "stream_json"
        case jsonSchema = "json_schema"
        case inputFormat = "input_format"
        case streamingInput = "streaming_input"
        case worktree
        case sandbox
        case systemPrompt = "system_prompt"
        case autoApprove = "auto_approve"
        case review
        case addDirs = "add_dirs"
        case maxTurns = "max_turns"
    }

    public init(
        interactive: FeatureSupport,
        nonInteractive: FeatureSupport,
        resume: FeatureSupport,
        resumeWithPrompt: FeatureSupport,
        sessionLogs: SessionLogSupport,
        jsonOutput: FeatureSupport,
        streamJson: FeatureSupport,
        jsonSchema: FeatureSupport,
        inputFormat: FeatureSupport,
        streamingInput: FeatureSupport,
        worktree: FeatureSupport,
        sandbox: FeatureSupport,
        systemPrompt: FeatureSupport,
        autoApprove: FeatureSupport,
        review: FeatureSupport,
        addDirs: FeatureSupport,
        maxTurns: FeatureSupport
    ) {
        self.interactive = interactive
        self.nonInteractive = nonInteractive
        self.resume = resume
        self.resumeWithPrompt = resumeWithPrompt
        self.sessionLogs = sessionLogs
        self.jsonOutput = jsonOutput
        self.streamJson = streamJson
        self.jsonSchema = jsonSchema
        self.inputFormat = inputFormat
        self.streamingInput = streamingInput
        self.worktree = worktree
        self.sandbox = sandbox
        self.systemPrompt = systemPrompt
        self.autoApprove = autoApprove
        self.review = review
        self.addDirs = addDirs
        self.maxTurns = maxTurns
    }
}

// MARK: - ProviderCapability

/// Full capability declaration for a provider.
public struct ProviderCapability: Codable, Equatable, Sendable {
    public let provider: String
    public let defaultModel: String
    public let availableModels: [String]
    public let sizeMappings: SizeMappings
    public let features: Features

    private enum CodingKeys: String, CodingKey {
        case provider
        case defaultModel = "default_model"
        case availableModels = "available_models"
        case sizeMappings = "size_mappings"
        case features
    }

    public init(
        provider: String,
        defaultModel: String,
        availableModels: [String],
        sizeMappings: SizeMappings,
        features: Features
    ) {
        self.provider = provider
        self.defaultModel = defaultModel
        self.availableModels = availableModels
        self.sizeMappings = sizeMappings
        self.features = features
    }
}

// MARK: - ResolvedModel

/// Result of resolving a model alias.
public struct ResolvedModel: Codable, Equatable, Sendable {
    public let input: String
    public let resolved: String
    public let isAlias: Bool
    public let provider: String

    private enum CodingKeys: String, CodingKey {
        case input
        case resolved
        case isAlias = "is_alias"
        case provider
    }

    public init(input: String, resolved: String, isAlias: Bool, provider: String) {
        self.input = input
        self.resolved = resolved
        self.isAlias = isAlias
        self.provider = provider
    }
}
