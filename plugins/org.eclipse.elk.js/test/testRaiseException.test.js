/*******************************************************************************
 * Ported from elkjs — Copyright (c) 2020 Kiel University and others.
 * SPDX-License-Identifier: EPL-2.0
 *******************************************************************************/
import { describe, it, expect } from 'vitest';
import ELK from '../js/index.js';

const elk = new ELK();

// A simple cycle for which it is not possible to have all nodes in the very first layer
const graph = {
  id: "root",
  properties: { 'algorithm': 'layered' },
  children: [
    { id: "n1", width: 30, height: 30, layoutOptions: { layerConstraint: "FIRST" } },
    { id: "n2", width: 30, height: 30, layoutOptions: { layerConstraint: "FIRST" } },
    { id: "n3", width: 30, height: 30, layoutOptions: { layerConstraint: "FIRST" } }
  ],
  edges: [
    { id: "e1", sources: ["n1"], targets: ["n2"] },
    { id: "e2", sources: ["n2"], targets: ["n3"] },
    { id: "e3", sources: ["n3"], targets: ["n1"] }
  ]
};

describe('Exceptions', () => {
  describe('#layout()', () => {

    it('should report an unsupported configuration.', async () => {
      await expect(elk.layout(graph)).rejects.toThrow(
        /org\.eclipse\.elk\.core\.UnsupportedConfigurationException/
      );
    });

  });
});
