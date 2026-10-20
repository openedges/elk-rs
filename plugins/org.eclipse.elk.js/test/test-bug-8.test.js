/*******************************************************************************
 * Ported from elkjs — Copyright (c) 2021 Kiel University and others.
 *
 * This program and the accompanying materials are made available under the
 * terms of the Eclipse Public License 2.0, https://www.eclipse.org/legal/epl-2.0.
 * This Source Code may also be made available under the following Secondary
 * Licenses when the conditions for such availability set forth in the Eclipse
 * Public License v. 2.0 are satisfied: GPL-3.0, https://www.gnu.org/licenses/gpl-3.0-standalone.html.
 * SPDX-License-Identifier: EPL-2.0 OR GPL-3.0-or-later
 *******************************************************************************/
import { describe, it, expect } from 'vitest';
import ELK from '../js/index.js';

const elk = new ELK();

describe('elkjs#8', () => {
  describe('#layout()', () => {

    // elk-rs handles cross-hierarchy edges gracefully instead of throwing
    // UnsupportedGraphException (unlike elkjs). Layout succeeds.
    it('should handle simple bottom-up layout with cross-hierarchy edges', async () => {
      const result = await elk.layout(graph, {
        layoutOptions: { 'hierarchyHandling': 'SEPARATE_CHILDREN'}
      });
      expect(result).toBeDefined();
      expect(result.children).toBeDefined();
    });

    it('should handle simple bottom-up layout with cross-hierarchy edges (primitive edge format)', async () => {
      const result = await elk.layout(graphPrimitiveEdgeFormat, {
        layoutOptions: { 'hierarchyHandling': 'SEPARATE_CHILDREN'}
      });
      expect(result).toBeDefined();
      expect(result.children).toBeDefined();
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
