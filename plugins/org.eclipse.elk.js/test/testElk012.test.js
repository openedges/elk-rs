import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import ELK from '../js/index.js';

const option = 'org.eclipse.elk.alg.layered.nodePlacement.networkSimplex.nodeFlexibility.recomputeNodePlacement';
function fixture(name) {
  return JSON.parse(readFileSync(new URL(`../../org.eclipse.elk.alg.layered/tests/resources/node_flexibility/${name}.json`, import.meta.url), 'utf8'));
}
function positions(graph) {
  return graph.children.map(({ id, x, y, width, height, ports, labels }) => ({ id, x, y, width, height, ports, labels }));
}

describe('ELK 0.12 node flexibility', () => {
  it('exposes the recomputation option and preserves null-as-disabled behavior', async () => {
    const elk = new ELK();
    expect((await elk.knownLayoutOptions()).map(description => description.id)).toContain(option);
    expect(positions(await elk.layout(fixture('null_recompute'))))
      .toEqual(positions(await elk.layout(fixture('no_recompute'))));
  });

  it('recomputes inherited flexible sizes and refreshes ports and labels', async () => {
    const elk = new ELK();
    const inherited = await elk.layout(fixture('recompute_inherited'));
    const explicit = await elk.layout(fixture('recompute_brandes_koepf'));
    expect(positions(inherited)).toEqual(positions(explicit));
    expect(positions(explicit)).not.toEqual(positions(await elk.layout(fixture('no_recompute'))));
  });

  it('separates the equal-size split nodes', async () => {
    const result = await new ELK().layout(fixture('unzipping_equal_split'));
    const a = result.children.find(node => node.id === 'a');
    const b = result.children.find(node => node.id === 'b');
    expect(a.x).not.toBe(b.x);
  });
});
