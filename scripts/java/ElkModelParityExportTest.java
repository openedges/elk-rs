/*******************************************************************************
 * Copyright (c) 2026.
 *
 * This program and the accompanying materials are made available under the
 * terms of the Eclipse Public License 2.0 which is available at
 * http://www.eclipse.org/legal/epl-2.0.
 *
 * SPDX-License-Identifier: EPL-2.0
 *******************************************************************************/
package org.eclipse.elk.graph.json.test;

import static org.junit.Assert.assertTrue;

import java.io.BufferedWriter;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.Collections;
import java.util.Comparator;
import java.util.List;
import java.util.Locale;
import java.util.stream.Collectors;
import java.util.stream.Stream;

import org.eclipse.elk.alg.test.PlainJavaInitialization;
import org.eclipse.elk.core.RecursiveGraphLayoutEngine;
import org.eclipse.elk.core.options.CoreOptions;
import org.eclipse.elk.core.util.BasicProgressMonitor;
import org.eclipse.elk.core.util.Maybe;
import org.eclipse.elk.graph.ElkNode;
import org.eclipse.elk.graph.json.ElkGraphJson;
import org.eclipse.elk.graph.json.JsonImporter;
import org.eclipse.emf.common.util.URI;
import org.eclipse.emf.ecore.EObject;
import org.eclipse.emf.ecore.resource.Resource;
import org.eclipse.emf.ecore.resource.ResourceSet;
import org.eclipse.emf.ecore.resource.impl.ResourceSetImpl;
import org.junit.BeforeClass;
import org.junit.Test;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;

/**
 * Java-side model export runner for ELK-vs-elk-rs layout parity.
 */
@SuppressWarnings("restriction")
public class ElkModelParityExportTest {

    private static final String DEFAULT_MODELS_ROOT = "external/elk-models";
    private static final String DEFAULT_OUTPUT_DIR = "parity/model_parity/java";
    private static final String MANIFEST_FILE_NAME = "java_manifest.tsv";
    private static final String INPUT_DIR_NAME = "input";
    private static final String LAYOUT_DIR_NAME = "layout";
    private static final String HEADER =
            "model_rel_path\tinput_json\tjava_layout_json\tjava_status\tjava_error";

    private static final Gson PRETTY_GSON =
            new GsonBuilder().disableHtmlEscaping().setPrettyPrinting().create();

    @BeforeClass
    public static void init() {
        if (System.getenv("ELK_TRACE_BK_JAVA") != null) {
            System.setProperty("elk.trace.bk.java", "1");
        }
        PlainJavaInitialization.initializePlainJavaLayout();
    }

    @Test
    public void exportModelParityInputsAndJavaLayouts() throws IOException {
        if (!Boolean.parseBoolean(System.getProperty("elk.parity.run", "false"))) {
            return;
        }

        final Path modelsRoot = Paths.get(System.getProperty("elk.parity.modelsRoot", DEFAULT_MODELS_ROOT))
                .toAbsolutePath()
                .normalize();
        final Path outputDir = Paths.get(System.getProperty("elk.parity.outputDir", DEFAULT_OUTPUT_DIR))
                .toAbsolutePath()
                .normalize();
        final Path inputDir = outputDir.resolve(INPUT_DIR_NAME);
        final Path layoutDir = outputDir.resolve(LAYOUT_DIR_NAME);
        final Path manifestPath = outputDir.resolve(MANIFEST_FILE_NAME);

        final int limit = parseIntProperty("elk.parity.limit", 0);
        final boolean failFast = Boolean.parseBoolean(System.getProperty("elk.parity.failFast", "false"));
        final boolean prettyPrint = Boolean.parseBoolean(System.getProperty("elk.parity.prettyPrint", "false"));
        final boolean resetOutput = Boolean.parseBoolean(System.getProperty("elk.parity.resetOutput", "true"));
        final int randomSeed = parseIntProperty("elk.parity.randomSeed", 1);
        final List<String> includeTokens = parseCsvTokens(System.getProperty("elk.parity.include", ""));
        final List<String> excludeTokens = parseCsvTokens(System.getProperty("elk.parity.exclude", ""));

        if (!Files.exists(modelsRoot) || !Files.isDirectory(modelsRoot)) {
            throw new IOException("models root does not exist or is not a directory: " + modelsRoot);
        }

        if (resetOutput) {
            deleteRecursively(outputDir);
        }
        Files.createDirectories(inputDir);
        Files.createDirectories(layoutDir);

        final List<Path> modelFiles = collectModelFiles(modelsRoot, includeTokens, excludeTokens, limit);
        assertTrue("No model files found under: " + modelsRoot, !modelFiles.isEmpty());

        int successCount = 0;
        int failureCount = 0;

        try (BufferedWriter manifest = Files.newBufferedWriter(
                manifestPath,
                StandardCharsets.UTF_8)) {
            manifest.write(HEADER);
            manifest.newLine();

            for (Path modelFile : modelFiles) {
                final String relPath = toUnixPath(modelsRoot.relativize(modelFile));
                final Path inputJsonPath = inputDir.resolve(relPath + ".json");
                final Path javaLayoutJsonPath = layoutDir.resolve(relPath + ".json");

                String status = "ok";
                String error = "";

                try {
                    Files.createDirectories(inputJsonPath.getParent());
                    Files.createDirectories(javaLayoutJsonPath.getParent());

                    String inputJson;
                    boolean isJsonModel = modelFile.getFileName().toString().toLowerCase(Locale.ROOT).endsWith(".json");

                    if (isJsonModel) {
                        // .json models are already valid ELK JSON — use directly
                        inputJson = Files.readString(modelFile, StandardCharsets.UTF_8);
                    } else {
                        // .elkt/.elkg — load via EMF and convert to JSON
                        ElkNode loadedGraph = loadGraph(modelFile);
                        loadedGraph.setProperty(CoreOptions.RANDOM_SEED, randomSeed);
                        inputJson = ElkGraphJson.forGraph(loadedGraph)
                                .prettyPrint(prettyPrint)
                                .shortLayoutOptionKeys(false)
                                .omitZeroPositions(false)
                                .omitZeroDimension(false)
                                .omitLayout(false)
                                .omitUnknownLayoutOptions(false)
                                .toJson();
                    }
                    Files.writeString(inputJsonPath, inputJson, StandardCharsets.UTF_8);

                    JsonObject jsonGraph = JsonParser.parseString(inputJson).getAsJsonObject();
                    Maybe<JsonImporter> importerMaybe = new Maybe<>();
                    ElkNode layoutGraph = ElkGraphJson.forGraph(jsonGraph)
                            .rememberImporter(importerMaybe)
                            .lenient(false)
                            .toElk();

                    if (!isJsonModel) {
                        layoutGraph.setProperty(CoreOptions.RANDOM_SEED, randomSeed);
                    }

                    new RecursiveGraphLayoutEngine().layout(layoutGraph, new BasicProgressMonitor());
                    importerMaybe.get().transferLayout(layoutGraph);

                    String layoutJson = prettyPrint ? PRETTY_GSON.toJson(jsonGraph) : jsonGraph.toString();
                    Files.writeString(javaLayoutJsonPath, layoutJson, StandardCharsets.UTF_8);
                    successCount++;
                } catch (Throwable throwable) {
                    status = "error";
                    error = sanitize(throwable.toString());
                    failureCount++;
                    if (failFast) {
                        writeRow(manifest, relPath, inputJsonPath, javaLayoutJsonPath, status, error);
                        throw throwable;
                    }
                }

                writeRow(manifest, relPath, inputJsonPath, javaLayoutJsonPath, status, error);
            }
        }

        System.out.println(String.format(
                Locale.ROOT,
                "ELK model parity export completed: total=%d, success=%d, failed=%d, output=%s",
                modelFiles.size(),
                successCount,
                failureCount,
                outputDir));
    }

    private static ElkNode loadGraph(final Path modelFile) throws IOException {
        ResourceSet resourceSet = new ResourceSetImpl();
        Resource resource = resourceSet.getResource(URI.createFileURI(modelFile.toString()), true);
        if (resource == null) {
            throw new IOException("failed to create EMF resource for model: " + modelFile);
        }
        resource.load(Collections.emptyMap());
        if (resource.getContents().isEmpty()) {
            throw new IOException("model resource has no root object: " + modelFile);
        }
        EObject eObject = resource.getContents().get(0);
        if (!(eObject instanceof ElkNode)) {
            throw new IOException("model root is not ElkNode: " + modelFile);
        }
        return (ElkNode) eObject;
    }

    private static List<Path> collectModelFiles(
            final Path modelsRoot,
            final List<String> includeTokens,
            final List<String> excludeTokens,
            final int limit) throws IOException {

        try (Stream<Path> stream = Files.walk(modelsRoot)) {
            List<Path> files = stream
                    .filter(Files::isRegularFile)
                    .filter(ElkModelParityExportTest::isSupportedModelFile)
                    .filter(path -> !matchesExcludeTokens(modelsRoot, path, excludeTokens))
                    .filter(path -> matchesIncludeTokens(modelsRoot, path, includeTokens))
                    .sorted(Comparator.comparing(path -> toUnixPath(modelsRoot.relativize(path))))
                    .collect(Collectors.toList());

            if (limit > 0 && files.size() > limit) {
                return new ArrayList<>(files.subList(0, limit));
            }
            return files;
        }
    }

    private static boolean isSupportedModelFile(final Path path) {
        String name = path.getFileName().toString().toLowerCase(Locale.ROOT);
        return name.endsWith(".elkt") || name.endsWith(".elkg") || name.endsWith(".json");
    }

    private static boolean matchesIncludeTokens(
            final Path modelsRoot,
            final Path path,
            final List<String> includeTokens) {
        if (includeTokens.isEmpty()) {
            return true;
        }
        String relPath = toUnixPath(modelsRoot.relativize(path)).toLowerCase(Locale.ROOT);
        for (String token : includeTokens) {
            if (relPath.contains(token.toLowerCase(Locale.ROOT))) {
                return true;
            }
        }
        return false;
    }

    private static boolean matchesExcludeTokens(
            final Path modelsRoot,
            final Path path,
            final List<String> excludeTokens) {
        if (excludeTokens.isEmpty()) {
            return false;
        }
        String relPath = toUnixPath(modelsRoot.relativize(path)).toLowerCase(Locale.ROOT);
        for (String token : excludeTokens) {
            if (relPath.contains(token.toLowerCase(Locale.ROOT))) {
                return true;
            }
        }
        return false;
    }

    private static int parseIntProperty(final String key, final int fallback) {
        String value = System.getProperty(key);
        if (value == null || value.isBlank()) {
            return fallback;
        }
        try {
            return Integer.parseInt(value.trim());
        } catch (NumberFormatException exception) {
            return fallback;
        }
    }

    private static List<String> parseCsvTokens(final String raw) {
        if (raw == null || raw.isBlank()) {
            return Collections.emptyList();
        }
        return Stream.of(raw.split(","))
                .map(String::trim)
                .filter(token -> !token.isEmpty())
                .collect(Collectors.toList());
    }

    private static void writeRow(
            final BufferedWriter writer,
            final String modelRelPath,
            final Path inputJsonPath,
            final Path javaLayoutJsonPath,
            final String status,
            final String error) throws IOException {

        writer.write(String.join(
                "\t",
                sanitize(modelRelPath),
                sanitize(inputJsonPath.toString()),
                sanitize(javaLayoutJsonPath.toString()),
                sanitize(status),
                sanitize(error)));
        writer.newLine();
    }

    private static String sanitize(final String value) {
        if (value == null) {
            return "";
        }
        return value.replace('\t', ' ')
                .replace('\n', ' ')
                .replace('\r', ' ');
    }

    private static String toUnixPath(final Path path) {
        return path.toString().replace('\\', '/');
    }

    private static void deleteRecursively(final Path target) throws IOException {
        if (!Files.exists(target)) {
            return;
        }
        try (Stream<Path> walk = Files.walk(target)) {
            for (Path path : walk.sorted(Comparator.reverseOrder()).collect(Collectors.toList())) {
                Files.deleteIfExists(path);
            }
        }
    }
}
