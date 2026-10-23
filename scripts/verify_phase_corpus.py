#!/usr/bin/env python3
"""Account for every expected input, including specified errors, and verify raw traces."""
import argparse
import csv
import json
from pathlib import Path
import re
import sys

from compare_phase_traces import SnapshotComparator, compare_model, collect_step_files, _find_trace_dirs
from compare_model_parity_layouts import compare_json


def read_manifest(path):
    with path.open(encoding="utf-8", newline="") as stream:
        reader=csv.DictReader(stream, delimiter="\t")
        if not reader.fieldnames or len(set(reader.fieldnames)) != len(reader.fieldnames):
            raise ValueError(f"missing or duplicate manifest header: {path}")
        result={}
        for row in reader:
            name=row.get("model_rel_path", "")
            if not name or Path(name).is_absolute() or ".." in Path(name).parts or name in result:
                raise ValueError(f"invalid or duplicate model path: {name!r}")
            if None in row or any(value is None for value in row.values()):
                raise ValueError(f"malformed manifest row: {name}")
            result[name]=row
        if not result: raise ValueError(f"empty manifest: {path}")
        return result


def exception_class(message):
    match=re.search(r"org\.eclipse\.elk\.core\.[A-Za-z]+Exception", message)
    return match.group(0) if match else None


def check_outcomes(expected, java, rust, errors):
    if set(expected) != set(java) or set(expected) != set(rust):
        raise ValueError("input and result manifests must contain exactly the same models")
    if not set(errors) <= set(expected): raise ValueError("expected-error list contains unknown models")
    outcomes={}
    for name, source in expected.items():
        j,r=java[name],rust[name]
        for row in (j,r):
            if Path(row["input_json"]).resolve() != Path(source["input_json"]).resolve():
                raise ValueError(f"different input for {name}")
        if name in errors:
            error=errors[name]["exception"]
            if j.get("java_status")!="error" or r.get("rust_status")!="error" or exception_class(j.get("java_error", ""))!=error or exception_class(r.get("rust_error", ""))!=error:
                raise ValueError(f"expected matching {error} on both sides: {name}")
            outcomes[name]="expected_error"
        elif j.get("java_status") == "ok" and r.get("rust_status") == "ok":
            outcomes[name]="ok"
        else:
            raise ValueError(f"unexpected execution outcome: {name}: Java={j.get('java_status')} Rust={r.get('rust_status')}")
    return outcomes


def verify(inputs, java_manifest, rust_manifest, java_trace, rust_trace, expected_errors):
    expected=read_manifest(inputs); java=read_manifest(java_manifest); rust=read_manifest(rust_manifest)
    errors=json.loads(expected_errors.read_text())
    outcomes=check_outcomes(expected,java,rust,errors)
    expected_names={name.removesuffix(".json") for name in expected}
    if len(expected_names)!=len(expected): raise ValueError("ambiguous normalized input paths")
    for base in (java_trace,rust_trace):
        unexpected=set(_find_trace_dirs(base))-expected_names
        if unexpected: raise ValueError(f"unexpected trace models: {sorted(unexpected)}")
    results=[]; steps=0; failed=0; positive=0; negative=0
    comparator=SnapshotComparator()
    for name,status in outcomes.items():
        if status=="expected_error":
            negative+=1;results.append({"model":name,"status":status,"exception":errors[name]["exception"]});continue
        positive+=1
        jdir=java_trace/name;rdir=rust_trace/name
        index=json.loads((jdir/"index.json").read_text())
        expected_steps=index["steps"]
        discovered=collect_step_files(jdir)
        if [item["file"] for item in expected_steps] != [p.name for i,s,p in discovered if i>=0]:
            raise ValueError(f"Java index/step coverage mismatch: {name}")
        comparison=compare_model(jdir,rdir,name,comparator)
        steps+=comparison.match_count+comparison.drift_count
        layout_diffs=[]
        jlayout=json.loads(Path(java[name]["java_layout_json"]).read_text())
        rlayout=json.loads(Path(rust[name]["rust_layout_json"]).read_text())
        comparator._finite_tree(jlayout,"java/layout",layout_diffs)
        comparator._finite_tree(rlayout,"rust/layout",layout_diffs)
        numeric=[]
        compare_json(jlayout,rlayout,"root",1e-6,100,numeric)
        layout_messages=[d.summary_line() for d in layout_diffs]+numeric
        bad=comparison.drift_count!=0 or bool(layout_messages)
        failed+=int(bad)
        result={"model":name,"status":"failed" if bad else "match", "steps":len(comparison.steps),"trace_drift_steps":comparison.drift_count,"layout_differences":len(layout_messages)}
        if bad:
            result["layout_details"]=layout_messages[:20]
            result["trace_details"]=[s.to_dict() for s in comparison.steps if s.status!="match"][:3]
        results.append(result)
        if positive % 100 == 0: print(f"verified={positive} failed={failed}",file=sys.stderr,flush=True)
    return {"summary":{"expected_inputs":len(expected),"successful_layouts":positive,"expected_errors":negative,"verified_steps":steps,"failed_models":failed,"status":"pass" if failed==0 else "fail"},"models":results}


def main():
    p=argparse.ArgumentParser(description=__doc__)
    for name in ("inputs","java-manifest","rust-manifest","java-trace","rust-trace","expected-errors","output"):
        p.add_argument("--"+name,type=Path,required=True)
    a=p.parse_args()
    try:
        result=verify(a.inputs,a.java_manifest,a.rust_manifest,a.java_trace,a.rust_trace,a.expected_errors)
    except (OSError,ValueError,KeyError,TypeError) as error:
        result={"summary":{"status":"fail","error":str(error)}}
    a.output.write_text(json.dumps(result,indent=2,allow_nan=False)+"\n")
    print(json.dumps(result["summary"],sort_keys=True))
    return 0 if result["summary"]["status"]=="pass" else 1

if __name__=="__main__":sys.exit(main())
