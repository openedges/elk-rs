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

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.nio.file.StandardOpenOption;
import java.time.Instant;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;

import org.eclipse.elk.alg.test.PlainJavaInitialization;
import org.eclipse.elk.core.RecursiveGraphLayoutEngine;
import org.eclipse.elk.core.util.BasicProgressMonitor;
import org.eclipse.elk.core.util.Maybe;
import org.eclipse.elk.graph.ElkNode;
import org.eclipse.elk.graph.json.ElkGraphJson;
import org.eclipse.elk.graph.json.JsonImporter;
import org.junit.BeforeClass;
import org.junit.Test;

import com.google.gson.JsonObject;
import com.google.gson.JsonParser;

/**
 * Java-side model benchmark for 5-way performance comparison.
 *
 * <p>Supports two modes:</p>
 * <ul>
 *   <li><b>synthetic</b> — runs 5 hardcoded issue scenarios (same as LayeredIssueParityBenchTest)</li>
 *   <li><b>models</b> — reads input JSON from a manifest TSV and benchmarks each model</li>
 * </ul>
 *
 * <p>This test is opt-in. It only runs when {@code -Delk.parity.run=true} is set.</p>
 *
 * <p>System properties:</p>
 * <ul>
 *   <li>{@code elk.bench.mode} — "synthetic" or "models" (default: "models")</li>
 *   <li>{@code elk.bench.manifest} — manifest TSV path (models mode)</li>
 *   <li>{@code elk.bench.iterations} — iterations per scenario (default: 20)</li>
 *   <li>{@code elk.bench.warmup} — warmup iterations (default: 3)</li>
 *   <li>{@code elk.bench.output} — CSV output path</li>
 *   <li>{@code elk.bench.limit} — max models (default: 50, 0=unlimited)</li>
 * </ul>
 */
public class ElkModelBenchTest {

    private static final String ENGINE = "java";
    private static final int DEFAULT_ITERATIONS = 20;
    private static final int DEFAULT_WARMUP = 3;
    private static final int DEFAULT_LIMIT = 50;
    private static final String DEFAULT_OUTPUT = "parity/java_model_bench_results.csv";
    private static final String CSV_HEADER =
            "timestamp,engine,scenario,iterations,warmup,elapsed_nanos,avg_ms,ops_per_sec";

    @BeforeClass
    public static void init() {
        PlainJavaInitialization.initializePlainJavaLayout();
    }

    @Test
    public void runModelBenchmark() throws IOException {
        if (!Boolean.parseBoolean(System.getProperty("elk.parity.run", "false"))) {
            return;
        }

        final String mode = System.getProperty("elk.bench.mode", "models");
        final int iterations = parseIntProperty("elk.bench.iterations", DEFAULT_ITERATIONS);
        final int warmup = parseIntProperty("elk.bench.warmup", DEFAULT_WARMUP);
        final String outputPath = System.getProperty("elk.bench.output", DEFAULT_OUTPUT);
        final int limit = parseIntProperty("elk.bench.limit", DEFAULT_LIMIT);

        final Path output = Paths.get(outputPath);
        if (output.getParent() != null) {
            Files.createDirectories(output.getParent());
        }

        // Write CSV header
        Files.write(output, (CSV_HEADER + "\n").getBytes(StandardCharsets.UTF_8),
                StandardOpenOption.CREATE, StandardOpenOption.TRUNCATE_EXISTING);

        int executed = 0;

        if ("synthetic".equals(mode)) {
            executed = runSyntheticBenchmarks(iterations, warmup, output);
        } else {
            final String manifestPath = System.getProperty("elk.bench.manifest",
                    "parity/model_parity/java/java_manifest.tsv");
            executed = runModelBenchmarks(manifestPath, iterations, warmup, limit, output);
        }

        assertTrue("No benchmarks were executed.", executed > 0);
    }

    // -----------------------------------------------------------------------
    // Synthetic mode
    // -----------------------------------------------------------------------

    private static int runSyntheticBenchmarks(
            final int iterations, final int warmup, final Path output) throws IOException {

        final String[][] scenarios = {
            { "issue_405", ISSUE_405_JSON },
            { "issue_603", ISSUE_603_JSON },
            { "issue_680", ISSUE_680_JSON },
            { "issue_871", ISSUE_871_JSON },
            { "issue_905", ISSUE_905_JSON },
        };

        int count = 0;
        for (String[] entry : scenarios) {
            runJsonBenchmark(entry[0], entry[1], iterations, warmup, output);
            count++;
        }
        return count;
    }

    // -----------------------------------------------------------------------
    // Models mode
    // -----------------------------------------------------------------------

    private static int runModelBenchmarks(
            final String manifestPath,
            final int iterations,
            final int warmup,
            final int limit,
            final Path output) throws IOException {

        final Path manifest = Paths.get(manifestPath);
        if (!Files.exists(manifest)) {
            System.out.println("Manifest not found: " + manifestPath);
            return 0;
        }

        final List<String> lines = Files.readAllLines(manifest, StandardCharsets.UTF_8);
        int count = 0;
        int errors = 0;

        for (String line : lines) {
            // Skip BOM and header
            String trimmed = line.startsWith("\uFEFF") ? line.substring(1) : line;
            if (trimmed.startsWith("model_rel_path")) continue;

            String[] cols = trimmed.split("\t");
            if (cols.length < 5) continue;

            String modelRel = cols[0];
            String inputJsonPath = cols[1];
            String status = cols[3];

            if (!"ok".equals(status)) continue;

            Path jsonPath = Paths.get(inputJsonPath);
            if (!Files.exists(jsonPath)) continue;

            String scenarioName = modelRel
                    .replaceAll("\\.[^.]+$", "")
                    .replace('/', '_')
                    .replace('\\', '_');

            try {
                String json = Files.readString(jsonPath, StandardCharsets.UTF_8);
                runJsonBenchmark(scenarioName, json, iterations, warmup, output);
                count++;
            } catch (Throwable t) {
                errors++;
                if (errors <= 3) {
                    System.out.println("  " + scenarioName + ": ERROR — " +
                            t.toString().substring(0, Math.min(120, t.toString().length())));
                }
            }

            if (limit > 0 && count >= limit) break;
        }

        System.out.println(String.format(Locale.ROOT,
                "Java model benchmark: %d ok, %d errors", count, errors));
        return count;
    }

    // -----------------------------------------------------------------------
    // Core benchmark
    // -----------------------------------------------------------------------

    private static void runJsonBenchmark(
            final String scenarioName,
            final String graphJson,
            final int iterations,
            final int warmup,
            final Path output) throws IOException {

        final RecursiveGraphLayoutEngine engine = new RecursiveGraphLayoutEngine();

        // Warmup
        for (int i = 0; i < warmup; i++) {
            layoutFromJson(engine, graphJson);
        }

        // Timed iterations
        final long start = System.nanoTime();
        for (int i = 0; i < iterations; i++) {
            layoutFromJson(engine, graphJson);
        }
        final long elapsedNanos = Math.max(1L, System.nanoTime() - start);

        final double avgMs = elapsedNanos / (double) iterations / 1_000_000.0;
        final double opsPerSec = iterations / (elapsedNanos / 1_000_000_000.0);
        final long timestamp = Instant.now().getEpochSecond();

        final String line = String.format(
                Locale.ROOT,
                "%d,%s,%s,%d,%d,%d,%.6f,%.2f%n",
                timestamp,
                ENGINE,
                scenarioName,
                iterations,
                warmup,
                elapsedNanos,
                avgMs,
                opsPerSec);
        Files.write(output, line.getBytes(StandardCharsets.UTF_8),
                StandardOpenOption.CREATE, StandardOpenOption.APPEND);
    }

    private static void layoutFromJson(
            final RecursiveGraphLayoutEngine engine,
            final String graphJson) {

        JsonObject jsonGraph = JsonParser.parseString(graphJson).getAsJsonObject();
        Maybe<JsonImporter> importerMaybe = new Maybe<>();
        ElkNode root = ElkGraphJson.forGraph(jsonGraph)
                .rememberImporter(importerMaybe)
                .lenient(false)
                .toElk();
        engine.layout(root, new BasicProgressMonitor());
        importerMaybe.get().transferLayout(root);
    }

    // -----------------------------------------------------------------------
    // Utilities
    // -----------------------------------------------------------------------

    private static int parseIntProperty(final String property, final int fallback) {
        final String value = System.getProperty(property);
        if (value == null || value.isBlank()) return fallback;
        try {
            return Math.max(1, Integer.parseInt(value));
        } catch (NumberFormatException e) {
            return fallback;
        }
    }

    // -----------------------------------------------------------------------
    // Synthetic scenario JSON (same graphs as Rust/JS benchmarks)
    // -----------------------------------------------------------------------

    private static final String ISSUE_405_JSON = "{"
            + "\"id\":\"root\","
            + "\"layoutOptions\":{"
            + "\"org.eclipse.elk.algorithm\":\"org.eclipse.elk.layered\","
            + "\"org.eclipse.elk.direction\":\"RIGHT\","
            + "\"org.eclipse.elk.edgeRouting\":\"ORTHOGONAL\"},"
            + "\"children\":["
            + "{\"id\":\"reference\",\"width\":80,\"height\":60,"
            + "\"layoutOptions\":{\"org.eclipse.elk.portConstraints\":\"FIXED_SIDE\","
            + "\"org.eclipse.elk.portLabels.placement\":\"OUTSIDE NEXT_TO_PORT_IF_POSSIBLE\"},"
            + "\"ports\":["
            + "{\"id\":\"west\",\"width\":10,\"height\":10,"
            + "\"layoutOptions\":{\"org.eclipse.elk.port.side\":\"WEST\"},"
            + "\"labels\":[{\"text\":\"west\",\"width\":20,\"height\":10}]},"
            + "{\"id\":\"east\",\"width\":10,\"height\":10,"
            + "\"layoutOptions\":{\"org.eclipse.elk.port.side\":\"EAST\"},"
            + "\"labels\":[{\"text\":\"east\",\"width\":20,\"height\":10}]},"
            + "{\"id\":\"north\",\"width\":10,\"height\":10,"
            + "\"layoutOptions\":{\"org.eclipse.elk.port.side\":\"NORTH\"},"
            + "\"labels\":[{\"text\":\"north\",\"width\":20,\"height\":10}]},"
            + "{\"id\":\"south\",\"width\":10,\"height\":10,"
            + "\"layoutOptions\":{\"org.eclipse.elk.port.side\":\"SOUTH\"},"
            + "\"labels\":[{\"text\":\"south\",\"width\":20,\"height\":10}]}"
            + "]},"
            + "{\"id\":\"westPartner\",\"width\":30,\"height\":20},"
            + "{\"id\":\"eastPartner\",\"width\":30,\"height\":20},"
            + "{\"id\":\"northPartner\",\"width\":30,\"height\":20},"
            + "{\"id\":\"southPartner\",\"width\":30,\"height\":20}],"
            + "\"edges\":["
            + "{\"id\":\"e_west\",\"sources\":[\"westPartner\"],\"targets\":[\"west\"]},"
            + "{\"id\":\"e_east\",\"sources\":[\"east\"],\"targets\":[\"eastPartner\"]},"
            + "{\"id\":\"e_north\",\"sources\":[\"north\"],\"targets\":[\"northPartner\"]},"
            + "{\"id\":\"e_south\",\"sources\":[\"southPartner\"],\"targets\":[\"south\"]}"
            + "]}";

    private static final String ISSUE_603_JSON = "{"
            + "\"id\":\"root\","
            + "\"layoutOptions\":{"
            + "\"org.eclipse.elk.algorithm\":\"org.eclipse.elk.layered\","
            + "\"org.eclipse.elk.nodeLabels.padding\":\"[top=24.0,left=0.0,bottom=0.0,right=0.0]\"},"
            + "\"children\":[{"
            + "\"id\":\"compound\",\"width\":120,\"height\":80,"
            + "\"layoutOptions\":{"
            + "\"org.eclipse.elk.nodeLabels.placement\":\"INSIDE V_TOP H_CENTER\","
            + "\"org.eclipse.elk.nodeLabels.padding\":\"[top=24.0,left=0.0,bottom=0.0,right=0.0]\"},"
            + "\"labels\":[{\"text\":\"compound\",\"width\":40,\"height\":16}],"
            + "\"children\":["
            + "{\"id\":\"childA\",\"width\":30,\"height\":30},"
            + "{\"id\":\"childB\",\"width\":30,\"height\":30}],"
            + "\"edges\":[{\"id\":\"e1\",\"sources\":[\"childA\"],\"targets\":[\"childB\"]}]"
            + "}]}";

    private static final String ISSUE_680_JSON = "{"
            + "\"id\":\"root\","
            + "\"layoutOptions\":{"
            + "\"org.eclipse.elk.algorithm\":\"org.eclipse.elk.layered\","
            + "\"org.eclipse.elk.direction\":\"DOWN\","
            + "\"org.eclipse.elk.edgeRouting\":\"ORTHOGONAL\"},"
            + "\"children\":[{"
            + "\"id\":\"parent\",\"width\":180,\"height\":110,"
            + "\"ports\":["
            + "{\"id\":\"p1\",\"width\":10,\"height\":10,"
            + "\"layoutOptions\":{\"org.eclipse.elk.port.side\":\"WEST\",\"org.eclipse.elk.port.borderOffset\":-20}},"
            + "{\"id\":\"p2\",\"width\":10,\"height\":10,"
            + "\"layoutOptions\":{\"org.eclipse.elk.port.side\":\"EAST\",\"org.eclipse.elk.port.borderOffset\":-22}}"
            + "],"
            + "\"children\":[{"
            + "\"id\":\"child\",\"width\":100,\"height\":60,"
            + "\"ports\":["
            + "{\"id\":\"c1\",\"width\":10,\"height\":10,"
            + "\"layoutOptions\":{\"org.eclipse.elk.port.side\":\"WEST\",\"org.eclipse.elk.port.borderOffset\":-8}},"
            + "{\"id\":\"c2\",\"width\":10,\"height\":10,"
            + "\"layoutOptions\":{\"org.eclipse.elk.port.side\":\"EAST\",\"org.eclipse.elk.port.borderOffset\":-8}}"
            + "]}],"
            + "\"edges\":["
            + "{\"id\":\"e1\",\"sources\":[\"p1\"],\"targets\":[\"c1\"]},"
            + "{\"id\":\"e2\",\"sources\":[\"c2\"],\"targets\":[\"p2\"]}"
            + "]}]}";

    private static final String ISSUE_871_JSON = "{"
            + "\"id\":\"root\","
            + "\"layoutOptions\":{"
            + "\"org.eclipse.elk.algorithm\":\"org.eclipse.elk.layered\","
            + "\"org.eclipse.elk.direction\":\"RIGHT\","
            + "\"org.eclipse.elk.layered.cycleBreaking.strategy\":\"MODEL_ORDER\","
            + "\"org.eclipse.elk.layered.considerModelOrder.strategy\":\"PREFER_EDGES\","
            + "\"org.eclipse.elk.layered.crossingMinimization.strategy\":\"NONE\","
            + "\"org.eclipse.elk.layered.crossingMinimization.greedySwitch.type\":\"OFF\","
            + "\"org.eclipse.elk.layered.feedbackEdges\":true},"
            + "\"children\":["
            + "{\"id\":\"n1\",\"width\":30,\"height\":30},"
            + "{\"id\":\"n2\",\"width\":30,\"height\":30},"
            + "{\"id\":\"n3\",\"width\":30,\"height\":30},"
            + "{\"id\":\"n4\",\"width\":30,\"height\":30}],"
            + "\"edges\":["
            + "{\"id\":\"e1\",\"sources\":[\"n1\"],\"targets\":[\"n2\"]},"
            + "{\"id\":\"e2\",\"sources\":[\"n1\"],\"targets\":[\"n3\"]},"
            + "{\"id\":\"e3\",\"sources\":[\"n2\"],\"targets\":[\"n4\"]},"
            + "{\"id\":\"e4\",\"sources\":[\"n4\"],\"targets\":[\"n3\"]}"
            + "]}";

    private static final String ISSUE_905_JSON = "{"
            + "\"id\":\"root\","
            + "\"layoutOptions\":{"
            + "\"org.eclipse.elk.algorithm\":\"org.eclipse.elk.layered\","
            + "\"org.eclipse.elk.direction\":\"RIGHT\"},"
            + "\"children\":["
            + "{\"id\":\"source\",\"width\":30,\"height\":30},"
            + "{\"id\":\"target\",\"width\":30,\"height\":30}],"
            + "\"edges\":[{"
            + "\"id\":\"e1\",\"sources\":[\"source\"],\"targets\":[\"target\"],"
            + "\"labels\":["
            + "{\"text\":\"tail\",\"width\":16,\"height\":10,\"x\":5,\"y\":10,"
            + "\"layoutOptions\":{\"org.eclipse.elk.edgeLabels.placement\":\"TAIL\"}},"
            + "{\"text\":\"center\",\"width\":20,\"height\":10,\"x\":20,\"y\":80,"
            + "\"layoutOptions\":{\"org.eclipse.elk.edgeLabels.placement\":\"CENTER\"}},"
            + "{\"text\":\"head\",\"width\":16,\"height\":10,\"x\":35,\"y\":150,"
            + "\"layoutOptions\":{\"org.eclipse.elk.edgeLabels.placement\":\"HEAD\"}}"
            + "]}]}";
}
