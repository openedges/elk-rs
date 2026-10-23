#!/usr/bin/env python3
"""Build a corrected Java reference and verify every model through raw phase traces."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys

from verify_phase_corpus import read_manifest

ROOT=Path(__file__).resolve().parents[1]


def digest(path): return hashlib.sha256(path.read_bytes()).hexdigest()


def run(command, log, *, cwd=ROOT, env=None, allowed=(0,)):
    with log.open("w") as stream:
        result=subprocess.run(command,cwd=cwd,env=env,stdout=stream,stderr=subprocess.STDOUT)
    if result.returncode not in allowed:
        raise RuntimeError(f"command failed ({result.returncode}); see {log}")
    return result.returncode


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument("--inputs",type=Path,required=True)
    p.add_argument("--java-classpath",required=True)
    p.add_argument("--output",type=Path,required=True)
    p.add_argument("--elk-source",type=Path,default=ROOT/"external/elk")
    p.add_argument("--models-root",type=Path,default=ROOT/"external/elk-models")
    a=p.parse_args();out=a.output.resolve();inputs=a.inputs.resolve();source=a.elk_source.resolve()
    if out.exists():p.error("output directory must not exist (stale results are not reusable)")
    rows=read_manifest(inputs)
    corpus={str(path.relative_to(a.models_root)) for path in a.models_root.rglob("*") if path.suffix in (".json",".elkg",".elkt") and path.is_file()}
    if set(rows)!=corpus:p.error("input manifest must account for every model under models-root")
    hashes={name:digest(Path(row["input_json"])) for name,row in rows.items()}
    out.mkdir(parents=True)
    copied=out/"reference-source";copied.mkdir()
    patches=sorted((ROOT/"scripts/java/patches").glob("*.patch"));java_sources=[]
    for patch in patches:
        for name in re.findall(r"^--- a/(.+)$",patch.read_text(),re.M):
            if Path(name).is_absolute() or ".." in Path(name).parts:raise ValueError("invalid patch path")
            target=copied/name;target.parent.mkdir(parents=True,exist_ok=True)
            if not target.exists():target.write_bytes((source/name).read_bytes());java_sources.append(str(target))
        run(["git","apply",str(patch)],out/(patch.stem+".log"),cwd=copied)
    classes=out/"java-classes"
    run(["javac","-cp",a.java_classpath,"-d",str(classes),*java_sources,str(ROOT/"scripts/java/ElkPhaseTraceExporter.java"),str(ROOT/"scripts/java/ElkJsonTraceRunner.java")],out/"javac.log")
    untracked = subprocess.check_output(["git", "ls-files", "--others", "--exclude-standard", "-z"], cwd=ROOT).decode().split("\0")
    metadata={"untracked_file_sha256": {name: digest(ROOT / name) for name in untracked if name and (ROOT / name).is_file()},
              "java_source_commit":subprocess.check_output(["git","rev-parse","HEAD"],cwd=source,text=True).strip(),
              "rust_commit":subprocess.check_output(["git","rev-parse","HEAD"],cwd=ROOT,text=True).strip(),
              "rust_worktree_diff_sha256":hashlib.sha256(subprocess.check_output(["git","diff","HEAD"],cwd=ROOT)).hexdigest(),
              "patches":{patch.name:digest(patch) for patch in patches},"input_sha256":hashes,
              "seed":1,"algorithm":"org.eclipse.elk.layered","execution":"recursive","trace":"raw"}
    (out/"provenance.json").write_text(json.dumps(metadata,indent=2)+"\n")
    run(["cargo","build","--workspace"],out/"build.log")
    print("Exporting corrected Java reference",flush=True)
    run(["java","-Xmx4g","-cp",str(classes)+os.pathsep+a.java_classpath,"org.eclipse.elk.graph.json.test.ElkJsonTraceRunner",str(inputs),str(out/"java")],out/"java.log",allowed=(0,1))
    print("Exporting Rust states",flush=True)
    env=dict(os.environ,MODEL_PARITY_RANDOM_SEED="1")
    run([str(ROOT/"target/debug/model_parity_layout_runner"),"--input-manifest",str(out/"java/manifest.tsv"),"--output-manifest",str(out/"rust-manifest.tsv"),"--rust-layout-dir",str(out/"rust-layout"),"--trace-dir",str(out/"rust-trace")],out/"rust.log",env=env,allowed=(0,1))
    if hashes!={name:digest(Path(row["input_json"])) for name,row in rows.items()}:raise ValueError("inputs changed during the run")
    print("Verifying all outcomes, layouts, and intermediate states",flush=True)
    result=run([sys.executable,"-B",str(ROOT/"scripts/verify_phase_corpus.py"),"--inputs",str(inputs),"--java-manifest",str(out/"java/manifest.tsv"),"--rust-manifest",str(out/"rust-manifest.tsv"),"--java-trace",str(out/"java/trace"),"--rust-trace",str(out/"rust-trace"),"--expected-errors",str(ROOT/"tests/phase_trace_expected_errors.json"),"--output",str(out/"report.json")],out/"verification.log",allowed=(0,1))
    print((out/"verification.log").read_text())
    return result

if __name__=="__main__":
    try:sys.exit(main())
    except (OSError,ValueError,RuntimeError) as error:
        print(f"error: {error}",file=sys.stderr);sys.exit(1)
