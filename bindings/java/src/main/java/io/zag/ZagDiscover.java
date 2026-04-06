package io.zag;

import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;

/** Discovery helpers for querying provider capabilities via the zag CLI. */
public final class ZagDiscover {

    private static final ObjectMapper MAPPER =
            new ObjectMapper().configure(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES, false);

    private ZagDiscover() {}

    /**
     * Run a zag discover subcommand and return the raw stdout.
     */
    private static String discoverExec(String bin, List<String> extraArgs) throws ZagException {
        List<String> command = new ArrayList<>();
        command.add(bin);
        command.add("discover");
        command.addAll(extraArgs);
        command.add("--json");

        try {
            ProcessBuilder pb = new ProcessBuilder(command);
            pb.redirectErrorStream(false);
            Process process = pb.start();

            String stdout =
                    new String(process.getInputStream().readAllBytes(), StandardCharsets.UTF_8);
            String stderr =
                    new String(process.getErrorStream().readAllBytes(), StandardCharsets.UTF_8);

            int exitCode = process.waitFor();
            if (exitCode != 0) {
                String msg = stderr.isEmpty() ? stdout : stderr;
                throw new ZagException(
                        "zag exited with code " + exitCode + ": " + msg, exitCode, stderr);
            }

            return stdout;
        } catch (ZagException e) {
            throw e;
        } catch (IOException | InterruptedException e) {
            throw new ZagException("Failed to run zag: " + e.getMessage(), null, "");
        }
    }

    /**
     * List all available provider names.
     *
     * @param bin path to the zag binary (defaults to {@code ZAG_BIN} env or {@code "zag"})
     * @return list of provider name strings
     */
    public static List<String> listProviders(String bin) throws ZagException {
        List<ProviderCapability> caps = getAllCapabilities(bin);
        List<String> names = new ArrayList<>(caps.size());
        for (ProviderCapability cap : caps) {
            names.add(cap.provider());
        }
        return names;
    }

    /** List all available provider names using the default binary. */
    public static List<String> listProviders() throws ZagException {
        return listProviders(ZagProcess.defaultBin());
    }

    /**
     * Get capability declarations for a specific provider.
     *
     * @param provider provider name (e.g. "claude", "codex", "gemini", "copilot", "ollama")
     * @param bin path to the zag binary
     * @return the provider's capability declaration
     */
    public static ProviderCapability getCapability(String provider, String bin)
            throws ZagException {
        String json = discoverExec(bin, List.of("-p", provider));
        try {
            return MAPPER.readValue(json, ProviderCapability.class);
        } catch (IOException e) {
            throw new ZagException(
                    "Failed to parse zag JSON output: " + json.substring(0, Math.min(json.length(), 200)),
                    null, "");
        }
    }

    /** Get capability declarations for a specific provider using the default binary. */
    public static ProviderCapability getCapability(String provider) throws ZagException {
        return getCapability(provider, ZagProcess.defaultBin());
    }

    /**
     * Get capability declarations for all providers.
     *
     * @param bin path to the zag binary
     * @return list of all provider capability declarations
     */
    public static List<ProviderCapability> getAllCapabilities(String bin) throws ZagException {
        String json = discoverExec(bin, List.of());
        try {
            return MAPPER.readValue(json, new TypeReference<List<ProviderCapability>>() {});
        } catch (IOException e) {
            throw new ZagException(
                    "Failed to parse zag JSON output: " + json.substring(0, Math.min(json.length(), 200)),
                    null, "");
        }
    }

    /** Get capability declarations for all providers using the default binary. */
    public static List<ProviderCapability> getAllCapabilities() throws ZagException {
        return getAllCapabilities(ZagProcess.defaultBin());
    }

    /**
     * Resolve a model alias for a given provider.
     *
     * <p>Size aliases ({@code small}/{@code s}, {@code medium}/{@code m}/{@code default},
     * {@code large}/{@code l}/{@code max}) are resolved to the provider-specific model.
     * Non-alias names pass through unchanged.
     *
     * @param provider provider name
     * @param model model name or alias to resolve
     * @param bin path to the zag binary
     * @return the resolved model information
     */
    public static ResolvedModel resolveModel(String provider, String model, String bin)
            throws ZagException {
        String json = discoverExec(bin, List.of("-p", provider, "--resolve", model));
        try {
            return MAPPER.readValue(json, ResolvedModel.class);
        } catch (IOException e) {
            throw new ZagException(
                    "Failed to parse zag JSON output: " + json.substring(0, Math.min(json.length(), 200)),
                    null, "");
        }
    }

    /** Resolve a model alias for a given provider using the default binary. */
    public static ResolvedModel resolveModel(String provider, String model) throws ZagException {
        return resolveModel(provider, model, ZagProcess.defaultBin());
    }
}
