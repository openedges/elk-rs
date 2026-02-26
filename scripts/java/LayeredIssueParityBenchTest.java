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
import org.eclipse.elk.core.math.ElkPadding;
import org.eclipse.elk.core.options.CoreOptions;
import org.eclipse.elk.core.options.Direction;
import org.eclipse.elk.core.options.EdgeLabelPlacement;
import org.eclipse.elk.core.options.EdgeRouting;
import org.eclipse.elk.core.options.NodeLabelPlacement;
import org.eclipse.elk.core.options.PortConstraints;
import org.eclipse.elk.core.options.PortLabelPlacement;
import org.eclipse.elk.core.options.PortSide;
import org.eclipse.elk.core.util.BasicProgressMonitor;
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

    private static final String DEFAULT_SCENARIOS = "issue_405,issue_603,issue_680,issue_871,issue_905";
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

        int executed = 0;
        for (String scenario : scenarios) {
            if (!isSupportedScenario(scenario)) {
                System.out.println("Skipping unknown scenario: " + scenario);
                continue;
            }
            runScenario(scenario, iterations, warmup, output);
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
            return true;
        default:
            return false;
        }
    }

    private static void runScenario(
            final String scenario,
            final int iterations,
            final int warmup,
            final Path output) throws IOException {

        final LayeredLayoutProvider provider = new LayeredLayoutProvider();

        for (int i = 0; i < warmup; i++) {
            provider.layout(buildScenario(scenario), new BasicProgressMonitor());
        }

        final long start = System.nanoTime();
        for (int i = 0; i < iterations; i++) {
            provider.layout(buildScenario(scenario), new BasicProgressMonitor());
        }
        final long elapsedNanos = Math.max(1L, System.nanoTime() - start);

        final double avgMs = elapsedNanos / (double) iterations / 1_000_000.0;
        final double scenariosPerSec = iterations / (elapsedNanos / 1_000_000_000.0);
        final long timestamp = Instant.now().getEpochSecond();

        final String line = String.format(
                Locale.ROOT,
                "%d,%s,%d,%d,%d,%.6f,%.2f%n",
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

    private static ElkNode newSizedNode(final ElkNode parent, final String id, final double width, final double height) {
        ElkNode node = ElkGraphUtil.createNode(parent);
        node.setIdentifier(id);
        node.setDimensions(width, height);
        return node;
    }

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
        graph.setProperty(CoreOptions.EDGE_LABELS_PLACEMENT, EdgeLabelPlacement.Center);

        ElkNode left = newSizedNode(graph, "left", 40.0, 30.0);
        ElkNode right = newSizedNode(graph, "right", 40.0, 30.0);
        ElkEdge edge = ElkGraphUtil.createSimpleEdge(left, right);

        ElkLabel tail = ElkGraphUtil.createLabel("tail", edge);
        tail.setProperty(LayeredOptions.EDGE_LABELS_PLACEMENT, EdgeLabelPlacement.Tail);
        tail.setLocation(5.0, 120.0);

        ElkLabel center = ElkGraphUtil.createLabel("center", edge);
        center.setProperty(LayeredOptions.EDGE_LABELS_PLACEMENT, EdgeLabelPlacement.Center);
        center.setLocation(30.0, 40.0);

        ElkLabel head = ElkGraphUtil.createLabel("head", edge);
        head.setProperty(LayeredOptions.EDGE_LABELS_PLACEMENT, EdgeLabelPlacement.Head);
        head.setLocation(45.0, 130.0);
        return graph;
    }
}
