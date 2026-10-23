/* SPDX-License-Identifier: EPL-2.0 */
package org.eclipse.elk.graph.json.test;

import java.nio.file.*;
import java.io.BufferedWriter;
import java.util.HashSet;
import com.google.gson.*;
import org.eclipse.elk.core.options.CoreOptions;
import org.eclipse.elk.core.util.Maybe;
import org.eclipse.elk.graph.ElkNode;
import org.eclipse.elk.graph.json.ElkGraphJson;
import org.eclipse.elk.graph.json.JsonImporter;

/** Execute exactly the manifest's JSON inputs, without a provider fallback. */
public final class ElkJsonTraceRunner {
    private static String cell(String text) {
        return text.replace('\t', ' ').replace('\r', ' ').replace('\n', ' ');
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 2) throw new IllegalArgumentException("Usage: ElkJsonTraceRunner input.tsv output-directory");
        Path output = Path.of(args[1]).toAbsolutePath();
        if (Files.exists(output)) throw new IllegalArgumentException("Output must not exist: " + output);
        Files.createDirectories(output);
        int failures = 0;
        HashSet<String> seen = new HashSet<>();
        try (BufferedWriter manifest = Files.newBufferedWriter(output.resolve("manifest.tsv"))) {
            manifest.write("model_rel_path\tinput_json\tjava_layout_json\tjava_status\tjava_error\n");
            java.util.List<String> lines = Files.readAllLines(Path.of(args[0]));
            for (String line : lines.subList(1, lines.size())) {
                String[] cols = line.split("\t", -1);
                if (cols.length < 2 || !seen.add(cols[0])) throw new IllegalArgumentException("Invalid/duplicate input row: " + line);
                Path name = Path.of(cols[0]);
                if (name.isAbsolute() || name.normalize().startsWith("..")) throw new IllegalArgumentException("Invalid model path: " + name);
                String status = "ok", error = "";
                Path layout = output.resolve("layout").resolve(cols[0] + ".json");
                try {
                    JsonObject json = JsonParser.parseString(Files.readString(Path.of(cols[1]))).getAsJsonObject();
                    Maybe<JsonImporter> importer = new Maybe<>();
                    ElkNode graph = ElkGraphJson.forGraph(json).rememberImporter(importer).toElk();
                    graph.setProperty(CoreOptions.RANDOM_SEED, 1);
                    graph.setProperty(CoreOptions.ALGORITHM, "org.eclipse.elk.layered");
                    Path trace = output.resolve("trace").resolve(name);
                    Files.createDirectories(trace);
                    ElkPhaseTraceExporter.traceWithRecursiveEngine(graph, trace, false);
                    importer.get().transferLayout(graph);
                    Files.createDirectories(layout.getParent());
                    Files.writeString(layout, json.toString());
                } catch (Exception ex) {
                    failures++;
                    status = "error";
                    Throwable cause = ex;
                    while (cause.getCause() != null && cause.getCause() != cause) cause = cause.getCause();
                    error = cause.toString();
                }
                manifest.write(cell(cols[0]) + "\t" + cell(cols[1]) + "\t" + cell(layout.toString()) + "\t" + status + "\t" + cell(error) + "\n");
                manifest.flush();
                System.err.println(seen.size() + " " + status + " " + cols[0]);
            }
        }
        if (seen.isEmpty()) throw new IllegalArgumentException("Input manifest is empty");
        System.out.println("total=" + seen.size() + " errors=" + failures);
        if (failures != 0) System.exit(1);
    }
}
