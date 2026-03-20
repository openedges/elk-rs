import type { ElkNode } from './elk-types';

// Dynamic import of the WASM glue module — only the layout function is needed
let layoutJsonFn: ((graphJson: string, optionsJson: string) => string) | null = null;

let initPromise: Promise<void> | null = null;

export async function initElk(): Promise<void> {
  if (layoutJsonFn) return;
  if (initPromise) return initPromise;

  initPromise = (async () => {
    const mod = await import('../wasm/org_eclipse_elk_wasm.js');
    await mod.default();
    layoutJsonFn = mod.layout_json;
  })();

  return initPromise;
}

export async function layoutGraph(graph: ElkNode): Promise<ElkNode> {
  await initElk();
  const graphJson = JSON.stringify(graph);
  const resultJson = layoutJsonFn!(graphJson, '{}');
  return JSON.parse(resultJson);
}
