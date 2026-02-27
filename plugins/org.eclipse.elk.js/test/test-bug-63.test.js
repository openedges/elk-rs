/*******************************************************************************
 * Ported from elkjs — Copyright (c) 2017 Kiel University and others.
 * SPDX-License-Identifier: EPL-2.0
 *******************************************************************************/
import { describe, it } from 'vitest';
import ELK from '../js/index.js';

const elk = new ELK();

describe('elkjs#63', () => {
  describe('#layout()', () => {

    it('COFFMAN_GRAHAM layering should cope with selfloops.', async () => {
      await elk.layout(graph);
    });

  });
});

const graph = {
  id: "root",
  properties: {
      'algorithm': 'layered',
      'layering.strategy': 'COFFMAN_GRAHAM' },
  children: [
    { id: "n1", width: 30, height: 30 },
    { id: "n2", width: 30, height: 30 },
    { id: "n3", width: 30, height: 30 }
  ],
  edges: [
    { id: "e1", sources: [ "n1" ], targets: [ "n2" ] },
    { id: "e2", sources: [ "n1" ], targets: [ "n3" ] },
    { id: "e3", sources: [ "n1" ], targets: [ "n1" ] }
  ]
};
