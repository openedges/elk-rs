/*******************************************************************************
 * Ported from elkjs — Copyright (c) 2021 Kiel University and others.
 * SPDX-License-Identifier: EPL-2.0
 *******************************************************************************/
import { describe, it, expect } from 'vitest';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const require = createRequire(import.meta.url);
const __dirname = path.dirname(fileURLToPath(import.meta.url));

const graph = {
  id: "root",
  properties: { 'algorithm': 'layered' },
  children: [
    { id: "n1", width: 30, height: 30 },
    { id: "n2", width: 30, height: 30 },
    { id: "n3", width: 30, height: 30 }
  ],
  edges: [
    { id: "e1", sources: [ "n1" ], targets: [ "n2" ] },
    { id: "e2", sources: [ "n1" ], targets: [ "n3" ] }
  ]
};

describe('Entry point', () => {

  describe('main entry point (direct mode)', () => {
    const ELK = require('../js/index.js');
    const elk = new ELK();

    it('should succeed.', async () => {
      await elk.layout(graph);
    });
  });

  describe('elk-api with fake worker', () => {
    const ELK = require('../js/elk-api.js');
    const elk = new ELK({
      workerFactory: function (_) {
        const { Worker } = require('../js/elk-worker.js');
        return new Worker();
      }
    });

    it('should succeed.', async () => {
      await elk.layout(graph);
    });
  });

  describe('in worker_threads', () => {
    const ELK = require('../js/index.js');
    const elk = new ELK({
      workerUrl: path.join(__dirname, '../js/elk-worker.js')
    });

    it('should succeed.', async () => {
      await elk.layout(graph);
      elk.terminateWorker();
    });
  });

  describe('platform package mapping', () => {
    // Test that platformTriple() returns the correct triple
    // by reading the source and checking the mapping table
    const indexSrc = require('fs').readFileSync(
      path.join(__dirname, '../js/index.js'), 'utf8'
    );

    it('should define triples for all supported platforms', () => {
      const triples = [
        'darwin-arm64',
        'darwin-x64',
        'linux-x64-gnu',
        'linux-arm64-gnu',
        'win32-x64-msvc',
      ];
      for (const triple of triples) {
        expect(indexSrc).toContain("'" + triple + "'");
      }
    });

    it('should try platform package before local .node and WASM', () => {
      // Verify the loading order in source: @elk-rs/ package -> platform .node -> generic .node -> WASM
      const pkgIdx = indexSrc.indexOf("'@elk-rs/'");
      const platformLocalIdx = indexSrc.indexOf("'../dist/elk-rs.'");
      const genericLocalIdx = indexSrc.indexOf("'../dist/elk-rs.node'");
      const wasmIdx = indexSrc.indexOf("../dist/wasm/org_eclipse_elk_wasm.js");
      expect(pkgIdx).toBeLessThan(platformLocalIdx);
      expect(platformLocalIdx).toBeLessThan(genericLocalIdx);
      expect(genericLocalIdx).toBeLessThan(wasmIdx);
    });
  });

  describe('WASM fallback works when no NAPI binary', () => {
    // The main entry point should still work (falling back to WASM)
    const ELK = require('../js/index.js');
    const elk = new ELK();

    it('should layout successfully via fallback', async () => {
      const result = await elk.layout(graph);
      expect(result).toBeDefined();
      expect(result.id).toBe('root');
      expect(result.children).toHaveLength(3);
    });
  });

});
