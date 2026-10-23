"""Manifest accounting must not turn missing or unexpected cases into a pass."""
import copy
from pathlib import Path
import tempfile
import unittest
from test_compare_layout_numbers import load_script
corpus=load_script("verify_phase_corpus")

class OutcomeTests(unittest.TestCase):
    def rows(self):
        inputs={"good.json":{"input_json":"/tmp/good.json"},"bad.json":{"input_json":"/tmp/bad.json"}}
        java={name:{**row,"java_status":"ok","java_error":""} for name,row in inputs.items()}
        rust={name:{**row,"rust_status":"ok","rust_error":""} for name,row in inputs.items()}
        exception="org.eclipse.elk.core.UnsupportedGraphException"
        java["bad.json"].update(java_status="error",java_error=exception+": bad edge")
        rust["bad.json"].update(rust_status="error",rust_error="panic during layout: "+exception+": bad edge")
        return inputs,java,rust,{"bad.json":{"exception":exception}}

    def test_both_positive_and_expected_negative_are_accounted_for(self):
        result=corpus.check_outcomes(*self.rows())
        self.assertEqual(result,{"good.json":"ok","bad.json":"expected_error"})

    def test_missing_results_even_on_both_sides_fail(self):
        inputs,java,rust,errors=self.rows();del java["good.json"];del rust["good.json"]
        with self.assertRaises(ValueError):corpus.check_outcomes(inputs,java,rust,errors)

    def test_wrong_exception_unexpected_error_or_unexpected_success_fail(self):
        for status,error in (("ok",""),("timeout","timeout"),("error","unrelated panic")):
            inputs,java,rust,errors=self.rows();rust["bad.json"].update(rust_status=status,rust_error=error)
            with self.assertRaises(ValueError):corpus.check_outcomes(inputs,java,rust,errors)
        inputs,java,rust,errors=self.rows();java["good.json"]["java_status"]="error";rust["good.json"]["rust_status"]="error"
        with self.assertRaises(ValueError):corpus.check_outcomes(inputs,java,rust,errors)

    def test_changed_input_and_stale_expected_error_fail(self):
        inputs,java,rust,errors=self.rows();rust["good.json"]["input_json"]="/tmp/different.json"
        with self.assertRaises(ValueError):corpus.check_outcomes(inputs,java,rust,errors)
        inputs,java,rust,errors=self.rows();errors["unknown.json"]={"exception":"Exception"}
        with self.assertRaises(ValueError):corpus.check_outcomes(inputs,java,rust,errors)

    def test_empty_duplicate_and_malformed_manifests_fail(self):
        for text in ("model_rel_path\tinput_json\n", "model_rel_path\tinput_json\nx\ta\nx\tb\n", "model_rel_path\tinput_json\nx\n"):
            with tempfile.TemporaryDirectory() as root:
                p=Path(root)/"manifest.tsv";p.write_text(text)
                with self.assertRaises(ValueError):corpus.read_manifest(p)

if __name__=="__main__":unittest.main()
