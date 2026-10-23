"""Regressions for trace false positives and representation-only renaming."""
import copy
import contextlib
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from test_compare_layout_numbers import phase


def snapshot():
    zero = {"x": 0.0, "y": 0.0}
    box = {"top": 0.0, "bottom": 0.0, "left": 0.0, "right": 0.0}
    def node(index):
        return {"id": f"N{index}", "type": "NORMAL", "layer": 0, "x": index * 20.0,
                "y": 0.0, "width": 10.0, "height": 10.0, "margin": box.copy(), "padding": box.copy(), "labels": [],
                "ports": [{"id": f"P{index}", **zero, "width": 0.0, "height": 0.0, "side": "EAST",
                           "anchor": zero.copy(), "margin": box.copy(), "labels": []}]}
    return {"step": 0, "processor": "Test", "nodes": [node(0), node(1)],
            "edges": [{"id": "E0", "sourceNode": "N0", "sourcePort": "P0", "targetNode": "N1", "targetPort": "P1", "bendPoints": [], "labels": []}],
            "layers": [["N0", "N1"]], "graphSize": {"width": 30.0, "height": 10.0},
            "size": {"width": 30.0, "height": 10.0}, "padding": box, "offset": zero}


class TraceIntegrityTests(unittest.TestCase):
    def test_every_geometry_group_is_compared(self):
        paths = [("nodes",0,"x"), ("nodes",0,"ports",0,"width"), ("nodes",0,"ports",0,"anchor","x"),
                 ("nodes",0,"ports",0,"margin","top"), ("nodes",0,"padding","top"),
                 ("nodes",0,"margin","top"), ("size","width"), ("offset","x"), ("padding","top")]
        for path in paths:
            a=snapshot(); b=copy.deepcopy(a); cursor=b
            for key in path[:-1]: cursor=cursor[key]
            cursor[path[-1]] += 37
            with self.subTest(path=path): self.assertTrue(phase.SnapshotComparator().compare(a,b))

    def test_nonfinite_values_in_unrecognized_fields_fail(self):
        for value in (float("nan"), float("inf"), -float("inf")):
            a=snapshot(); a["extra"]={"geometry":value}
            self.assertTrue(phase.SnapshotComparator().compare(a,copy.deepcopy(a)))

    def test_signed_zero_does_not_change_port_identity(self):
        a=snapshot(); b=copy.deepcopy(a)
        b["nodes"][0]["ports"][0]["x"]=-0.0
        self.assertFalse(phase.SnapshotComparator().compare(a,b))

    def test_null_geometry_and_layer_membership_corruption_fail(self):
        a=snapshot(); b=copy.deepcopy(a);b["nodes"][0]["width"]=None
        self.assertTrue(phase.validate_snapshot(b,"rust"))
        for components in ([], {}, [None], [{}]):
            with self.subTest(components=components):
                self.assertTrue(phase.validate_snapshot({"components": components}, "rust"))
        b=copy.deepcopy(a);b["layers"]=[["N0","N0"]]
        self.assertTrue(phase.SnapshotComparator().compare(a,b))

    def test_endpoint_rewiring_is_detected(self):
        a=snapshot(); b=copy.deepcopy(a)
        b["edges"][0].update(targetNode="N0", targetPort="P0")
        self.assertTrue(phase.SnapshotComparator().compare(a,b))

    def test_identifier_renaming_is_allowed(self):
        a=snapshot(); b=json.loads(json.dumps(a).replace('N0','renamed-node').replace('P0','renamed-port'))
        self.assertFalse(phase.SnapshotComparator().compare(a,b))

    def test_coincident_port_permutation_preserves_connectivity(self):
        a=snapshot(); a["nodes"][1]["ports"].append(dict(a["nodes"][1]["ports"][0],id="P2"))
        b=copy.deepcopy(a); b["nodes"][1]["ports"].reverse()
        self.assertFalse(phase.SnapshotComparator().compare(a,b))
        b["edges"][0]["targetNode"]="N0"; b["edges"][0]["targetPort"]="P0"
        self.assertTrue(phase.SnapshotComparator().compare(a,b))

    def test_empty_and_missing_batches_fail(self):
        with tempfile.TemporaryDirectory() as root:
            j=Path(root)/"java"; r=Path(root)/"rust"; j.mkdir();r.mkdir()
            for missing in (False,True):
                if missing:
                    m=r/"model";m.mkdir();(m/"step_000_Test.json").write_text(json.dumps(snapshot()))
                with patch("sys.argv",["compare",str(j),str(r),"--batch"]), contextlib.redirect_stdout(io.StringIO()):
                    self.assertNotEqual(phase.main(),0)

    def test_empty_single_model_fails(self):
        with tempfile.TemporaryDirectory() as root:
            p=Path(root)
            result=phase.compare_model(p,p,"empty",phase.SnapshotComparator())
            self.assertGreater(result.drift_count,0)

    def test_duplicate_steps_and_holes_fail(self):
        for names in (("step_000_A.json","step_000_B.json"),("step_000_A.json","step_002_B.json")):
            with tempfile.TemporaryDirectory() as root:
                for name in names: (Path(root)/name).write_text("{}")
                with self.assertRaises(ValueError): phase.collect_step_files(Path(root))

    def test_duplicate_normalized_model_names_fail(self):
        with tempfile.TemporaryDirectory() as root:
            for name in ("model","model.json"):
                p=Path(root)/name;p.mkdir();(p/"step_000_Test.json").write_text("{}")
            with self.assertRaises(ValueError): phase._find_trace_dirs(Path(root))

    def test_step_metadata_and_missing_snapshot_fields_fail(self):
        with tempfile.TemporaryDirectory() as root:
            p=Path(root);s=snapshot();s["step"]=42;del s["size"]
            (p/"step_000_Test.json").write_text(json.dumps(s))
            self.assertGreater(phase.compare_model(p,p,"model",phase.SnapshotComparator()).drift_count,0)

if __name__ == "__main__": unittest.main()
