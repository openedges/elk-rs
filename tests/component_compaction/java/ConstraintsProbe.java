import java.nio.file.*;
import java.util.*;
import com.google.gson.*;
import org.eclipse.elk.alg.layered.compaction.oned.*;
import org.eclipse.elk.core.math.ElkRectangle;
import org.eclipse.elk.core.options.Direction;

public class ConstraintsProbe {
    static class Rect extends CNode {
        Rect(double x,double y,double w,double h) { hitbox=new ElkRectangle(x,y,w,h); }
        public double getHorizontalSpacing() { return 20; }
        public double getVerticalSpacing() { return 20; }
        public double getElementPosition() { return hitbox.x; }
        public void applyElementPosition() { }
    }
    public static void main(String[] args) throws Exception {
        CGraph graph=new CGraph(EnumSet.allOf(Direction.class));
        Map<Integer,CGroup> groups=new LinkedHashMap<>();
        for (JsonElement el : JsonParser.parseString(Files.readString(Path.of(args[0]))).getAsJsonArray()) {
            JsonArray a=el.getAsJsonArray();
            int id=a.get(0).getAsInt();
            CGroup g=groups.computeIfAbsent(id,k->{CGroup v=new CGroup();graph.cGroups.add(v);return v;});
            Rect n=new Rect(a.get(1).getAsDouble(),a.get(2).getAsDouble(),a.get(3).getAsDouble(),a.get(4).getAsDouble());
            graph.cNodes.add(n);g.addCNode(n);
        }
        new OneDimensionalCompactor(graph).compact();
        boolean finite=graph.cNodes.stream().allMatch(n->Double.isFinite(n.hitbox.x)&&Double.isFinite(n.hitbox.y));
        System.out.println(finite?"finite":"nonfinite");
    }
}
