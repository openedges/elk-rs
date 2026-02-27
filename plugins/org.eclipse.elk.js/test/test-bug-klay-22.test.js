/*******************************************************************************
 * Ported from elkjs — Copyright (c) 2017 Kiel University and others.
 * SPDX-License-Identifier: EPL-2.0
 *******************************************************************************/
import { describe, it, expect } from 'vitest';
import ELK from '../js/index.js';

const elk = new ELK();

const simpleGraph = {
  id: "root",
  children: [
    { id: "n1", width: 100, height: 100,
      labels: [ { id: "l1", text: "Label1" } ] },
    { id: "n2", width: 100, height: 100,
      labels: [ {
        id: "l2",
        text: "Label2",
        layoutOptions: {
          'elk.nodeLabels.placement': 'INSIDE V_CENTER H_CENTER'
        }
      }],
    }
  ],
  edges: [{
    id: "e1",
    sources: [ "n1" ],
    targets: [ "n2" ]
  }]
};

let globalLayoutOptions = {
  'elk.nodeLabels.placement': 'OUTSIDE V_TOP H_CENTER'
};

describe('klayjs#22', () => {
  describe('#layout(...)', () => {

    it('should place labels according to set options', async () => {
      const graph = await elk.layout(simpleGraph, {
        layoutOptions: globalLayoutOptions
      });
      // OUTSIDE V_TOP H_CENTER
      expect(graph.children[0].labels[0].x).toBe(50);
      expect(graph.children[0].labels[0].y).toBe(-5);
      // INSIDE V_CENTER H_CENTER
      expect(graph.children[1].labels[0].x).toBe(50);
      expect(graph.children[1].labels[0].y).toBe(50);
    });

  });
});
