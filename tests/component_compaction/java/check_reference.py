#!/usr/bin/env python3
"""Check Java references in an isolated directory; never modify external/elk."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


def run(command, **kwargs):
    result = subprocess.run(command, capture_output=True, text=True, **kwargs)
    if result.returncode:
        raise RuntimeError(result.stderr or result.stdout)
    return result.stdout.strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--classpath', required=True, help='ELK v0.12.0 classes/jars and their dependencies')
    parser.add_argument('--elk-source', type=Path, required=True, help='Unmodified ELK v0.12.0 source checkout')
    parser.add_argument('--output', type=Path, help='Empty scratch directory; defaults to a temporary directory')
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    fixtures = here.parent
    repository = here.parents[2]
    sys.path.insert(0, str(repository / 'scripts'))
    from compare_model_parity_layouts import compare_json

    work = args.output.resolve() if args.output else Path(tempfile.mkdtemp(prefix='elk-component-reference-'))
    if work.exists() and any(work.iterdir()):
        raise ValueError('The output directory must be empty')
    work.mkdir(parents=True, exist_ok=True)
    clean = work / 'original-classes'
    fixed = work / 'corrected-classes'
    clean.mkdir()
    fixed.mkdir()
    sources = [str(here / name) for name in ['LayoutProbe.java', 'ConstraintsProbe.java']]
    run(['javac', '-cp', args.classpath, '-d', str(clean), *sources])
    source_path = Path('plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/compaction/oned/algs/ScanlineConstraintCalculator.java')
    target = work / source_path
    target.parent.mkdir(parents=True)
    shutil.copyfile(args.elk_source / source_path, target)
    run(['git', 'apply', str(here / 'zero-extent-scanline.patch')], cwd=work)
    run(['javac', '-cp', args.classpath, '-d', str(fixed), *sources, str(target)])

    for classes, expected in [(clean, 'nonfinite'), (fixed, 'finite')]:
        actual = run(['java', '-cp', str(classes) + os.pathsep + args.classpath,
                      'ConstraintsProbe', str(here / 'minimal-rectangles.json')])
        if actual != expected:
            raise AssertionError(f'{classes.name}: expected {expected}, actual {actual}')
        print(f'Minimal reproduction ({classes.name}): {actual}', flush=True)

    references = json.loads((fixtures / 'references.json').read_text())
    for name, reference in references.items():
        classes = fixed if reference == 'zero-extent-scanline.patch' else clean
        output = work / (name + '.json')
        run(['java', '-cp', str(classes) + os.pathsep + args.classpath, 'LayoutProbe',
             str(fixtures / (name + '.input.json')), str(output)])
        expected = json.loads((fixtures / (name + '.expected.json')).read_text())
        actual = json.loads(output.read_text())
        diffs = []
        compare_json(expected, actual, '', 1e-6, 20, diffs)
        if diffs:
            raise AssertionError(f'{name}: {diffs}')
        print(f'{name}: matches ({reference})', flush=True)
    print(f'All {len(references)} Java references verified. Evidence: {work}')


if __name__ == '__main__':
    main()
