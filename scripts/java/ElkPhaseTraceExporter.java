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

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.Collections;
import java.util.Comparator;
import java.util.HashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.stream.Collectors;
import java.util.stream.Stream;

import org.eclipse.elk.alg.layered.graph.LEdge;
import org.eclipse.elk.alg.layered.graph.LGraph;
import org.eclipse.elk.alg.layered.graph.LLabel;
import org.eclipse.elk.alg.layered.graph.LNode;
import org.eclipse.elk.alg.layered.graph.LPort;
import org.eclipse.elk.alg.layered.graph.Layer;
import org.eclipse.elk.alg.layered.options.InternalProperties;
import org.eclipse.elk.alg.layered.options.LayeredMetaDataProvider;
import org.eclipse.elk.alg.layered.options.LayeredOptions;
import org.eclipse.elk.core.RecursiveGraphLayoutEngine;
import org.eclipse.elk.core.data.LayoutMetaDataService;
import org.eclipse.elk.core.math.KVector;
import org.eclipse.elk.core.math.KVectorChain;
import org.eclipse.elk.core.options.CoreOptions;
import org.eclipse.elk.core.testing.TestController;
import org.eclipse.elk.core.testing.TestController.ILayoutExecutionListener;
import org.eclipse.elk.core.util.BasicProgressMonitor;
import org.eclipse.elk.graph.ElkNode;
import org.eclipse.elk.graph.json.ElkGraphJson;
import org.eclipse.emf.common.util.URI;
import org.eclipse.emf.ecore.EObject;
import org.eclipse.emf.ecore.resource.Resource;
import org.eclipse.emf.ecore.resource.ResourceSet;
import org.eclipse.emf.ecore.resource.impl.ResourceSetImpl;

/**
 * Java-side phase trace exporter for ELK-vs-elk-rs layout parity debugging.
 *
 * <p>Captures the internal LGraph state after each processor step during
 * the layered layout algorithm and writes JSON snapshots to disk.</p>
 *
 * <p>Standalone program -- compiled and run with plain javac/java to bypass
 * Tycho/OSGi package visibility restrictions on internal ELK classes.</p>
 *
 * <p>System properties:</p>
 * <ul>
 *   <li>{@code elk.trace.modelsRoot} - path to model directory</li>
 *   <li>{@code elk.trace.outputDir} - path to output directory</li>
 *   <li>{@code elk.trace.limit} - max models to process (0 = unlimited)</li>
 *   <li>{@code elk.trace.include} - CSV include tokens</li>
 *   <li>{@code elk.trace.exclude} - CSV exclude tokens</li>
 *   <li>{@code elk.trace.randomSeed} - random seed (default 1)</li>
 *   <li>{@code elk.trace.prettyPrint} - pretty-print JSON (default true)</li>
 * </ul>
 */
@SuppressWarnings("restriction")
public class ElkPhaseTraceExporter {

    private static final String DEFAULT_MODELS_ROOT = "external/elk-models";
    private static final String DEFAULT_OUTPUT_DIR = "perf/model_parity/java_trace";

    public static void main(String[] args) throws IOException {
        // Register ELK layout providers directly (without Xtext/Guice from PlainJavaInitialization)
        LayoutMetaDataService.getInstance().registerLayoutMetaDataProviders(
                new CoreOptions(), new LayeredMetaDataProvider());

        final Path modelsRoot = Paths.get(
                System.getProperty("elk.trace.modelsRoot", DEFAULT_MODELS_ROOT))
                .toAbsolutePath()
                .normalize();
        final Path outputDir = Paths.get(
                System.getProperty("elk.trace.outputDir", DEFAULT_OUTPUT_DIR))
                .toAbsolutePath()
                .normalize();

        final int limit = parseIntProperty("elk.trace.limit", 0);
        final int randomSeed = parseIntProperty("elk.trace.randomSeed", 1);
        final boolean prettyPrint = Boolean.parseBoolean(
                System.getProperty("elk.trace.prettyPrint", "true"));
        final List<String> includeTokens = parseCsvTokens(
                System.getProperty("elk.trace.include", ""));
        final List<String> excludeTokens = parseCsvTokens(
                System.getProperty("elk.trace.exclude", ""));

        if (!Files.exists(modelsRoot) || !Files.isDirectory(modelsRoot)) {
            System.err.println("models root does not exist or is not a directory: " + modelsRoot);
            System.exit(1);
        }

        deleteRecursively(outputDir);
        Files.createDirectories(outputDir);

        final List<Path> modelFiles = collectModelFiles(
                modelsRoot, includeTokens, excludeTokens, limit);

        if (modelFiles.isEmpty()) {
            System.err.println("No model files found under: " + modelsRoot);
            System.exit(1);
        }

        int successCount = 0;
        int failureCount = 0;

        for (Path modelFile : modelFiles) {
            final String relPath = toUnixPath(modelsRoot.relativize(modelFile));

            try {
                ElkNode loadedGraph = loadGraph(modelFile);
                loadedGraph.setProperty(CoreOptions.RANDOM_SEED, randomSeed);

                // Round-trip through JSON (same as ElkModelParityExportTest)
                String inputJson = ElkGraphJson.forGraph(loadedGraph)
                        .prettyPrint(false)
                        .shortLayoutOptionKeys(false)
                        .omitZeroPositions(false)
                        .omitZeroDimension(false)
                        .omitLayout(false)
                        .omitUnknownLayoutOptions(false)
                        .toJson();
                ElkNode reimportedGraph = ElkGraphJson.forGraph(inputJson).toElk();

                // Build output directory for this model
                Path modelTraceDir = outputDir.resolve(relPath);
                Files.createDirectories(modelTraceDir);

                // Use recursive layout + TestController so hierarchy/includeChildren models
                // are traced as well. For phase-gate parity we force layered explicitly.
                reimportedGraph.setProperty(CoreOptions.ALGORITHM, LayeredOptions.ALGORITHM_ID);

                TraceCaptureListener traceListener = new TraceCaptureListener(modelTraceDir, prettyPrint);
                TestController testController = new TestController(LayeredOptions.ALGORITHM_ID);
                testController.addLayoutExecutionListener(traceListener);
                try {
                    RecursiveGraphLayoutEngine engine = new RecursiveGraphLayoutEngine();
                    engine.layout(reimportedGraph, testController, new BasicProgressMonitor());
                } finally {
                    testController.uninstall();
                }

                // Write an index file listing all processors in observed order.
                traceListener.writeIndex();

                successCount++;

            } catch (Throwable throwable) {
                failureCount++;
                Throwable rootCause = rootCause(throwable);
                System.err.println(String.format(
                        Locale.ROOT,
                        "ERROR tracing model %s: %s",
                        relPath,
                        rootCause.toString()));
                rootCause.printStackTrace(System.err);
            }
        }

        System.out.println(String.format(
                Locale.ROOT,
                "ELK phase trace export completed: total=%d, success=%d, failed=%d, output=%s",
                modelFiles.size(),
                successCount,
                failureCount,
                outputDir));
    }

    // ========================== Snapshot I/O ==========================

    private static void writeSnapshot(
            final Path dir,
            final String fileName,
            final String content) throws IOException {

        Path path = dir.resolve(fileName);
        Files.write(path, content.getBytes(StandardCharsets.UTF_8));
    }

    private static void writeIndex(
            final List<String> processors,
            final Path modelTraceDir,
            final boolean prettyPrint) throws IOException {

        JsonBuilder b = new JsonBuilder(prettyPrint);
        b.beginObject();
        b.key("count"); b.value(processors.size());
        b.key("steps"); b.beginArray();

        for (int i = 0; i < processors.size(); i++) {
            String name = processors.get(i);
            b.beginObject();
            b.key("step"); b.value(i);
            b.key("processor"); b.value(name);
            b.key("file"); b.value(String.format(
                    Locale.ROOT, "step_%03d_%s.json", i, name));
            b.endObject();
        }
        b.endArray();
        b.endObject();

        Path indexPath = modelTraceDir.resolve("index.json");
        Files.write(indexPath, b.toString().getBytes(StandardCharsets.UTF_8));
    }

    private static Throwable rootCause(final Throwable throwable) {
        Throwable cursor = throwable;
        while (cursor.getCause() != null && cursor.getCause() != cursor) {
            cursor = cursor.getCause();
        }
        return cursor;
    }

    private static final class TraceCaptureListener implements ILayoutExecutionListener {
        private final Path modelTraceDir;
        private final boolean prettyPrint;
        private final List<String> processors = new ArrayList<>();
        private int step = 0;

        TraceCaptureListener(final Path modelTraceDir, final boolean prettyPrint) {
            this.modelTraceDir = modelTraceDir;
            this.prettyPrint = prettyPrint;
        }

        @Override
        public void layoutProcessorReady(
                final org.eclipse.elk.core.alg.ILayoutProcessor<?> processor,
                final Object graph,
                final boolean isRoot) {
            // Snapshot is captured after each processor, matching Rust semantics.
        }

        @Override
        public void layoutProcessorFinished(
                final org.eclipse.elk.core.alg.ILayoutProcessor<?> processor,
                final Object graph,
                final boolean isRoot) {
            if (!(graph instanceof LGraph)) {
                return;
            }

            String processorName = processor.getClass().getSimpleName();
            processors.add(processorName);

            String snapshot = buildSnapshot(
                    step, processorName, Collections.singletonList((LGraph) graph), prettyPrint);
            String fileName = String.format(
                    Locale.ROOT, "step_%03d_%s.json", step, processorName);
            try {
                writeSnapshot(modelTraceDir, fileName, snapshot);
            } catch (IOException exception) {
                throw new RuntimeException(
                        "failed to write Java trace snapshot for step " + step + ": " + fileName,
                        exception);
            }

            step++;
        }

        void writeIndex() throws IOException {
            ElkPhaseTraceExporter.writeIndex(processors, modelTraceDir, prettyPrint);
        }
    }

    // ========================== LGraph Snapshot Serialization ==========================

    /**
     * Builds a JSON snapshot of all component graphs at the current step.
     */
    private static String buildSnapshot(
            final int step,
            final String processorName,
            final List<LGraph> graphs,
            final boolean prettyPrint) {

        JsonBuilder b = new JsonBuilder(prettyPrint);
        b.beginObject();
        b.key("step"); b.value(step);
        b.key("processor"); b.value(processorName);

        if (graphs.size() == 1) {
            // Single component: inline the graph fields directly
            serializeGraphFields(b, graphs.get(0));
        } else {
            // Multiple components: wrap in a "components" array
            b.key("components"); b.beginArray();
            for (int i = 0; i < graphs.size(); i++) {
                b.beginObject();
                b.key("component"); b.value(i);
                serializeGraphFields(b, graphs.get(i));
                b.endObject();
            }
            b.endArray();
        }

        b.endObject();
        return b.toString();
    }

    /**
     * Serializes the LGraph's nodes, edges, layers, size, and padding into the
     * given JSON builder.
     */
    private static void serializeGraphFields(final JsonBuilder b, final LGraph lgraph) {
        // Assign stable IDs to all nodes and ports
        Map<LNode, String> nodeIds = new HashMap<>();
        Map<LPort, String> portIds = new HashMap<>();
        assignIds(lgraph, nodeIds, portIds);

        // Nodes
        b.key("nodes"); b.beginArray();
        for (LNode node : getAllNodes(lgraph)) {
            serializeNode(b, node, nodeIds, portIds);
        }
        b.endArray();

        // Edges (collected from all nodes' outgoing ports)
        b.key("edges"); b.beginArray();
        int edgeCounter = 0;
        for (LNode node : getAllNodes(lgraph)) {
            for (LPort port : node.getPorts()) {
                for (LEdge edge : port.getOutgoingEdges()) {
                    serializeEdge(b, edge, edgeCounter++, nodeIds, portIds);
                }
            }
        }
        b.endArray();

        // Layers (each layer is an array of node IDs)
        b.key("layers"); b.beginArray();
        for (Layer layer : lgraph.getLayers()) {
            b.beginArray();
            for (LNode node : layer.getNodes()) {
                b.value(nodeIds.getOrDefault(node, node.toString()));
            }
            b.endArray();
        }
        b.endArray();

        // Graph actual size (size + padding)
        KVector actualSize = lgraph.getActualSize();
        b.key("graphSize"); b.beginObject();
        b.key("width"); b.value(actualSize.x);
        b.key("height"); b.value(actualSize.y);
        b.endObject();

        // Graph inner size (without padding)
        KVector innerSize = lgraph.getSize();
        b.key("size"); b.beginObject();
        b.key("width"); b.value(innerSize.x);
        b.key("height"); b.value(innerSize.y);
        b.endObject();

        // Offset
        KVector offset = lgraph.getOffset();
        b.key("offset"); b.beginObject();
        b.key("x"); b.value(offset.x);
        b.key("y"); b.value(offset.y);
        b.endObject();

        // Padding
        b.key("padding"); b.beginObject();
        b.key("top"); b.value(lgraph.getPadding().top);
        b.key("bottom"); b.value(lgraph.getPadding().bottom);
        b.key("left"); b.value(lgraph.getPadding().left);
        b.key("right"); b.value(lgraph.getPadding().right);
        b.endObject();
    }

    /**
     * Returns all nodes in the graph: layered nodes first (in layer order),
     * then layerless nodes.
     */
    private static List<LNode> getAllNodes(final LGraph lgraph) {
        List<LNode> all = new ArrayList<>();
        for (Layer layer : lgraph.getLayers()) {
            all.addAll(layer.getNodes());
        }
        all.addAll(lgraph.getLayerlessNodes());
        return all;
    }

    /**
     * Assigns stable string IDs to every node and port in the graph.
     * Prefers ElkNode/ElkPort identifiers when the node/port has an origin object.
     */
    private static void assignIds(
            final LGraph lgraph,
            final Map<LNode, String> nodeIds,
            final Map<LPort, String> portIds) {

        int nodeCounter = 0;
        for (LNode node : getAllNodes(lgraph)) {
            String nodeId = deriveNodeId(node, nodeCounter++);
            nodeIds.put(node, nodeId);

            int portCounter = 0;
            for (LPort port : node.getPorts()) {
                String portId = derivePortId(port, nodeId, portCounter++);
                portIds.put(port, portId);
            }
        }
    }

    /**
     * Derives a stable ID for an LNode.
     * If the node was imported from an ElkNode, use the ElkNode's identifier.
     * Otherwise fall back to positional naming.
     */
    private static String deriveNodeId(final LNode node, final int globalIndex) {
        Object origin = node.getProperty(InternalProperties.ORIGIN);
        if (origin instanceof ElkNode) {
            String identifier = ((ElkNode) origin).getIdentifier();
            if (identifier != null && !identifier.isEmpty()) {
                return identifier;
            }
        }

        Layer layer = node.getLayer();
        if (layer != null) {
            int layerIndex = layer.getIndex();
            int inLayerIndex = node.getIndex();
            return "L" + layerIndex + "_N" + inLayerIndex;
        }

        return "N" + globalIndex;
    }

    /**
     * Derives a stable ID for an LPort.
     * If the port was imported from an ElkPort, use the ElkPort's identifier.
     * Otherwise use the parent node ID + port index.
     */
    private static String derivePortId(
            final LPort port,
            final String parentNodeId,
            final int portIndex) {

        Object origin = port.getProperty(InternalProperties.ORIGIN);
        if (origin instanceof org.eclipse.elk.graph.ElkPort) {
            String identifier = ((org.eclipse.elk.graph.ElkPort) origin).getIdentifier();
            if (identifier != null && !identifier.isEmpty()) {
                return identifier;
            }
        }
        return parentNodeId + "_P" + portIndex;
    }

    // ========================== Element Serialization ==========================

    private static void serializeNode(
            final JsonBuilder b,
            final LNode node,
            final Map<LNode, String> nodeIds,
            final Map<LPort, String> portIds) {

        b.beginObject();
        b.key("id"); b.value(nodeIds.getOrDefault(node, node.toString()));
        b.key("x"); b.value(node.getPosition().x);
        b.key("y"); b.value(node.getPosition().y);
        b.key("width"); b.value(node.getSize().x);
        b.key("height"); b.value(node.getSize().y);
        b.key("type"); b.value(node.getType().name());

        Layer layer = node.getLayer();
        b.key("layer"); b.value(layer != null ? layer.getIndex() : -1);

        String designation = node.getDesignation();
        if (designation != null && !designation.isEmpty()) {
            b.key("designation"); b.value(designation);
        }

        b.key("ports"); b.beginArray();
        for (LPort port : node.getPorts()) {
            serializePort(b, port, portIds);
        }
        b.endArray();

        b.key("labels"); b.beginArray();
        for (LLabel label : node.getLabels()) {
            serializeLabel(b, label);
        }
        b.endArray();

        b.key("margin"); b.beginObject();
        b.key("top"); b.value(node.getMargin().top);
        b.key("bottom"); b.value(node.getMargin().bottom);
        b.key("left"); b.value(node.getMargin().left);
        b.key("right"); b.value(node.getMargin().right);
        b.endObject();

        b.key("padding"); b.beginObject();
        b.key("top"); b.value(node.getPadding().top);
        b.key("bottom"); b.value(node.getPadding().bottom);
        b.key("left"); b.value(node.getPadding().left);
        b.key("right"); b.value(node.getPadding().right);
        b.endObject();

        b.endObject();
    }

    private static void serializePort(
            final JsonBuilder b,
            final LPort port,
            final Map<LPort, String> portIds) {

        b.beginObject();
        b.key("id"); b.value(portIds.getOrDefault(port, port.toString()));
        b.key("x"); b.value(port.getPosition().x);
        b.key("y"); b.value(port.getPosition().y);
        b.key("width"); b.value(port.getSize().x);
        b.key("height"); b.value(port.getSize().y);
        b.key("side"); b.value(port.getSide().name());

        KVector anchor = port.getAnchor();
        b.key("anchor"); b.beginObject();
        b.key("x"); b.value(anchor.x);
        b.key("y"); b.value(anchor.y);
        b.endObject();

        b.key("margin"); b.beginObject();
        b.key("top"); b.value(port.getMargin().top);
        b.key("bottom"); b.value(port.getMargin().bottom);
        b.key("left"); b.value(port.getMargin().left);
        b.key("right"); b.value(port.getMargin().right);
        b.endObject();

        b.key("labels"); b.beginArray();
        for (LLabel label : port.getLabels()) {
            serializeLabel(b, label);
        }
        b.endArray();

        b.endObject();
    }

    private static void serializeLabel(final JsonBuilder b, final LLabel label) {
        b.beginObject();
        String text = label.getText();
        b.key("text"); b.value(text != null ? text : "");
        b.key("x"); b.value(label.getPosition().x);
        b.key("y"); b.value(label.getPosition().y);
        b.key("width"); b.value(label.getSize().x);
        b.key("height"); b.value(label.getSize().y);
        b.endObject();
    }

    private static void serializeEdge(
            final JsonBuilder b,
            final LEdge edge,
            final int edgeIndex,
            final Map<LNode, String> nodeIds,
            final Map<LPort, String> portIds) {

        b.beginObject();
        b.key("id"); b.value("E" + edgeIndex);

        LPort source = edge.getSource();
        LPort target = edge.getTarget();

        b.key("sourceNode"); b.value(
                nodeIds.getOrDefault(source.getNode(), source.getNode().toString()));
        b.key("sourcePort"); b.value(
                portIds.getOrDefault(source, source.toString()));
        b.key("targetNode"); b.value(
                nodeIds.getOrDefault(target.getNode(), target.getNode().toString()));
        b.key("targetPort"); b.value(
                portIds.getOrDefault(target, target.toString()));

        b.key("bendPoints"); b.beginArray();
        KVectorChain bendPoints = edge.getBendPoints();
        if (bendPoints != null) {
            for (KVector bp : bendPoints) {
                b.beginObject();
                b.key("x"); b.value(bp.x);
                b.key("y"); b.value(bp.y);
                b.endObject();
            }
        }
        b.endArray();

        b.key("labels"); b.beginArray();
        for (LLabel label : edge.getLabels()) {
            serializeLabel(b, label);
        }
        b.endArray();

        b.endObject();
    }

    // ========================== File I/O Utilities ==========================

    private static ElkNode loadGraph(final Path modelFile) throws IOException {
        String content = new String(Files.readAllBytes(modelFile), StandardCharsets.UTF_8);
        if (modelFile.toString().endsWith(".json")) {
            // Load from JSON (preferred - avoids Xtext dependency)
            return ElkGraphJson.forGraph(content).toElk();
        }
        // For .elkt/.elkg files, try EMF ResourceSet (requires Xtext)
        ResourceSet resourceSet = new ResourceSetImpl();
        Resource resource = resourceSet.getResource(
                URI.createFileURI(modelFile.toString()), true);
        if (resource == null) {
            throw new IOException(
                    "failed to create EMF resource for model: " + modelFile);
        }
        resource.load(Collections.emptyMap());
        if (resource.getContents().isEmpty()) {
            throw new IOException(
                    "model resource has no root object: " + modelFile);
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
                    .filter(ElkPhaseTraceExporter::isSupportedModelFile)
                    .filter(path -> !matchesExcludeTokens(
                            modelsRoot, path, excludeTokens))
                    .filter(path -> matchesIncludeTokens(
                            modelsRoot, path, includeTokens))
                    .sorted(Comparator.comparing(
                            path -> toUnixPath(modelsRoot.relativize(path))))
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
        String relPath = toUnixPath(modelsRoot.relativize(path))
                .toLowerCase(Locale.ROOT);
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
        String relPath = toUnixPath(modelsRoot.relativize(path))
                .toLowerCase(Locale.ROOT);
        for (String token : excludeTokens) {
            if (relPath.contains(token.toLowerCase(Locale.ROOT))) {
                return true;
            }
        }
        return false;
    }

    private static int parseIntProperty(final String key, final int fallback) {
        String value = System.getProperty(key);
        if (value == null || value.isEmpty()) {
            return fallback;
        }
        try {
            return Integer.parseInt(value.trim());
        } catch (NumberFormatException exception) {
            return fallback;
        }
    }

    private static List<String> parseCsvTokens(final String raw) {
        if (raw == null || raw.isEmpty()) {
            return Collections.emptyList();
        }
        return Stream.of(raw.split(","))
                .map(String::trim)
                .filter(token -> !token.isEmpty())
                .collect(Collectors.toList());
    }

    private static String toUnixPath(final Path path) {
        return path.toString().replace('\\', '/');
    }

    private static void deleteRecursively(final Path target) throws IOException {
        if (!Files.exists(target)) {
            return;
        }
        try (Stream<Path> walk = Files.walk(target)) {
            for (Path path : walk.sorted(Comparator.reverseOrder())
                    .collect(Collectors.toList())) {
                Files.deleteIfExists(path);
            }
        }
    }

    // ========================== Minimal JSON Builder ==========================

    /**
     * Lightweight JSON writer using StringBuilder.
     * Supports objects, arrays, string values, and numeric (double/int) values.
     * Optional pretty-printing with 2-space indentation.
     */
    private static final class JsonBuilder {

        private final StringBuilder sb = new StringBuilder();
        private final boolean pretty;
        private int depth = 0;

        // Stack: 'o' = object, 'a' = array
        private final char[] stack = new char[256];
        // Track whether the current container already has at least one entry
        private final boolean[] hasEntry = new boolean[256];
        private int top = -1;

        JsonBuilder(final boolean pretty) {
            this.pretty = pretty;
        }

        // --- Structure ---

        void beginObject() {
            beforeValue();
            push('o');
            sb.append('{');
        }

        void endObject() {
            pop();
            if (pretty && hasEntry[top + 1]) {
                sb.append('\n');
                indent();
            }
            sb.append('}');
        }

        void beginArray() {
            beforeValue();
            push('a');
            sb.append('[');
        }

        void endArray() {
            pop();
            if (pretty && hasEntry[top + 1]) {
                sb.append('\n');
                indent();
            }
            sb.append(']');
        }

        void key(final String name) {
            // Object key: separator + indent + quoted name + colon
            if (top >= 0 && hasEntry[top]) {
                sb.append(',');
            }
            if (pretty) {
                sb.append('\n');
                indent();
            }
            appendString(name);
            sb.append(pretty ? ": " : ":");
            keyPending = true;
        }

        private boolean keyPending = false;

        void value(final String s) {
            beforeValue();
            if (s == null) {
                sb.append("null");
            } else {
                appendString(s);
            }
        }

        void value(final double d) {
            beforeValue();
            // Produce compact representation: no trailing .0 for whole numbers
            if (d == Math.floor(d) && !Double.isInfinite(d) && Math.abs(d) < 1e15) {
                sb.append((long) d);
            } else {
                sb.append(d);
            }
        }

        void value(final int i) {
            beforeValue();
            sb.append(i);
        }

        @Override
        public String toString() {
            return sb.toString();
        }

        // --- Internals ---

        private void push(final char type) {
            top++;
            stack[top] = type;
            hasEntry[top] = false;
        }

        private void pop() {
            top--;
        }

        private void beforeValue() {
            if (keyPending) {
                // The separator/indent was already written by key()
                keyPending = false;
                if (top >= 0) {
                    hasEntry[top] = true;
                }
                return;
            }
            if (top >= 0) {
                if (hasEntry[top]) {
                    sb.append(',');
                }
                if (pretty) {
                    sb.append('\n');
                    indent();
                }
                hasEntry[top] = true;
            }
        }

        private void indent() {
            for (int i = 0; i <= top; i++) {
                sb.append("  ");
            }
        }

        private void appendString(final String s) {
            sb.append('"');
            for (int i = 0; i < s.length(); i++) {
                char c = s.charAt(i);
                switch (c) {
                    case '"':  sb.append("\\\""); break;
                    case '\\': sb.append("\\\\"); break;
                    case '\n': sb.append("\\n");  break;
                    case '\r': sb.append("\\r");  break;
                    case '\t': sb.append("\\t");  break;
                    default:
                        if (c < 0x20) {
                            sb.append(String.format("\\u%04x", (int) c));
                        } else {
                            sb.append(c);
                        }
                        break;
                }
            }
            sb.append('"');
        }
    }
}
