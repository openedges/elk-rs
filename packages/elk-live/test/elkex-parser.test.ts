import { describe, it, expect } from 'vitest';
import { parseElkExample, buildCategoryTree } from '../src/elkex/parser';

const SAMPLE_EXAMPLE = `/*
// elkex:category
// General > Direction

// elkex:label
// Basics

// elkex:doc
// The example illustrates how different layout directions can be set.
*/

// elkex:graph
node leftToRight {
    elk.direction: RIGHT
    nodeLabels.placement: "H_LEFT V_TOP OUTSIDE"
    node n1
    node n2
    edge n1->n2
    label "leftToRight"
}`;

describe('parseElkExample', () => {
  it('parses a standard example file', () => {
    const ex = parseElkExample('general/direction', SAMPLE_EXAMPLE);
    expect(ex.path).toBe('general/direction');
    expect(ex.label).toBe('Basics');
    expect(ex.category).toEqual(['General', 'Direction']);
    expect(ex.doc).toContain('layout directions');
    expect(ex.graph).toContain('node leftToRight');
  });

  it('throws for missing fields', () => {
    expect(() => parseElkExample('test', '// elkex:graph\nnode n1')).toThrow('missing');
  });
});

describe('buildCategoryTree', () => {
  it('builds a category tree', () => {
    const ex1 = parseElkExample('general/direction', SAMPLE_EXAMPLE);
    const ex2 = { ...ex1, path: 'general/spacing', label: 'Spacing', category: ['General', 'Spacing'] };
    const ex3 = { ...ex1, path: 'ports/sides', label: 'Port Sides', category: ['Ports'] };

    const tree = buildCategoryTree([ex1, ex2, ex3]);
    expect(tree.name).toBe('root');
    expect(tree.subCategories).toHaveLength(2);

    const general = tree.subCategories.find(c => c.name === 'General');
    expect(general).toBeDefined();
    expect(general!.subCategories).toHaveLength(2);
  });
});
