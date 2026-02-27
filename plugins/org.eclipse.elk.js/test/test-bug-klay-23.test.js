/*******************************************************************************
 * Ported from elkjs — Copyright (c) 2017 Kiel University and others.
 * SPDX-License-Identifier: EPL-2.0
 *******************************************************************************/
import { describe, it } from 'vitest';
import ELK from '../js/index.js';

const elk = new ELK();

const simpleGraph = {
  id: "root",
  layoutOptions: {
    'elk.algorithm': 'layered',
    'elk.layered.crossingMinimization.strategy': 'INTERACTIVE'
  },
  children: [
    { id: "n1", width: 10, height: 10 },
    { id: "n2", width: 10, height: 10 }
  ],
  edges: [{
    id: "e1",
    sources: [ "n1" ],
    targets: [ "n2" ]
  },{
    id: "e2",
    sources: [ "n1" ],
    targets: [ "n2" ],
    sections: [{
      id: "es2",
      startPoint: { x: 0, y: 0 },
      bendPoints: [{ x: 20, y: 0 }],
      endPoint: { x: 50, y: 0 }
    }]
  }]
};

describe('klayjs#23', () => {
  describe('#layout(...)', () => {

    it('should be fine with unspecified bendpoints', async () => {
      await elk.layout(simpleGraph);
    });

  });
});
