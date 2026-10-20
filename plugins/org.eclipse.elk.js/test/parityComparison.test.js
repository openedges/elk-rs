import { describe, expect, it } from 'vitest';
import { classifyResults, parseJavaManifest, usesExtraAlgorithm } from './parity/compare.mjs';

const java = { id: 'root', width: 100 };
const drift = { id: 'root', width: 120 };

describe('three-way parity classification', () => {
  it('rejects shared JavaScript drift instead of claiming a Java match', () => {
    expect(classifyResults({ java, rust: drift, elkjs: drift })).toMatchObject({
      classification: 'SHARED_DRIFT', failure: true, rustMatchesJava: false,
    });
  });

  it('counts all-different output as a failure', () => {
    expect(classifyResults({ java, rust: drift, elkjs: { ...java, width: 90 } }))
      .toMatchObject({ classification: 'ALL_DIFFER', failure: true });
  });

  it('does not count a two-way match as Java verification', () => {
    expect(classifyResults({ rust: java, elkjs: java })).toMatchObject({
      classification: 'TWO_WAY_MATCH', hasJava: false, rustMatchesJava: false,
    });
  });

  it('verifies Rust extra capabilities against Java before accepting them', () => {
    const results = { java, elkjsError: 'Algorithm unavailable', extraCapability: true };
    expect(classifyResults({ ...results, rust: java })).toMatchObject({
      classification: 'RUST_EXTRA', rustMatchesJava: true, failure: false,
    });
    expect(classifyResults({ ...results, rust: drift })).toMatchObject({
      classification: 'ELKRS_DRIFT', failure: true,
    });
  });

  it('rejects Rust errors when Java succeeds even if elkjs also fails', () => {
    expect(classifyResults({ java, rustError: 'failed', elkjsError: 'failed' }))
      .toMatchObject({ classification: 'ELKRS_ERROR', failure: true });
  });

  it('maps original JSON baselines ahead of legacy .elkg aliases', () => {
    const manifest = 'a.json\tinput\tnew.json.json\tok\n'
      + 'a.elkg\tinput\told.elkg.json\tok\n'
      + 'b.json\tinput\tbad.json\terror\n';
    expect([...parseJavaManifest(manifest)]).toEqual([['a.json', 'new.json.json']]);
  });

  it('does not replace a failed original JSON export with a sibling .elkg output', () => {
    expect(parseJavaManifest('a.elkg\tinput\ta.elkg.json\tok\n'
      + 'a.json\tinput\ta.json.json\terror\n').has('a.json')).toBe(false);
  });

  it('recognizes nested extra algorithms with supported identifier aliases', () => {
    expect(usesExtraAlgorithm({ children: [{ properties: { algorithm: 'elk.vertiflex' } }] },
      new Set(['org.eclipse.elk.vertiflex']))).toBe(true);
    expect(usesExtraAlgorithm({ layoutOptions: { algorithm: 'unknown' } },
      new Set(['org.eclipse.elk.vertiflex']))).toBe(false);
  });
});
