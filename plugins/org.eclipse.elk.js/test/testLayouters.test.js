/*******************************************************************************
 * Ported from elkjs — Copyright (c) 2018 Kiel University and others.
 * SPDX-License-Identifier: EPL-2.0
 *******************************************************************************/
import { describe, it, expect } from 'vitest';
import ELK from '../js/index.js';

const elk = new ELK();

const graph = {
  id: 'root',
  children: [
    { id: 'n1', x: 20, y: 20, width: 10, height: 10 },
    { id: 'n2', x: 50, y: 50, width: 10, height: 10 }
  ],
  edges: [{ id: 'e1', sources: [ 'n1' ], targets: [ 'n2' ] }]
};

const graphOverlapping = {
  id: 'root',
  children: [
    { id: 'n1', x: 20, y: 20, width: 10, height: 10 },
    { id: 'n2', x: 25, y: 25, width: 10, height: 10 }
  ],
  edges: [{ id: 'e1', sources: [ 'n1' ], targets: [ 'n2' ] }]
};

describe('Layout Algorithms', () => {

  it('SPOrE Compaction', async () => {
    const result = await elk.layout(graph, {
      layoutOptions: {
        'algorithm': 'elk.sporeCompaction',
        'elk.spacing.nodeNode': 14,
        'elk.padding': '[left=2, top=2, right=2, bottom=2]'
      }
    });
    expect(result.children[0].x).toBe(2);
    expect(result.children[0].y).toBe(2);
    expect(result.children[1].x).toBe(26);
    expect(result.children[1].y).toBe(26);
  });

  it('SPOrE Overlap Removal', async () => {
    const result = await elk.layout(graphOverlapping, {
      layoutOptions: {
        'algorithm': 'elk.sporeOverlap',
        'elk.spacing.nodeNode': 13,
        'elk.padding': '[left=3, top=3, right=3, bottom=3]' }
    });
    expect(result.children[0].x).toBe(3);
    expect(result.children[0].y).toBe(3);
    expect(result.children[1].x).toBe(26);
    expect(result.children[1].y).toBe(26);
  });

  it('Rectangle Packing', async () => {
    await elk.layout(graphOverlapping, {
      layoutOptions: {
        'algorithm': 'elk.rectpacking'
      }
    });
  });

});
