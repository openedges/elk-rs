"""Regression tests for numeric layout comparison, without a Java/Rust build."""

import importlib.util
from pathlib import Path
import sys
import unittest


def load_script(name):
    path = Path(__file__).resolve().parents[1] / f"{name}.py"
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


phase = load_script("compare_phase_traces")
model = load_script("compare_model_parity_layouts")


class NumericComparisonTests(unittest.TestCase):
    def phase_diffs(self, left, right):
        return phase.SnapshotComparator().compare(
            {"nodes": [{"x": left}]}, {"nodes": [{"x": right}]}
        )

    def model_diffs(self, left, right):
        diffs = []
        model.compare_json({"x": left}, {"x": right}, "root", 1e-6, 20, diffs)
        return diffs

    def test_nonfinite_geometry_never_matches(self):
        for bad in (float("nan"), float("inf"), float("-inf")):
            for other in (0.0, 22.0, bad, -bad):
                for left, right in ((bad, other), (other, bad)):
                    with self.subTest(left=left, right=right):
                        self.assertTrue(self.phase_diffs(left, right))
                        self.assertTrue(self.model_diffs(left, right))

    def test_finite_values_match_within_each_tolerance(self):
        self.assertFalse(self.phase_diffs(22.0, 22.0005))
        self.assertTrue(self.phase_diffs(22.0, 22.01))
        self.assertFalse(self.model_diffs(22.0, 22.0000005))
        self.assertTrue(self.model_diffs(22.0, 22.00001))

    def test_zero_coordinates_are_compared_normally(self):
        self.assertTrue(self.phase_diffs(0.0, 22.0))
        self.assertTrue(self.phase_diffs(22.0, 0.0))
        self.assertTrue(self.model_diffs(0.0, 22.0))

    def test_missing_geometry_remains_a_difference(self):
        self.assertTrue(self.phase_diffs(None, 22.0))
        self.assertTrue(self.model_diffs(None, 22.0))


if __name__ == "__main__":
    unittest.main()
