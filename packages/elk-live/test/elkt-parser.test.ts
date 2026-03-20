import { describe, it, expect } from 'vitest';
import { parseElkt, tokenize, TokenType } from '../src/elkt/parser';

describe('tokenize', () => {
  it('tokenizes simple elkt', () => {
    const tokens = tokenize('node n1');
    expect(tokens[0]).toMatchObject({ type: TokenType.KEYWORD, value: 'node' });
    expect(tokens[1]).toMatchObject({ type: TokenType.ID, value: 'n1' });
    expect(tokens[2]).toMatchObject({ type: TokenType.EOF });
  });

  it('tokenizes arrow and colon', () => {
    const tokens = tokenize('edge e1: n1 -> n2');
    expect(tokens.map(t => t.value)).toEqual(['edge', 'e1', ':', 'n1', '->', 'n2', '']);
  });

  it('tokenizes strings', () => {
    const tokens = tokenize('label "hello world"');
    expect(tokens[1]).toMatchObject({ type: TokenType.STRING, value: 'hello world' });
  });

  it('tokenizes qualified IDs with dots', () => {
    const tokens = tokenize('elk.algorithm: layered');
    expect(tokens[0]).toMatchObject({ type: TokenType.ID, value: 'elk.algorithm' });
  });

  it('skips line comments', () => {
    const tokens = tokenize('// comment\nnode n1');
    expect(tokens[0]).toMatchObject({ type: TokenType.KEYWORD, value: 'node' });
  });

  it('skips block comments', () => {
    const tokens = tokenize('/* block */ node n1');
    expect(tokens[0]).toMatchObject({ type: TokenType.KEYWORD, value: 'node' });
  });

  it('tokenizes numbers', () => {
    const tokens = tokenize('width: 30.5');
    expect(tokens[2]).toMatchObject({ type: TokenType.NUMBER, value: '30.5' });
  });

  it('tokenizes booleans', () => {
    const tokens = tokenize('true false');
    expect(tokens[0]).toMatchObject({ type: TokenType.BOOLEAN, value: 'true' });
    expect(tokens[1]).toMatchObject({ type: TokenType.BOOLEAN, value: 'false' });
  });
});

describe('parseElkt', () => {
  it('parses simple graph with nodes and edges', () => {
    const graph = parseElkt(`
      algorithm: layered
      node n1
      node n2
      edge n1 -> n2
    `);
    expect(graph.id).toBe('root');
    expect(graph.layoutOptions?.['algorithm']).toBe('layered');
    expect(graph.children).toHaveLength(2);
    expect(graph.children![0].id).toBe('n1');
    expect(graph.children![1].id).toBe('n2');
    expect(graph.edges).toHaveLength(1);
    expect(graph.edges![0].sources).toEqual(['n1']);
    expect(graph.edges![0].targets).toEqual(['n2']);
  });

  it('parses nested nodes', () => {
    const graph = parseElkt(`
      node parent {
        node child1
        node child2
        edge child1 -> child2
      }
    `);
    expect(graph.children).toHaveLength(1);
    const parent = graph.children![0];
    expect(parent.id).toBe('parent');
    expect(parent.children).toHaveLength(2);
    expect(parent.children![0].id).toBe('parent$child1');
    expect(parent.children![1].id).toBe('parent$child2');
    expect(parent.edges).toHaveLength(1);
    expect(parent.edges![0].sources).toEqual(['parent$child1']);
    expect(parent.edges![0].targets).toEqual(['parent$child2']);
  });

  it('parses ports', () => {
    const graph = parseElkt(`
      node n1 {
        port p1
        port p2
      }
    `);
    expect(graph.children![0].ports).toHaveLength(2);
    expect(graph.children![0].ports![0].id).toBe('n1$p1');
  });

  it('parses labels', () => {
    const graph = parseElkt(`
      node n1 {
        label "Node 1"
      }
    `);
    expect(graph.children![0].labels).toHaveLength(1);
    expect(graph.children![0].labels![0].text).toBe('Node 1');
  });

  it('parses edge with ID', () => {
    const graph = parseElkt(`
      node n1
      node n2
      edge e1: n1 -> n2
    `);
    expect(graph.edges![0].id).toBe('e1');
    expect(graph.edges![0].sources).toEqual(['n1']);
    expect(graph.edges![0].targets).toEqual(['n2']);
  });

  it('parses layout options on nodes', () => {
    const graph = parseElkt(`
      node n1 {
        elk.direction: RIGHT
        nodeLabels.placement: "H_LEFT V_TOP OUTSIDE"
      }
    `);
    const opts = graph.children![0].layoutOptions;
    expect(opts?.['elk.direction']).toBe('RIGHT');
    expect(opts?.['nodeLabels.placement']).toBe('H_LEFT V_TOP OUTSIDE');
  });

  it('parses width and height on nodes', () => {
    const graph = parseElkt(`
      node n1 {
        width: 50
        height: 30
      }
    `);
    expect(graph.children![0].width).toBe(50);
    expect(graph.children![0].height).toBe(30);
  });

  it('parses multi-source/target edges', () => {
    const graph = parseElkt(`
      node n1
      node n2
      node n3
      edge n1, n2 -> n3
    `);
    expect(graph.edges![0].sources).toEqual(['n1', 'n2']);
    expect(graph.edges![0].targets).toEqual(['n3']);
  });

  it('parses edge with labels', () => {
    const graph = parseElkt(`
      node n1
      node n2
      edge n1 -> n2 {
        label "my edge"
      }
    `);
    expect(graph.edges![0].labels).toHaveLength(1);
    expect(graph.edges![0].labels![0].text).toBe('my edge');
  });

  it('parses the direction example', () => {
    const graph = parseElkt(`
      node leftToRight {
        elk.direction: RIGHT
        nodeLabels.placement: "H_LEFT V_TOP OUTSIDE"
        node n1
        node n2
        edge n1 -> n2
        label "leftToRight"
      }
    `);
    expect(graph.children).toHaveLength(1);
    const ltr = graph.children![0];
    expect(ltr.id).toBe('leftToRight');
    expect(ltr.layoutOptions?.['elk.direction']).toBe('RIGHT');
    expect(ltr.children).toHaveLength(2);
    expect(ltr.children![0].id).toBe('leftToRight$n1');
    expect(ltr.children![1].id).toBe('leftToRight$n2');
    expect(ltr.edges).toHaveLength(1);
    expect(ltr.edges![0].sources).toEqual(['leftToRight$n1']);
    expect(ltr.edges![0].targets).toEqual(['leftToRight$n2']);
    expect(ltr.labels).toHaveLength(1);
    expect(ltr.labels![0].text).toBe('leftToRight');
  });

  it('generates unique IDs across sibling containers', () => {
    const graph = parseElkt(`
      node a {
        node n1
        node n2
        edge n1 -> n2
      }
      node b {
        node n1
        node n2
        edge n1 -> n2
      }
    `);
    const a = graph.children![0];
    const b = graph.children![1];
    expect(a.children![0].id).toBe('a$n1');
    expect(b.children![0].id).toBe('b$n1');
    expect(a.edges![0].sources).toEqual(['a$n1']);
    expect(b.edges![0].sources).toEqual(['b$n1']);
    // All IDs are globally unique
    const allIds = [
      ...a.children!.map(c => c.id),
      ...b.children!.map(c => c.id),
    ];
    expect(new Set(allIds).size).toBe(allIds.length);
  });
});
