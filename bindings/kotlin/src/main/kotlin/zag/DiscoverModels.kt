package zag

import kotlinx.serialization.*

/**
 * Feature support declaration for a provider capability.
 */
@Serializable
data class FeatureSupport(
    val supported: Boolean = false,
    @SerialName("native") val isNative: Boolean = false,
)

/**
 * Session log support with completeness level.
 */
@Serializable
data class SessionLogSupport(
    val supported: Boolean = false,
    @SerialName("native") val isNative: Boolean = false,
    val completeness: String? = null,
)

/**
 * Size alias mappings (small/medium/large to actual model names).
 */
@Serializable
data class SizeMappings(
    val small: String = "",
    val medium: String = "",
    val large: String = "",
)

/**
 * All feature flags for a provider.
 */
@Serializable
data class Features(
    val interactive: FeatureSupport = FeatureSupport(),
    @SerialName("non_interactive") val nonInteractive: FeatureSupport = FeatureSupport(),
    val resume: FeatureSupport = FeatureSupport(),
    @SerialName("resume_with_prompt") val resumeWithPrompt: FeatureSupport = FeatureSupport(),
    @SerialName("session_logs") val sessionLogs: SessionLogSupport = SessionLogSupport(),
    @SerialName("json_output") val jsonOutput: FeatureSupport = FeatureSupport(),
    @SerialName("stream_json") val streamJson: FeatureSupport = FeatureSupport(),
    @SerialName("json_schema") val jsonSchema: FeatureSupport = FeatureSupport(),
    @SerialName("input_format") val inputFormat: FeatureSupport = FeatureSupport(),
    @SerialName("streaming_input") val streamingInput: FeatureSupport = FeatureSupport(),
    val worktree: FeatureSupport = FeatureSupport(),
    val sandbox: FeatureSupport = FeatureSupport(),
    @SerialName("system_prompt") val systemPrompt: FeatureSupport = FeatureSupport(),
    @SerialName("auto_approve") val autoApprove: FeatureSupport = FeatureSupport(),
    val review: FeatureSupport = FeatureSupport(),
    @SerialName("add_dirs") val addDirs: FeatureSupport = FeatureSupport(),
    @SerialName("max_turns") val maxTurns: FeatureSupport = FeatureSupport(),
)

/**
 * Full capability declaration for a provider.
 */
@Serializable
data class ProviderCapability(
    val provider: String = "",
    @SerialName("default_model") val defaultModel: String = "",
    @SerialName("available_models") val availableModels: List<String> = emptyList(),
    @SerialName("size_mappings") val sizeMappings: SizeMappings = SizeMappings(),
    val features: Features = Features(),
)

/**
 * Result of resolving a model alias.
 */
@Serializable
data class ResolvedModel(
    val input: String = "",
    val resolved: String = "",
    @SerialName("is_alias") val isAlias: Boolean = false,
    val provider: String = "",
)
