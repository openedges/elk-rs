/*******************************************************************************
 * Copyright (c) 2026.
 *
 * This program and the accompanying materials are made available under the
 * terms of the Eclipse Public License 2.0 which is available at
 * http://www.eclipse.org/legal/epl-2.0.
 *
 * SPDX-License-Identifier: EPL-2.0
 *******************************************************************************/
package org.eclipse.elk.alg.layered.issues;

import static org.junit.Assert.assertTrue;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.nio.file.StandardOpenOption;
import java.time.Instant;
import java.util.EnumSet;
import java.util.LinkedHashSet;
import java.util.Locale;
import java.util.Set;
import java.util.stream.Collectors;
import java.util.stream.Stream;

import org.eclipse.elk.alg.layered.LayeredLayoutProvider;
import org.eclipse.elk.alg.layered.options.CrossingMinimizationStrategy;
import org.eclipse.elk.alg.layered.options.CycleBreakingStrategy;
import org.eclipse.elk.alg.layered.options.GreedySwitchType;
import org.eclipse.elk.alg.layered.options.LayeredOptions;
import org.eclipse.elk.alg.layered.options.OrderingStrategy;
import org.eclipse.elk.alg.test.PlainJavaInitialization;
import org.eclipse.elk.core.RecursiveGraphLayoutEngine;
import org.eclipse.elk.core.math.ElkPadding;
import org.eclipse.elk.core.options.CoreOptions;
import org.eclipse.elk.core.options.Direction;
import org.eclipse.elk.core.options.EdgeLabelPlacement;
import org.eclipse.elk.core.options.EdgeRouting;
import org.eclipse.elk.core.options.HierarchyHandling;
import org.eclipse.elk.core.options.NodeLabelPlacement;
import org.eclipse.elk.core.options.PortConstraints;
import org.eclipse.elk.core.options.PortLabelPlacement;
import org.eclipse.elk.core.options.PortSide;
import org.eclipse.elk.core.util.BasicProgressMonitor;
import org.eclipse.elk.core.util.NullElkProgressMonitor;
import org.eclipse.elk.graph.ElkEdge;
import org.eclipse.elk.graph.ElkLabel;
import org.eclipse.elk.graph.ElkNode;
import org.eclipse.elk.graph.ElkPort;
import org.eclipse.elk.graph.util.ElkGraphUtil;
import org.junit.BeforeClass;
import org.junit.Test;

/**
 * Java-side parity benchmark for layered issue scenarios.
 *
 * <p>This test is opt-in. It only runs when {@code -Delk.parity.run=true} is set.</p>
 */
public class LayeredIssueParityBenchTest {

    // Layered issue scenarios (parity pipeline, matched by the Rust
    // `perf_layered_issue_scenarios` binary) followed by the synthetic 6-way
    // benchmark scenarios.
    private static final String DEFAULT_SCENARIOS =
            "issue_405,issue_603,issue_680,issue_871,issue_905,"
            + "layered_small,layered_medium,layered_large,layered_xlarge,"
            + "force_medium,force_large,force_xlarge,"
            + "stress_medium,stress_large,stress_xlarge,"
            + "mrtree_medium,mrtree_large,mrtree_xlarge,"
            + "radial_medium,radial_large,radial_xlarge,"
            + "rectpacking_medium,rectpacking_large,rectpacking_xlarge,"
            + "routing_polyline,routing_orthogonal,routing_splines,"
            + "crossmin_layer_sweep,crossmin_none,"
            + "hierarchy_flat,hierarchy_nested";
    private static final int DEFAULT_ITERATIONS = 20;
    private static final int DEFAULT_WARMUP = 3;
    private static final String DEFAULT_OUTPUT = "parity/java_results_layered_issue_scenarios.csv";

    @BeforeClass
    public static void init() {
        PlainJavaInitialization.initializePlainJavaLayout();
    }

    @Test
    public void runLayeredIssuePerfBench() throws IOException {
        if (!Boolean.parseBoolean(System.getProperty("elk.parity.run", "false"))) {
            return;
        }

        final String scenariosArg = System.getProperty("elk.parity.scenarios", DEFAULT_SCENARIOS);
        final int iterations = parseIntProperty("elk.parity.iterations", DEFAULT_ITERATIONS);
        final int warmup = parseIntProperty("elk.parity.warmup", DEFAULT_WARMUP);
        final String outputPath = System.getProperty("elk.parity.output", DEFAULT_OUTPUT);

        final Path output = Paths.get(outputPath);
        if (output.getParent() != null) {
            Files.createDirectories(output.getParent());
        }

        final Set<String> scenarios = Stream.of(scenariosArg.split(","))
                .map(String::trim)
                .filter(s -> !s.isEmpty())
                .collect(Collectors.toCollection(LinkedHashSet::new));

        // CSV format: "legacy" (default) writes the 7-column rows the parity scripts and
        // baselines use (timestamp,scenario,iterations,warmup,elapsed_nanos,avg_ms,ops_per_sec,
        // identical to the Rust perf_layered_issue_scenarios CSV); "engine" adds a header and
        // an engine column for the 6-way benchmark (run_perf_benchmark.sh converts either).
        final boolean engineColumn = "engine".equals(System.getProperty("elk.parity.csvFormat", "legacy"));
        if (engineColumn) {
            Files.write(output,
                    "timestamp,engine,scenario,iterations,warmup,elapsed_nanos,avg_ms,ops_per_sec\n"
                            .getBytes(StandardCharsets.UTF_8),
                    StandardOpenOption.CREATE, StandardOpenOption.TRUNCATE_EXISTING);
        }

        int executed = 0;
        for (String scenario : scenarios) {
            if (!isSupportedScenario(scenario)) {
                System.out.println("Skipping unknown scenario: " + scenario);
                continue;
            }
            runScenario(scenario, iterations, warmup, output, engineColumn);
            executed++;
        }

        assertTrue("No supported scenarios were selected.", executed > 0);
    }

    private static int parseIntProperty(final String property, final int fallback) {
        final String value = System.getProperty(property);
        if (value == null || value.isBlank()) {
            return fallback;
        }
        try {
            return Math.max(1, Integer.parseInt(value));
        } catch (NumberFormatException exception) {
            return fallback;
        }
    }

    private static boolean isSupportedScenario(final String scenario) {
        switch (scenario) {
        case "issue_405":
        case "issue_603":
        case "issue_680":
        case "issue_871":
        case "issue_905":
        case "layered_small":
        case "layered_medium":
        case "layered_large":
        case "layered_xlarge":
        case "force_medium":
        case "force_large":
        case "force_xlarge":
        case "stress_medium":
        case "stress_large":
        case "stress_xlarge":
        case "mrtree_medium":
        case "mrtree_large":
        case "mrtree_xlarge":
        case "radial_medium":
        case "radial_large":
        case "radial_xlarge":
        case "rectpacking_medium":
        case "rectpacking_large":
        case "rectpacking_xlarge":
        case "routing_polyline":
        case "routing_orthogonal":
        case "routing_splines":
        case "crossmin_layer_sweep":
        case "crossmin_none":
        case "hierarchy_flat":
        case "hierarchy_nested":
            return true;
        default:
            return false;
        }
    }

    private static void runScenario(
            final String scenario,
            final int iterations,
            final int warmup,
            final Path output,
            final boolean engineColumn) throws IOException {

        // Issue scenarios time LayeredLayoutProvider directly, like the Rust
        // perf_layered_issue_scenarios binary; synthetic scenarios go through the
        // RecursiveGraphLayoutEngine like the other 6-way benchmark engines.
        final boolean issueScenario = scenario.startsWith("issue_");
        final LayeredLayoutProvider provider = issueScenario ? new LayeredLayoutProvider() : null;
        final RecursiveGraphLayoutEngine engine = issueScenario ? null : new RecursiveGraphLayoutEngine();

        for (int i = 0; i < warmup; i++) {
            layoutOnce(provider, engine, buildScenario(scenario));
        }

        final long start = System.nanoTime();
        for (int i = 0; i < iterations; i++) {
            layoutOnce(provider, engine, buildScenario(scenario));
        }
        final long elapsedNanos = Math.max(1L, System.nanoTime() - start);

        final double avgMs = elapsedNanos / (double) iterations / 1_000_000.0;
        final double scenariosPerSec = iterations / (elapsedNanos / 1_000_000_000.0);
        final long timestamp = Instant.now().getEpochSecond();

        final String line = String.format(
                Locale.ROOT,
                engineColumn ? "%d,java,%s,%d,%d,%d,%.6f,%.2f%n" : "%d,%s,%d,%d,%d,%.6f,%.2f%n",
                timestamp,
                scenario,
                iterations,
                warmup,
                elapsedNanos,
                avgMs,
                scenariosPerSec);
        Files.write(output, line.getBytes(StandardCharsets.UTF_8),
                StandardOpenOption.CREATE, StandardOpenOption.APPEND);
    }

    private static void layoutOnce(
            final LayeredLayoutProvider provider,
            final RecursiveGraphLayoutEngine engine,
            final ElkNode graph) {
        if (provider != null) {
            provider.layout(graph, new BasicProgressMonitor());
        } else {
            engine.layout(graph, new NullElkProgressMonitor());
        }
    }

    private static ElkNode buildScenario(final String scenario) {
        switch (scenario) {
        case "issue_405":
            return buildIssue405Scenario();
        case "issue_603":
            return buildIssue603Scenario();
        case "issue_680":
            return buildIssue680Scenario();
        case "issue_871":
            return buildIssue871Scenario();
        case "issue_905":
            return buildIssue905Scenario();
        case "layered_small":
            return buildLayeredDagScenario(10, 15, 42);
        case "layered_medium":
            return buildLayeredDagScenario(50, 100, 42);
        case "layered_large":
            return buildLayeredDagScenario(200, 500, 42);
        case "layered_xlarge":
            return buildLayeredDagScenario(1000, 3000, 42);
        case "force_medium":
            return buildGeneralGraphScenario(50, 80, 100, "org.eclipse.elk.force");
        case "force_large":
            return buildGeneralGraphScenario(200, 400, 100, "org.eclipse.elk.force");
        case "force_xlarge":
            return buildGeneralGraphScenario(500, 1200, 100, "org.eclipse.elk.force");
        case "stress_medium":
            return buildGeneralGraphScenario(50, 80, 100, "org.eclipse.elk.stress");
        case "stress_large":
            return buildGeneralGraphScenario(200, 400, 100, "org.eclipse.elk.stress");
        case "stress_xlarge":
            return buildGeneralGraphScenario(500, 1200, 100, "org.eclipse.elk.stress");
        case "mrtree_medium":
            return buildTreeScenario(50, 200, "org.eclipse.elk.mrtree");
        case "mrtree_large":
            return buildTreeScenario(200, 200, "org.eclipse.elk.mrtree");
        case "mrtree_xlarge":
            return buildTreeScenario(1000, 200, "org.eclipse.elk.mrtree");
        case "radial_medium":
            return buildTreeScenario(50, 200, "org.eclipse.elk.radial");
        case "radial_large":
            return buildTreeScenario(200, 200, "org.eclipse.elk.radial");
        case "radial_xlarge":
            return buildTreeScenario(1000, 200, "org.eclipse.elk.radial");
        case "rectpacking_medium":
            return buildRectpackingScenario(50, 100);
        case "rectpacking_large":
            return buildRectpackingScenario(200, 100);
        case "rectpacking_xlarge":
            return buildRectpackingScenario(1000, 100);
        case "routing_polyline":
            return buildRoutingScenario(50, 100, 42, EdgeRouting.POLYLINE);
        case "routing_orthogonal":
            return buildRoutingScenario(50, 100, 42, EdgeRouting.ORTHOGONAL);
        case "routing_splines":
            return buildRoutingScenario(50, 100, 42, EdgeRouting.SPLINES);
        case "crossmin_layer_sweep":
            return buildCrossminScenario(50, 100, 42, true);
        case "crossmin_none":
            return buildCrossminScenario(50, 100, 42, false);
        case "hierarchy_flat":
            return buildLayeredDagScenario(30, 50, 300);
        case "hierarchy_nested":
            return buildHierarchyNestedScenario(300);
        default:
            throw new IllegalArgumentException("Unsupported scenario: " + scenario);
        }
    }

    private static ElkNode baseGraph() {
        ElkNode graph = ElkGraphUtil.createGraph();
        graph.setProperty(CoreOptions.ALGORITHM, LayeredOptions.ALGORITHM_ID);
        graph.setProperty(CoreOptions.DIRECTION, Direction.RIGHT);
        graph.setProperty(CoreOptions.EDGE_ROUTING, EdgeRouting.ORTHOGONAL);
        return graph;
    }

    private static ElkNode graphWithAlgorithm(final String algorithm) {
        ElkNode graph = ElkGraphUtil.createGraph();
        graph.setProperty(CoreOptions.ALGORITHM, algorithm);
        return graph;
    }

    private static ElkNode newSizedNode(final ElkNode parent, final String id, final double width, final double height) {
        ElkNode node = ElkGraphUtil.createNode(parent);
        node.setIdentifier(id);
        node.setDimensions(width, height);
        return node;
    }

    // ---- layered issue scenarios (parity pipeline) ----

    private static ElkPort newPort(final ElkNode parent, final String id, final PortSide side) {
        ElkPort port = ElkGraphUtil.createPort(parent);
        port.setIdentifier(id);
        port.setDimensions(8.0, 8.0);
        port.setProperty(CoreOptions.PORT_SIDE, side);
        return port;
    }

    private static ElkNode buildIssue405Scenario() {
        ElkNode graph = baseGraph();
        ElkNode reference = newSizedNode(graph, "reference", 90.0, 70.0);
        reference.setProperty(CoreOptions.PORT_CONSTRAINTS, PortConstraints.FIXED_SIDE);

        EnumSet<PortLabelPlacement> placement = PortLabelPlacement.outside();
        placement.add(PortLabelPlacement.NEXT_TO_PORT_IF_POSSIBLE);
        reference.setProperty(CoreOptions.PORT_LABELS_PLACEMENT, placement);

        ElkPort west = newPort(reference, "west", PortSide.WEST);
        ElkPort east = newPort(reference, "east", PortSide.EAST);
        ElkPort north = newPort(reference, "north", PortSide.NORTH);
        ElkPort south = newPort(reference, "south", PortSide.SOUTH);
        ElkGraphUtil.createLabel("west", west);
        ElkGraphUtil.createLabel("east", east);
        ElkGraphUtil.createLabel("north", north);
        ElkGraphUtil.createLabel("south", south);

        ElkNode westNode = newSizedNode(graph, "westNode", 30.0, 20.0);
        ElkNode eastNode = newSizedNode(graph, "eastNode", 30.0, 20.0);
        ElkNode northNode = newSizedNode(graph, "northNode", 30.0, 20.0);
        ElkNode southNode = newSizedNode(graph, "southNode", 30.0, 20.0);

        ElkPort westNodePort = newPort(westNode, "westNodePort", PortSide.EAST);
        ElkPort eastNodePort = newPort(eastNode, "eastNodePort", PortSide.WEST);
        ElkPort northNodePort = newPort(northNode, "northNodePort", PortSide.SOUTH);
        ElkPort southNodePort = newPort(southNode, "southNodePort", PortSide.NORTH);

        ElkGraphUtil.createSimpleEdge(westNodePort, west);
        ElkGraphUtil.createSimpleEdge(east, eastNodePort);
        ElkGraphUtil.createSimpleEdge(northNodePort, north);
        ElkGraphUtil.createSimpleEdge(south, southNodePort);
        return graph;
    }

    private static ElkNode buildIssue603Scenario() {
        ElkNode graph = baseGraph();
        graph.setProperty(CoreOptions.DIRECTION, Direction.DOWN);

        ElkNode compound = newSizedNode(graph, "compound", 180.0, 140.0);
        compound.setProperty(CoreOptions.NODE_LABELS_PLACEMENT, NodeLabelPlacement.insideTopCenter());
        ElkGraphUtil.createLabel("compound", compound);

        ElkNode a = newSizedNode(compound, "a", 36.0, 24.0);
        ElkNode b = newSizedNode(compound, "b", 36.0, 24.0);
        ElkNode c = newSizedNode(compound, "c", 36.0, 24.0);
        ElkNode d = newSizedNode(compound, "d", 36.0, 24.0);
        ElkNode e = newSizedNode(compound, "e", 36.0, 24.0);
        ElkGraphUtil.createSimpleEdge(a, c);
        ElkGraphUtil.createSimpleEdge(b, c);
        ElkGraphUtil.createSimpleEdge(c, d);
        ElkGraphUtil.createSimpleEdge(c, e);
        return graph;
    }

    private static ElkNode buildIssue680Scenario() {
        ElkNode graph = baseGraph();
        graph.setProperty(CoreOptions.DIRECTION, Direction.DOWN);

        ElkNode parent = newSizedNode(graph, "parent", 120.0, 120.0);
        parent.setProperty(CoreOptions.ALGORITHM, LayeredOptions.ALGORITHM_ID);
        parent.setProperty(CoreOptions.DIRECTION, Direction.DOWN);
        parent.setProperty(CoreOptions.EDGE_ROUTING, EdgeRouting.ORTHOGONAL);

        ElkPort parentTop = newPort(parent, "parentTop", PortSide.NORTH);
        parentTop.setProperty(LayeredOptions.PORT_BORDER_OFFSET, -20.0);
        ElkPort parentBottom = newPort(parent, "parentBottom", PortSide.SOUTH);
        parentBottom.setProperty(LayeredOptions.PORT_BORDER_OFFSET, -18.0);

        ElkNode child = newSizedNode(parent, "child", 52.0, 78.0);
        ElkPort childTop = newPort(child, "childTop", PortSide.NORTH);
        childTop.setProperty(LayeredOptions.PORT_BORDER_OFFSET, -8.0);
        ElkPort childBottom = newPort(child, "childBottom", PortSide.SOUTH);
        childBottom.setProperty(LayeredOptions.PORT_BORDER_OFFSET, -8.0);

        ElkGraphUtil.createSimpleEdge(parentTop, childTop);
        ElkGraphUtil.createSimpleEdge(childBottom, parentBottom);
        return graph;
    }

    private static ElkNode buildIssue871Scenario() {
        ElkNode graph = baseGraph();
        graph.setProperty(LayeredOptions.CYCLE_BREAKING_STRATEGY, CycleBreakingStrategy.MODEL_ORDER);
        graph.setProperty(LayeredOptions.CONSIDER_MODEL_ORDER_STRATEGY, OrderingStrategy.PREFER_EDGES);
        graph.setProperty(LayeredOptions.CROSSING_MINIMIZATION_STRATEGY, CrossingMinimizationStrategy.NONE);
        graph.setProperty(LayeredOptions.CROSSING_MINIMIZATION_GREEDY_SWITCH_TYPE, GreedySwitchType.OFF);
        graph.setProperty(CoreOptions.PADDING, new ElkPadding(0.0));
        graph.setProperty(CoreOptions.SPACING_NODE_NODE, 10.0);
        graph.setProperty(LayeredOptions.SPACING_NODE_NODE_BETWEEN_LAYERS, 20.0);
        graph.setProperty(LayeredOptions.FEEDBACK_EDGES, true);

        ElkNode n1 = newSizedNode(graph, "n1", 30.0, 30.0);
        ElkNode n2 = newSizedNode(graph, "n2", 30.0, 30.0);
        ElkNode n3 = newSizedNode(graph, "n3", 30.0, 30.0);
        ElkNode n4 = newSizedNode(graph, "n4", 30.0, 30.0);
        ElkGraphUtil.createSimpleEdge(n1, n2);
        ElkGraphUtil.createSimpleEdge(n1, n3);
        ElkGraphUtil.createSimpleEdge(n2, n4);
        ElkGraphUtil.createSimpleEdge(n4, n3);
        return graph;
    }

    private static ElkNode buildIssue905Scenario() {
        ElkNode graph = baseGraph();
        graph.setProperty(CoreOptions.EDGE_LABELS_PLACEMENT, EdgeLabelPlacement.CENTER);

        ElkNode left = newSizedNode(graph, "left", 40.0, 30.0);
        ElkNode right = newSizedNode(graph, "right", 40.0, 30.0);
        ElkEdge edge = ElkGraphUtil.createSimpleEdge(left, right);

        ElkLabel tail = ElkGraphUtil.createLabel("tail", edge);
        tail.setProperty(LayeredOptions.EDGE_LABELS_PLACEMENT, EdgeLabelPlacement.TAIL);
        tail.setLocation(5.0, 120.0);

        ElkLabel center = ElkGraphUtil.createLabel("center", edge);
        center.setProperty(LayeredOptions.EDGE_LABELS_PLACEMENT, EdgeLabelPlacement.CENTER);
        center.setLocation(30.0, 40.0);

        ElkLabel head = ElkGraphUtil.createLabel("head", edge);
        head.setProperty(LayeredOptions.EDGE_LABELS_PLACEMENT, EdgeLabelPlacement.HEAD);
        head.setLocation(45.0, 130.0);
        return graph;
    }

    // ---- synthetic 6-way benchmark scenarios ----

    private static int lcg(int state) {
        return (int)((((long)state * 1103515245L + 12345L)) & 0x7fffffffL);
    }

    private static ElkNode buildLayeredDagScenario(int nodes, int edges, int seed) {
        ElkNode graph = baseGraph();

        ElkNode[] nodeArray = new ElkNode[nodes];
        for (int i = 0; i < nodes; i++) {
            nodeArray[i] = newSizedNode(graph, "n" + i, 40.0, 30.0);
        }

        int state = seed;
        int created = 0;
        int attempts = 0;
        int maxAttempts = edges * 100;
        while (created < edges && attempts < maxAttempts) {
            state = lcg(state);
            int src = state % nodes;
            if (src < 0) src += nodes;
            state = lcg(state);
            int tgt = state % nodes;
            if (tgt < 0) tgt += nodes;
            attempts++;

            int layerSrc = src * 5 / nodes;
            int layerTgt = tgt * 5 / nodes;
            if (layerSrc >= layerTgt) {
                continue;
            }

            ElkGraphUtil.createSimpleEdge(nodeArray[src], nodeArray[tgt]);
            created++;
        }

        return graph;
    }

    private static ElkNode buildGeneralGraphScenario(int nodes, int edges, int seed, String algorithm) {
        ElkNode graph = graphWithAlgorithm(algorithm);

        ElkNode[] nodeArray = new ElkNode[nodes];
        for (int i = 0; i < nodes; i++) {
            nodeArray[i] = newSizedNode(graph, "n" + i, 40.0, 30.0);
        }

        int state = seed;
        for (int i = 0; i < edges; i++) {
            state = lcg(state);
            int src = state % nodes;
            if (src < 0) src += nodes;
            state = lcg(state);
            int tgt = state % nodes;
            if (tgt < 0) tgt += nodes;
            ElkGraphUtil.createSimpleEdge(nodeArray[src], nodeArray[tgt]);
        }

        return graph;
    }

    private static ElkNode buildTreeScenario(int nodes, int seed, String algorithm) {
        ElkNode graph = graphWithAlgorithm(algorithm);

        ElkNode[] nodeArray = new ElkNode[nodes];
        nodeArray[0] = newSizedNode(graph, "n0", 40.0, 30.0);

        int state = seed;
        for (int i = 1; i < nodes; i++) {
            nodeArray[i] = newSizedNode(graph, "n" + i, 40.0, 30.0);
            state = lcg(state);
            int parent = state % i;
            if (parent < 0) parent += i;
            ElkGraphUtil.createSimpleEdge(nodeArray[parent], nodeArray[i]);
        }

        return graph;
    }

    private static ElkNode buildRectpackingScenario(int nodes, int seed) {
        ElkNode graph = graphWithAlgorithm("org.eclipse.elk.rectpacking");

        int state = seed;
        for (int i = 0; i < nodes; i++) {
            state = lcg(state);
            double width = 20.0 + (state % 61);
            if (width < 20.0) width += 61.0;
            state = lcg(state);
            double height = 20.0 + (state % 61);
            if (height < 20.0) height += 61.0;
            newSizedNode(graph, "n" + i, width, height);
        }

        return graph;
    }

    private static ElkNode buildRoutingScenario(int nodes, int edges, int seed, EdgeRouting routing) {
        ElkNode graph = baseGraph();
        graph.setProperty(CoreOptions.EDGE_ROUTING, routing);

        ElkNode[] nodeArray = new ElkNode[nodes];
        for (int i = 0; i < nodes; i++) {
            nodeArray[i] = newSizedNode(graph, "n" + i, 40.0, 30.0);
        }

        int state = seed;
        int created = 0;
        int attempts = 0;
        int maxAttempts = edges * 100;
        while (created < edges && attempts < maxAttempts) {
            state = lcg(state);
            int src = state % nodes;
            if (src < 0) src += nodes;
            state = lcg(state);
            int tgt = state % nodes;
            if (tgt < 0) tgt += nodes;
            attempts++;

            int layerSrc = src * 5 / nodes;
            int layerTgt = tgt * 5 / nodes;
            if (layerSrc >= layerTgt) {
                continue;
            }

            ElkGraphUtil.createSimpleEdge(nodeArray[src], nodeArray[tgt]);
            created++;
        }

        return graph;
    }

    private static ElkNode buildCrossminScenario(int nodes, int edges, int seed, boolean layerSweep) {
        ElkNode graph = baseGraph();

        if (layerSweep) {
            graph.setProperty(LayeredOptions.CROSSING_MINIMIZATION_STRATEGY,
                    CrossingMinimizationStrategy.LAYER_SWEEP);
            graph.setProperty(LayeredOptions.CROSSING_MINIMIZATION_GREEDY_SWITCH_TYPE,
                    GreedySwitchType.TWO_SIDED);
        } else {
            graph.setProperty(LayeredOptions.CROSSING_MINIMIZATION_STRATEGY,
                    CrossingMinimizationStrategy.NONE);
            graph.setProperty(LayeredOptions.CROSSING_MINIMIZATION_GREEDY_SWITCH_TYPE,
                    GreedySwitchType.OFF);
        }

        ElkNode[] nodeArray = new ElkNode[nodes];
        for (int i = 0; i < nodes; i++) {
            nodeArray[i] = newSizedNode(graph, "n" + i, 40.0, 30.0);
        }

        int state = seed;
        int created = 0;
        int attempts = 0;
        int maxAttempts = edges * 100;
        while (created < edges && attempts < maxAttempts) {
            state = lcg(state);
            int src = state % nodes;
            if (src < 0) src += nodes;
            state = lcg(state);
            int tgt = state % nodes;
            if (tgt < 0) tgt += nodes;
            attempts++;

            int layerSrc = src * 5 / nodes;
            int layerTgt = tgt * 5 / nodes;
            if (layerSrc >= layerTgt) {
                continue;
            }

            ElkGraphUtil.createSimpleEdge(nodeArray[src], nodeArray[tgt]);
            created++;
        }

        return graph;
    }

    private static ElkNode buildHierarchyNestedScenario(int seed) {
        ElkNode graph = ElkGraphUtil.createGraph();
        graph.setProperty(CoreOptions.ALGORITHM, LayeredOptions.ALGORITHM_ID);
        graph.setProperty(CoreOptions.DIRECTION, Direction.RIGHT);
        graph.setProperty(CoreOptions.EDGE_ROUTING, EdgeRouting.ORTHOGONAL);
        graph.setProperty(CoreOptions.HIERARCHY_HANDLING, HierarchyHandling.INCLUDE_CHILDREN);

        int state = seed;

        // 3 compound children, each with 9 leaves
        ElkNode[][] allLeaves = new ElkNode[3][9];
        for (int c = 0; c < 3; c++) {
            ElkNode compound = ElkGraphUtil.createNode(graph);
            compound.setIdentifier("compound" + c);
            compound.setDimensions(0.0, 0.0);

            for (int l = 0; l < 9; l++) {
                allLeaves[c][l] = newSizedNode(compound, "c" + c + "_l" + l, 40.0, 30.0);
            }

            // Tree pattern: leaf i connects to random parent in [0, i)
            for (int i = 1; i < 9; i++) {
                state = lcg(state);
                int parent = state % i;
                if (parent < 0) parent += i;
                ElkGraphUtil.createSimpleEdge(allLeaves[c][parent], allLeaves[c][i]);
            }
        }

        // Cross-compound: leaf[c][0] -> leaf[c+1][0]
        for (int c = 0; c < 2; c++) {
            ElkGraphUtil.createSimpleEdge(allLeaves[c][0], allLeaves[c + 1][0]);
        }

        return graph;
    }
}
