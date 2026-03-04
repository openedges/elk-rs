#!/usr/bin/env node
/**
 * JS Performance Benchmark for elkjs, NAPI, and WASM engines.
 *
 * Usage: node bench/bench.mjs [options]
 *   --mode synthetic|models    Benchmark mode (default: synthetic)
 *   --engines elkjs,napi,wasm  Engines to measure (default: elkjs,napi,wasm)
 *   --iterations N             Iterations per scenario (default: 20)
 *   --warmup N                 Warmup iterations (default: 3)
 *   --output PATH              CSV output path (default: stdout)
 *   --models-dir PATH          Model JSON directory (models mode)
 *   --manifest PATH            Java manifest TSV (models mode)
 *   --limit N                  Max models (models mode, default: 50)
 *
 * CSV output format:
 *   timestamp,engine,scenario,iterations,warmup,elapsed_nanos,avg_ms,ops_per_sec
 *
 * Notes:
 *   - elkjs is async (Promise-based); NAPI/WASM are synchronous.
 *   - elkjs mutates the input graph, so each iteration receives a fresh clone.
 *   - NAPI/WASM take a JSON string, so a pre-serialized string is reused.
 *   - WASM module initialization cost is absorbed during warmup.
 */
import { createRequire } from 'node:module';
import { dirname, join, relative, basename } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  readFileSync, writeFileSync, existsSync, mkdirSync,
  readdirSync, statSync,
} from 'node:fs';

const require = createRequire(import.meta.url);
const __dirname = fileURLToPath(new URL('.', import.meta.url));
const PKG_ROOT = join(__dirname, '..');
const REPO_ROOT = join(PKG_ROOT, '../..');

// ---------------------------------------------------------------------------
// CLI argument parsing
// ---------------------------------------------------------------------------
const args = process.argv.slice(2);

function getArg(name, fallback) {
  const idx = args.indexOf(name);
  return idx === -1 ? fallback : args[idx + 1];
}

const MODE = getArg('--mode', 'synthetic');
const ENGINES = getArg('--engines', 'elkjs,napi,wasm').split(',').map(s => s.trim()).filter(Boolean);
const ITERATIONS = Math.max(1, parseInt(getArg('--iterations', '20'), 10));
const WARMUP = Math.max(0, parseInt(getArg('--warmup', '3'), 10));
const OUTPUT = getArg('--output', null);
const MODELS_DIR = getArg('--models-dir', join(REPO_ROOT, 'parity/model_parity/java/input'));
const MANIFEST = getArg('--manifest', join(REPO_ROOT, 'parity/model_parity/java/java_manifest.tsv'));
const LIMIT = parseInt(getArg('--limit', '50'), 10);

// ---------------------------------------------------------------------------
// Scenario loading
// ---------------------------------------------------------------------------
import { allScenarios } from './scenarios.mjs';

// ---------------------------------------------------------------------------
// Engine loaders
// ---------------------------------------------------------------------------

async function loadEngine(name) {
  switch (name) {
    case 'elkjs': {
      const ELK = require('elkjs');
      const elk = new ELK();
      return {
        name: 'elkjs',
        isAsync: true,
        layout(graphJson) {
          // elkjs works with JS objects and mutates in place, so deep-clone each call.
          const graph = JSON.parse(graphJson);
          return elk.layout(graph).then(result => {
            JSON.stringify(result);  // match rust_api/napi/wasm serialize scope
            return result;
          });
        },
      };
    }
    case 'napi': {
      try {
        const napi = require(join(PKG_ROOT, 'dist/elk-rs.node'));
        return {
          name: 'napi',
          isAsync: false,
          layout(graphJson) {
            return napi.layout_json(graphJson, '{}');
          },
        };
      } catch (e) {
        console.error(`[warn] NAPI engine not available: ${e.message}`);
        return null;
      }
    }
    case 'wasm': {
      try {
        const wasmJsPath = join(PKG_ROOT, 'dist/wasm/org_eclipse_elk_wasm.js');
        const wasmBinPath = join(PKG_ROOT, 'dist/wasm/org_eclipse_elk_wasm_bg.wasm');
        if (!existsSync(wasmBinPath)) {
          console.error(`[warn] WASM binary not found: ${wasmBinPath}`);
          return null;
        }
        const wasmMod = await import(wasmJsPath);
        const wasmBytes = readFileSync(wasmBinPath);
        wasmMod.initSync({ module: wasmBytes });
        return {
          name: 'wasm',
          isAsync: false,
          layout(graphJson) {
            return wasmMod.layout_json(graphJson, '{}');
          },
        };
      } catch (e) {
        console.error(`[warn] WASM engine not available: ${e.message}`);
        return null;
      }
    }
    default:
      console.error(`[warn] Unknown engine: ${name}`);
      return null;
  }
}

// ---------------------------------------------------------------------------
// CSV output
// ---------------------------------------------------------------------------
const CSV_HEADER = 'timestamp,engine,scenario,iterations,warmup,elapsed_nanos,avg_ms,ops_per_sec';
const csvRows = [];

function appendResult(engine, scenario, iterations, warmup, elapsedNanos, avgMs, opsPerSec) {
  const timestamp = Math.floor(Date.now() / 1000);
  csvRows.push(
    `${timestamp},${engine},${scenario},${iterations},${warmup},${elapsedNanos},${avgMs.toFixed(6)},${opsPerSec.toFixed(2)}`
  );
}

function writeCsv() {
  const csv = [CSV_HEADER, ...csvRows].join('\n') + '\n';
  if (OUTPUT) {
    mkdirSync(dirname(OUTPUT), { recursive: true });
    writeFileSync(OUTPUT, csv);
    console.error(`Results written to ${OUTPUT}`);
  } else {
    process.stdout.write(csv);
  }
}

// ---------------------------------------------------------------------------
// Benchmark core
// ---------------------------------------------------------------------------

async function benchmarkScenario(engine, scenarioName, graphJson, iterations, warmup) {
  // Warmup
  for (let i = 0; i < warmup; i++) {
    if (engine.isAsync) {
      await engine.layout(graphJson);
    } else {
      engine.layout(graphJson);
    }
  }

  // Timed iterations
  const start = process.hrtime.bigint();
  for (let i = 0; i < iterations; i++) {
    if (engine.isAsync) {
      await engine.layout(graphJson);
    } else {
      engine.layout(graphJson);
    }
  }
  const elapsed = Number(process.hrtime.bigint() - start);
  const avgMs = elapsed / iterations / 1_000_000;
  const opsPerSec = iterations / (elapsed / 1_000_000_000);

  return { elapsedNanos: elapsed, avgMs, opsPerSec };
}

// ---------------------------------------------------------------------------
// Model loading (for --mode models)
// ---------------------------------------------------------------------------

function loadModelsFromManifest() {
  if (!existsSync(MANIFEST)) {
    console.error(`Manifest not found: ${MANIFEST}`);
    return [];
  }
  const lines = readFileSync(MANIFEST, 'utf-8').trim().split('\n');
  const models = [];
  for (const line of lines) {
    const cols = line.split('\t');
    if (cols.length < 4) continue;
    const [modelRel, inputJsonPath, , status] = cols;
    if (status !== 'ok') continue;
    if (!existsSync(inputJsonPath)) continue;
    try {
      const json = readFileSync(inputJsonPath, 'utf-8');
      JSON.parse(json); // validate
      const name = modelRel.replace(/\.[^.]+$/, '').replace(/[/\\]/g, '_');
      models.push({ name, json });
    } catch { /* skip invalid */ }
    if (LIMIT > 0 && models.length >= LIMIT) break;
  }
  return models;
}

function loadModelsFromDirectory() {
  if (!existsSync(MODELS_DIR)) {
    console.error(`Models directory not found: ${MODELS_DIR}`);
    return [];
  }
  const models = [];
  const files = findJsonFiles(MODELS_DIR);
  for (const file of files) {
    try {
      const json = readFileSync(file, 'utf-8');
      JSON.parse(json); // validate
      const name = relative(MODELS_DIR, file).replace(/\.json$/, '').replace(/[/\\]/g, '_');
      models.push({ name, json });
    } catch { /* skip invalid */ }
    if (LIMIT > 0 && models.length >= LIMIT) break;
  }
  return models;
}

function findJsonFiles(dir) {
  const results = [];
  try {
    for (const entry of readdirSync(dir)) {
      const full = join(dir, entry);
      try {
        const stat = statSync(full);
        if (stat.isDirectory()) {
          results.push(...findJsonFiles(full));
        } else if (entry.endsWith('.json') && !entry.startsWith('.')) {
          results.push(full);
        }
      } catch { /* skip */ }
    }
  } catch { /* skip */ }
  return results;
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main() {
  console.error(`JS Performance Benchmark`);
  console.error(`  Mode: ${MODE}`);
  console.error(`  Engines: ${ENGINES.join(', ')}`);
  console.error(`  Iterations: ${ITERATIONS}, Warmup: ${WARMUP}`);
  console.error('');

  // Load engines
  const loadedEngines = [];
  for (const name of ENGINES) {
    const engine = await loadEngine(name);
    if (engine) loadedEngines.push(engine);
  }

  if (loadedEngines.length === 0) {
    console.error('No engines available. Build NAPI/WASM first: sh build.sh');
    process.exit(1);
  }

  console.error(`Loaded engines: ${loadedEngines.map(e => e.name).join(', ')}`);
  console.error('');

  if (MODE === 'synthetic') {
    const scenarios = allScenarios();
    for (const engine of loadedEngines) {
      console.error(`[${engine.name}]`);
      for (const [name, graph] of scenarios) {
        const graphJson = JSON.stringify(graph);
        try {
          const { elapsedNanos, avgMs, opsPerSec } = await benchmarkScenario(
            engine, name, graphJson, ITERATIONS, WARMUP,
          );
          appendResult(engine.name, name, ITERATIONS, WARMUP, elapsedNanos, avgMs, opsPerSec);
          console.error(`  ${name}: ${avgMs.toFixed(4)} ms/op, ${opsPerSec.toFixed(0)} ops/s`);
        } catch (e) {
          console.error(`  ${name}: ERROR — ${e.message || e}`);
        }
      }
      console.error('');
    }
  } else if (MODE === 'models') {
    // Load models from manifest (preferred) or directory
    let models;
    if (existsSync(MANIFEST)) {
      console.error(`Loading models from manifest: ${MANIFEST}`);
      models = loadModelsFromManifest();
    } else {
      console.error(`Loading models from directory: ${MODELS_DIR}`);
      models = loadModelsFromDirectory();
    }

    if (models.length === 0) {
      console.error('No models found. Run Java export first or check paths.');
      process.exit(1);
    }

    console.error(`Loaded ${models.length} models`);
    console.error('');

    for (const engine of loadedEngines) {
      console.error(`[${engine.name}]`);
      let ok = 0, errors = 0;
      for (const { name, json } of models) {
        try {
          const { elapsedNanos, avgMs, opsPerSec } = await benchmarkScenario(
            engine, name, json, ITERATIONS, WARMUP,
          );
          appendResult(engine.name, name, ITERATIONS, WARMUP, elapsedNanos, avgMs, opsPerSec);
          ok++;
          if (ok <= 5 || ok % 10 === 0) {
            console.error(`  ${name}: ${avgMs.toFixed(4)} ms/op`);
          }
        } catch (e) {
          errors++;
          if (errors <= 3) {
            console.error(`  ${name}: ERROR — ${(e.message || String(e)).slice(0, 120)}`);
          }
        }
      }
      console.error(`  Done: ${ok} ok, ${errors} errors`);
      console.error('');
    }
  } else {
    console.error(`Unknown mode: ${MODE}. Use 'synthetic' or 'models'.`);
    process.exit(1);
  }

  writeCsv();
}

main().catch(err => {
  console.error('Fatal error:', err);
  process.exit(2);
});
