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
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { cpSync, existsSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { spawnSync } from 'node:child_process';

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
      // Verify the loading order in source: @openedges/elk-layout-* package -> platform .node -> generic .node -> WASM
      const pkgIdx = indexSrc.indexOf("'@openedges/elk-layout-'");
      const platformLocalIdx = indexSrc.indexOf("'../dist/elk-layout.'");
      const genericLocalIdx = indexSrc.indexOf("'../dist/elk-layout.node'");
      const wasmIdx = indexSrc.indexOf("./load-wasm.js");
      expect(pkgIdx).toBeLessThan(platformLocalIdx);
      expect(platformLocalIdx).toBeLessThan(genericLocalIdx);
      expect(genericLocalIdx).toBeLessThan(wasmIdx);
    });
  });

  describe('WASM fallback works when no NAPI binary', () => {
    it('initializes the release WASM in direct, fake-worker, and worker_threads modes', () => {
      // Copy only JS and WASM outside the repo so no optional native package or
      // local .node file can satisfy the loader accidentally.
      const isolated = mkdtempSync(path.join(tmpdir(), 'elk-layout-wasm-fallback-'));
      try {
        cpSync(path.join(__dirname, '../js'), path.join(isolated, 'js'), { recursive: true });
        cpSync(path.join(__dirname, '../dist/wasm'), path.join(isolated, 'dist/wasm'), { recursive: true });
        if (existsSync(path.join(__dirname, '../dist/wasm-node'))) {
          cpSync(path.join(__dirname, '../dist/wasm-node'), path.join(isolated, 'dist/wasm-node'), { recursive: true });
        }
        const script = `
          const assert = require('node:assert/strict');
          const ELK = require('./js/index.js');
          const API = require('./js/elk-api.js');
          const FakeWorker = require('./js/elk-worker.js').Worker;
          const path = require('node:path');
          const graph = ${JSON.stringify(graph)};
          (async () => {
            for (const elk of [new ELK(), new API({ workerFactory: () => new FakeWorker() }),
              new ELK({ workerUrl: path.resolve('./js/elk-worker.js') })]) {
              try {
                const output = await elk.layout(graph);
                assert.equal(output.children.length, 3);
                assert(output.children.every(node => Number.isFinite(node.x) && Number.isFinite(node.y)));
              } finally { elk.terminateWorker(); }
            }
          })().catch(error => { console.error(error); process.exitCode = 1; });
        `;
        const result = spawnSync(process.execPath, ['-e', script], {
          cwd: isolated, encoding: 'utf8', timeout: 20000,
        });
        expect(result.error, result.stderr).toBeUndefined();
        expect(result.status, result.stderr).toBe(0);
      } finally {
        rmSync(isolated, { recursive: true, force: true });
      }
    }, 30000);
  });

});
