/*******************************************************************************
 * Ported from elkjs — Copyright (c) 2021 Kiel University and others.
 * SPDX-License-Identifier: EPL-2.0
 *******************************************************************************/
import { describe, it, expect } from 'vitest';
import ELK from '../js/index.js';

const elk = new ELK();

describe('elkjs#8', () => {
  describe('#layout()', () => {

    it('should not add edge sections for simple bottom-up layout ', async () => {
      await expect(elk.layout(graph, {
        layoutOptions: { 'hierarchyHandling': 'SEPARATE_CHILDREN'}
      })).rejects.toThrow(
        /org\.eclipse\.elk\.core\.UnsupportedGraphException/
      );
    });

    it('should not add edge sections for simple bottom-up layout (primitive edge format)', async () => {
      await expect(elk.layout(graphPrimitiveEdgeFormat, {
        layoutOptions: { 'hierarchyHandling': 'SEPARATE_CHILDREN'}
      })).rejects.toThrow(
        /org\.eclipse\.elk\.core\.UnsupportedGraphException/
      );
    });

    it('should add edge sections for hierarchical layout', async () => {
      const result = await elk.layout(graph, {
        layoutOptions: { 'hierarchyHandling': 'INCLUDE_CHILDREN'}
      });
      const edgeSections = result.children[0].edges[0].sections;
      expect(edgeSections).toBeDefined();
      expect(Array.isArray(edgeSections)).toBe(true);
      expect(edgeSections).toHaveLength(1);
      const firstSection = edgeSections[0];
      expect(firstSection).toHaveProperty('startPoint');
      expect(firstSection).toHaveProperty('endPoint');
    });

    it('should add edge sections for hierarchical layout (primitive edge format)', async () => {
      const result = await elk.layout(graphPrimitiveEdgeFormat, {
        layoutOptions: { 'hierarchyHandling': 'INCLUDE_CHILDREN'}
      });
      const edgeSections = result.children[0].edges[0].sections;
      expect(edgeSections).toBeDefined();
      expect(Array.isArray(edgeSections)).toBe(true);
      expect(edgeSections).toHaveLength(1);
      const firstSection = edgeSections[0];
      expect(firstSection).toHaveProperty('startPoint');
      expect(firstSection).toHaveProperty('endPoint');
    });

  });
});

const graph = {
  "id": "root",
  "children": [
    {
      "id": "A",
      "children": [
        { "id": "a1" },
        { "id": "a2" },
        { "id": "$generated_A_initial_0" }
      ],
      "edges": [ { "id": "a1:0", "sources": [ "a1" ], "targets": [ "A" ] } ],
    },
    { "id": "$generated_root_initial_0" }
  ]
};

const graphPrimitiveEdgeFormat = {
  "id": "root",
  "children": [
    {
      "id": "A",
      "children": [
        { "id": "a1" },
        { "id": "a2" },
        { "id": "$generated_A_initial_0" }
      ],
      "edges": [ { "id": "a1:0", "source": "a1", "target": "A" } ],
    },
    { "id": "$generated_root_initial_0" }
  ]
};
