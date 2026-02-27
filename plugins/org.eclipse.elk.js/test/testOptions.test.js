/*******************************************************************************
 * Ported from elkjs — Copyright (c) 2017 Kiel University and others.
 * SPDX-License-Identifier: EPL-2.0
 *******************************************************************************/
import { describe, it, expect } from 'vitest';
import ELK from '../js/index.js';

const elk = new ELK();

var simpleGraph = {
  id: "root",
  layoutOptions: { 'elk.direction': 'RIGHT' },
  children: [
    { id: "n1", width: 10, height: 10 },
    { id: "n2", width: 10, height: 10 }
  ],
  edges: [{
    id: "e1",
    sources: [ "n1" ],
    targets: [ "n2" ]
  }]
};

describe('Layout Options', () => {

  describe('#layout(...)', () => {

    it('should respect "options"', async () => {
      const graph = await elk.layout(simpleGraph, {
        layoutOptions: {
          'org.eclipse.elk.layered.spacing.nodeNodeBetweenLayers': 11
        }});
      expect(graph.children[0].y).toBe(graph.children[1].y);
      expect(Math.abs(graph.children[0].x - graph.children[1].x)).toBe(10 + 11);
    });

    it('should not override concrete layout options', async () => {
      const graph = await elk.layout(simpleGraph, {
        layoutOptions: {
          'org.eclipse.elk.direction': 'DOWN'
        }});
      expect(graph.layoutOptions['elk.direction']).toBe('RIGHT');
      expect(Math.abs(graph.children[0].x - graph.children[1].x)).toBeGreaterThan(0);
      expect(graph.children[0].y).toBe(graph.children[1].y);
    });

    it('should correctly parse ElkPadding', async () => {
      let paddingGraph = {
        id: "root",
        layoutOptions: { 'elk.padding': '[left=2, top=3, right=3, bottom=2]' },
        children: [ { id: "n1", width: 10, height: 10 } ]
      };
      const graph = await elk.layout(paddingGraph);
      expect(graph.children[0].x).toBe(2);
      expect(graph.children[0].y).toBe(3);
      expect(graph.width).toBe(15);
      expect(graph.height).toBe(15);
    });

    it('should correctly parse KVector', async () => {
      let kvectorGraph = {
        id: "root",
        children: [
          {
            id: "n1", width: 10, height: 10,
            layoutOptions: { position: "(23, 43)"}
          }
        ]
      };
      const graph = await elk.layout(kvectorGraph, {
        layoutOptions: {
          algorithm: 'fixed'
        }});
      expect(graph.children[0].x).toBe(23);
      expect(graph.children[0].y).toBe(43);
    });

    it('should correctly parse KVectorChain', async () => {
      let kvectorchainGraph = {
        id: "root",
        children: [
          { id: "n1", width: 10, height: 10 },
          { id: "n2", width: 10, height: 10 }
        ],
        edges: [{
          id: "e1",
          sources: [ "n1" ],
          targets: [ "n2" ],
          layoutOptions: { bendPoints: "( {1,2}, {3,4} )"}
        }]
      };
      const graph = await elk.layout(kvectorchainGraph, {
        layoutOptions: {
          algorithm: 'fixed'
        }});
      expect(graph.edges[0].sections[0].startPoint.x).toBe(1);
      expect(graph.edges[0].sections[0].startPoint.y).toBe(2);
      expect(graph.edges[0].sections[0].endPoint.x).toBe(3);
      expect(graph.edges[0].sections[0].endPoint.y).toBe(4);
    });

    it('should raise an exception for an invalid layouter id', async () => {
      let graph = {
        id: "root",
        children: [{ id: "n1", width: 10, height: 10 }],
        layoutOptions: { algorithm: "foo.bar.baz" }
      };
      await expect(elk.layout(graph)).rejects.toThrow(
        /org\.eclipse\.elk\.core\.UnsupportedConfigurationException/
      );
    });

    it('should default to elk.layered if no layouter has been specified', async () => {
      let graph = {
        id: "root",
        children: [{ id: "n1", width: 10, height: 10 }],
        layoutOptions: { }
      };
      await elk.layout(graph);
    });

  });
});

// Test default layout options

const secondElk = new ELK({
  defaultLayoutOptions: {
    'elk.layered.spacing.nodeNodeBetweenLayers': 33
  }
});

describe('Global Layout Options', () => {

  describe('#layout(...)', () => {

    it('should respect global layout"', async () => {
      const graph = await secondElk.layout(simpleGraph);
      expect(graph.children[0].y).toBe(graph.children[1].y);
      expect(Math.abs(graph.children[0].x - graph.children[1].x)).toBe(10 + 33);
    });

  });
});
