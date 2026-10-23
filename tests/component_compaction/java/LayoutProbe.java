import java.nio.file.*;
import com.google.gson.*;
import org.eclipse.elk.core.*;
import org.eclipse.elk.core.util.*;
import org.eclipse.elk.graph.*;
import org.eclipse.elk.graph.json.*;

public class LayoutProbe {
    public static void main(String[] args) throws Exception {
        JsonObject json = JsonParser.parseString(Files.readString(Path.of(args[0]))).getAsJsonObject();
        Maybe<org.eclipse.elk.graph.json.JsonImporter> importer = new Maybe<>();
        ElkNode graph = ElkGraphJson.forGraph(json).rememberImporter(importer).toElk();
        new RecursiveGraphLayoutEngine().layout(graph, new BasicProgressMonitor());
        importer.get().transferLayout(graph);
        Files.writeString(Path.of(args[1]), json.toString());
    }
}
