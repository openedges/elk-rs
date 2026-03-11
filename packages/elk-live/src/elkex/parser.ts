/**
 * Parser for .elkt example files with elkex annotations.
 *
 * Example format:
 *   // elkex:category
 *   // General > Direction
 *   // elkex:label
 *   // Basics
 *   // elkex:doc
 *   // Description here
 *   // elkex:graph
 *   node n1 { ... }
 */

export interface ElkExample {
  path: string;
  label: string;
  category: string[];
  doc: string;
  graph: string;
}

export interface ExampleCategory {
  name: string;
  elements: ElkExample[];
  subCategories: ExampleCategory[];
}

const KEYS = new Set(['graph', 'doc', 'category', 'label']);
const LEAF_CATEGORY = '_leaf_';

export function parseElkExample(path: string, source: string): ElkExample {
  const sections = source.split(/\/\/\s*elkex:/);
  const result: Partial<ElkExample> = { path };

  for (const section of sections) {
    const trimmed = section.trim();
    if (!trimmed) continue;

    const firstSpace = trimmed.search(/\s/);
    if (firstSpace < 0) continue;

    const key = trimmed.substring(0, firstSpace);
    if (!KEYS.has(key)) continue;

    const firstNewline = trimmed.search(/(\r|\n)/);
    let content = firstNewline >= 0 ? trimmed.substring(firstNewline + 1) : '';

    if (key !== 'graph') {
      // Remove comment markers
      content = content.replace(/^[ \t]*\/\//gm, '')
        .replace(/\/\*/g, '')
        .replace(/\*\//g, '');
    }
    content = content.trim();

    if (key === 'category') {
      (result as Record<string, unknown>).category = content.split('>').map(s => s.trim());
    } else {
      (result as Record<string, unknown>)[key] = content;
    }
  }

  if (!result.label || !result.graph || !result.doc || !result.category) {
    throw new Error(`Example '${path}' is missing required fields: ${
      ['label', 'graph', 'doc', 'category'].filter(k => !(result as Record<string, unknown>)[k]).join(', ')
    }`);
  }

  return result as ElkExample;
}

export function buildCategoryTree(examples: ElkExample[], depth = 0, name = 'root'): ExampleCategory {
  const grouped: Record<string, ElkExample[]> = {};
  for (const ex of examples) {
    const key = ex.category[depth] || LEAF_CATEGORY;
    if (!grouped[key]) grouped[key] = [];
    grouped[key].push(ex);
  }

  const subCategories = Object.keys(grouped)
    .filter(k => k !== LEAF_CATEGORY)
    .sort()
    .map(k => buildCategoryTree(grouped[k], depth + 1, k));

  const elements = (grouped[LEAF_CATEGORY] || []).sort((a, b) => a.label.localeCompare(b.label));

  return { name, elements, subCategories };
}
