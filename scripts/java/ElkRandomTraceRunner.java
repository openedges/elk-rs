package org.eclipse.elk.graph.json.test;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.Collections;
import java.util.List;
import java.util.Locale;

import org.eclipse.elk.alg.layered.ElkLayered;
import org.eclipse.elk.alg.layered.ElkLayered.TestExecutionState;
import org.eclipse.elk.alg.layered.graph.LGraph;
import org.eclipse.elk.alg.layered.options.InternalProperties;
import org.eclipse.elk.core.data.LayoutMetaDataService;
import org.eclipse.elk.core.options.CoreOptions;
import org.eclipse.elk.alg.layered.options.LayeredMetaDataProvider;
import org.eclipse.elk.core.alg.ILayoutProcessor;
import org.eclipse.elk.graph.ElkNode;
import org.eclipse.elk.graph.json.ElkGraphJson;
import org.eclipse.emf.common.util.URI;
import org.eclipse.emf.ecore.EObject;
import org.eclipse.emf.ecore.resource.Resource;
import org.eclipse.emf.ecore.resource.ResourceSet;
import org.eclipse.emf.ecore.resource.impl.ResourceSetImpl;

import java.lang.reflect.Constructor;
import java.lang.reflect.Method;

/**
 * Runs ELK layered layout on a single model with {@link TracingRandom} injected,
 * producing a random call trace on stderr that can be compared with the Rust
 * {@code CROSSMIN_RANDOM_TRACE=1} output.
 *
 * <p>Usage:</p>
 * <pre>
 * java ... ElkRandomTraceRunner &lt;model-path&gt; [randomSeed]
 * </pre>
 *
 * <p>The random trace is written to stderr. Redirect it to a file:</p>
 * <pre>
 * java ... ElkRandomTraceRunner model.elkt 1 2&gt;java_trace.txt
 * </pre>
 */
@SuppressWarnings("restriction")
public class ElkRandomTraceRunner {

    public static void main(String[] args) throws Exception {
        if (args.length < 1) {
            System.err.println("Usage: ElkRandomTraceRunner <model-path> [randomSeed]");
            System.exit(1);
        }

        // Register ELK layout providers
        LayoutMetaDataService.getInstance().registerLayoutMetaDataProviders(
                new CoreOptions(), new LayeredMetaDataProvider());

        Path modelPath = Paths.get(args[0]).toAbsolutePath().normalize();
        int randomSeed = args.length >= 2 ? Integer.parseInt(args[1]) : 1;

        if (!Files.exists(modelPath)) {
            System.err.println("Model file not found: " + modelPath);
            System.exit(1);
        }

        System.err.println("=== Java Random Trace Runner ===");
        System.err.println("Model: " + modelPath);
        System.err.println("Seed:  " + randomSeed);
        System.err.println("================================");

        // 1. Load ElkNode from file
        ElkNode loadedGraph = loadGraph(modelPath);
        loadedGraph.setProperty(CoreOptions.RANDOM_SEED, randomSeed);

        // 2. Round-trip through JSON (same as parity test)
        String inputJson = ElkGraphJson.forGraph(loadedGraph)
                .prettyPrint(false)
                .shortLayoutOptionKeys(false)
                .omitZeroPositions(false)
                .omitZeroDimension(false)
                .omitLayout(false)
                .omitUnknownLayoutOptions(false)
                .toJson();
        ElkNode reimportedGraph = ElkGraphJson.forGraph(inputJson).toElk();

        // 3. Import to LGraph (reflection for package-private access)
        Class<?> importerClass = Class.forName(
                "org.eclipse.elk.alg.layered.graph.transform.ElkGraphImporter");
        Constructor<?> ctor = importerClass.getDeclaredConstructor();
        ctor.setAccessible(true);
        Object graphImporter = ctor.newInstance();
        Method importMethod = importerClass.getMethod("importGraph", ElkNode.class);
        importMethod.setAccessible(true);
        LGraph lgraph = (LGraph) importMethod.invoke(graphImporter, reimportedGraph);

        // 4. Prepare layout test (sets up processor chain, GraphConfigurator creates Random)
        ElkLayered elkLayered = new ElkLayered();
        TestExecutionState state = elkLayered.prepareLayoutTest(lgraph);
        List<ILayoutProcessor<LGraph>> configuration =
                elkLayered.getLayoutTestConfiguration(state);

        // 5. Replace Random on all graphs with a SINGLE shared TracingRandom
        //    This matches how Java ELK actually works: GraphConfigurator sets Random
        //    on the root graph, ComponentsProcessor.split() copies the properties map
        //    so all components share the SAME Random object reference.
        TracingRandom sharedRandom = new TracingRandom(randomSeed);
        TracingRandom.resetGlobalCounter();
        for (LGraph graph : state.getGraphs()) {
            graph.setProperty(InternalProperties.RANDOM, sharedRandom);
        }

        System.err.println("=== BEGIN RANDOM TRACE ===");

        // 6. Step through all processors
        int step = 0;
        while (!elkLayered.isLayoutTestFinished(state)) {
            String processorName = configuration.get(state.getStep())
                    .getClass().getSimpleName();

            elkLayered.runLayoutTestStep(state);

            System.err.println("--- step " + step + ": " + processorName + " ---");
            step++;
        }

        System.err.println("=== END RANDOM TRACE ===");
        System.err.println("Total processors: " + step);
    }

    /**
     * Recursively replace InternalProperties.RANDOM on this graph and all
     * nested child graphs (inside compound nodes).
     */
    private static void replaceRandomRecursively(LGraph graph, int seed) {
        TracingRandom tr = new TracingRandom(seed);
        graph.setProperty(InternalProperties.RANDOM, tr);

        // Check all nodes for nested graphs
        for (org.eclipse.elk.alg.layered.graph.LNode node : getAllNodes(graph)) {
            LGraph nestedGraph = node.getNestedGraph();
            if (nestedGraph != null) {
                replaceRandomRecursively(nestedGraph, seed);
            }
        }
    }

    private static java.util.List<org.eclipse.elk.alg.layered.graph.LNode> getAllNodes(
            LGraph lgraph) {
        java.util.List<org.eclipse.elk.alg.layered.graph.LNode> all =
                new java.util.ArrayList<>();
        for (org.eclipse.elk.alg.layered.graph.Layer layer : lgraph.getLayers()) {
            all.addAll(layer.getNodes());
        }
        all.addAll(lgraph.getLayerlessNodes());
        return all;
    }

    private static ElkNode loadGraph(Path modelFile) throws IOException {
        String content = new String(Files.readAllBytes(modelFile), StandardCharsets.UTF_8);
        if (modelFile.toString().endsWith(".json")) {
            return ElkGraphJson.forGraph(content).toElk();
        }
        ResourceSet resourceSet = new ResourceSetImpl();
        Resource resource = resourceSet.getResource(
                URI.createFileURI(modelFile.toString()), true);
        if (resource == null) {
            throw new IOException("failed to create EMF resource: " + modelFile);
        }
        resource.load(Collections.emptyMap());
        if (resource.getContents().isEmpty()) {
            throw new IOException("model resource empty: " + modelFile);
        }
        EObject root = resource.getContents().get(0);
        if (!(root instanceof ElkNode)) {
            throw new IOException("root is not ElkNode: " + modelFile);
        }
        return (ElkNode) root;
    }
}
