/**
 * End-to-end test: parse all elk-models examples with our ELKT parser,
 * then run NAPI layout, then compare node positions/sizes with model parity reference.
 *
 * Uses the NAPI module directly (same layout engine as WASM).
 */
import { describe, it, expect } from 'vitest';
import { parseElkt } from '../src/elkt/parser';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import type { ElkNode } from '../src/elk/elk-types';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Use NAPI module for layout (same engine as WASM) — CJS module requires createRequire
const require = createRequire(import.meta.url);
const ELK = require(path.resolve(__dirname, '../../../plugins/org.eclipse.elk.js/js/index.js'));

const examplesDir = path.resolve(__dirname, '../../../external/elk-models/examples');
const parityRefDir = path.resolve(__dirname, '../../../tests/model_parity_full/rust/layout/examples');

function findElkt(dir: string): string[] {
  let results: string[] = [];
  for (const f of fs.readdirSync(dir)) {
    const full = path.join(dir, f);
    if (fs.statSync(full).isDirectory()) results = results.concat(findElkt(full));
    else if (f.endsWith('.elkt')) results.push(full);
  }
  return results;
}

function extractGraph(content: string): string {
  const parts = content.split(/\/\/\s*elkex:graph/);
  return parts.length > 1 ? parts[1].trim() : content;
}

/** Compare layout output: check node positions and sizes match reference */
function compareLaidOut(our: ElkNode, ref: ElkNode, nodePath: string): string[] {
  const diffs: string[] = [];
  if (!our || !ref) return diffs;

  const ourChildren = our.children || [];
  const refChildren = ref.children || [];
  if (ourChildren.length !== refChildren.length) {
    diffs.push(`${nodePath} children: ${ourChildren.length} vs ${refChildren.length}`);
    return diffs; // Can't compare further
  }

  for (let i = 0; i < refChildren.length; i++) {
    const oc = ourChildren[i], rc = refChildren[i];
    const name = `${nodePath}/${rc.id}`;
    // Compare layout results (with tolerance)
    if (Math.abs((oc.width || 0) - (rc.width || 0)) > 1) diffs.push(`${name} w: ${oc.width} vs ${rc.width}`);
    if (Math.abs((oc.height || 0) - (rc.height || 0)) > 1) diffs.push(`${name} h: ${oc.height} vs ${rc.height}`);
    if (Math.abs((oc.x || 0) - (rc.x || 0)) > 1) diffs.push(`${name} x: ${oc.x} vs ${rc.x}`);
    if (Math.abs((oc.y || 0) - (rc.y || 0)) > 1) diffs.push(`${name} y: ${oc.y} vs ${rc.y}`);
    diffs.push(...compareLaidOut(oc, rc, name));
  }
  return diffs;
}

const elk = new ELK();
const allFiles = findElkt(examplesDir);

describe('ELKT parser → NAPI layout matches reference', () => {
  for (const file of allFiles) {
    const rel = file.replace(examplesDir + '/', '');
    const refPath = path.join(parityRefDir, rel + '.json');
    if (!fs.existsSync(refPath)) continue;

    it(rel, async () => {
      const content = fs.readFileSync(file, 'utf8');
      const graphText = extractGraph(content);
      const graph = parseElkt(graphText);

      // Set edgeCoords PARENT for correct edge coordinates
      if (!graph.properties) graph.properties = {};
      graph.properties['org.eclipse.elk.json.edgeCoords'] = 'PARENT';

      const result = await elk.layout(graph);
      const ref = JSON.parse(fs.readFileSync(refPath, 'utf8'));
      const diffs = compareLaidOut(result, ref, 'root');

      if (diffs.length > 0) {
        console.log(`  ${diffs.length} diffs:`, diffs.slice(0, 5).join('; '));
      }
      expect(diffs).toEqual([]);
    });
  }
});
