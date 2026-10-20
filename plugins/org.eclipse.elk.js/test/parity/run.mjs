#!/usr/bin/env node
/**
 * Compare elk-rs JS, elkjs (GWT), and an independently exported Java baseline.
 * Use a fresh Java export for the target ELK version; --require-java prevents
 * a missing baseline from being reported as successful three-way parity.
 *
 * node test/parity/run.mjs --models <dir> --java-baseline <dir>/java
 *   [--require-java] [--backend <local NAPI/WASM module>] [--output <dir>]
 */
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { join, relative, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { createRequire } from 'node:module';
import { readdirSync } from 'node:fs';
import { classifyResults, parseJavaManifest, stripInternal, usesExtraAlgorithm } from './compare.mjs';

const require = createRequire(import.meta.url);
const PKG_ROOT = fileURLToPath(new URL('../..', import.meta.url));
const REPO_ROOT = resolve(PKG_ROOT, '../..');
const args = process.argv.slice(2);
function getArg(name, fallback) {
  const idx = args.indexOf(name);
  return idx === -1 ? fallback : args[idx + 1];
}
const MODELS_DIR = resolve(getArg('--models', join(REPO_ROOT, 'external/elk-models')));
const JAVA_BASELINE_DIR = resolve(getArg('--java-baseline', join(REPO_ROOT, 'tests/model_parity_full/java')));
const REPORT_DIR = resolve(getArg('--output', join(PKG_ROOT, 'test/parity/results')));
const ABS_TOL = Number(getArg('--tolerance', '1e-6'));
const REQUIRE_JAVA = args.includes('--require-java');
const BACKEND_PATH = getArg('--backend', null);

function findJsonFiles(dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap(entry => {
    const path = join(dir, entry.name);
    return entry.isDirectory() ? findJsonFiles(path)
      : entry.name.endsWith('.json') && !entry.name.startsWith('.') ? [path] : [];
  }).sort();
}

async function runParity() {
  if (!Number.isFinite(ABS_TOL) || ABS_TOL < 0) throw new Error('Invalid --tolerance');
  const ELKjs = require('elkjs');
  const jsEngine = new ELKjs();
  let rsEngine;
  if (BACKEND_PATH) {
    // An explicit local backend avoids inadvertently verifying an older installed platform package.
    const absoluteBackend = resolve(BACKEND_PATH);
    const loaded = absoluteBackend.endsWith('.node') ? require(absoluteBackend)
      : await import(pathToFileURL(absoluteBackend).href);
    const backend = typeof loaded.layout_json === 'function' ? loaded : loaded.default;
    if (typeof backend.initSync === 'function') {
      backend.initSync({ module: await readFile(absoluteBackend.replace(/\.js$/, '_bg.wasm')) });
    }
    const ELKApi = require(join(PKG_ROOT, 'js/elk-api.js'));
    rsEngine = new ELKApi({ backend });
  } else {
    const ELKrs = require(join(PKG_ROOT, 'js/index.js'));
    rsEngine = new ELKrs();
  }
  let javaManifest = new Map();
  try {
    javaManifest = parseJavaManifest(await readFile(join(JAVA_BASELINE_DIR, 'java_manifest.tsv'), 'utf8'));
  } catch (error) {
    if (REQUIRE_JAVA) throw error;
  }
  const jsIds = new Set((await jsEngine.knownLayoutAlgorithms()).map(algorithm => algorithm.id));
  const extraIds = new Set((await rsEngine.knownLayoutAlgorithms()).map(algorithm => algorithm.id).filter(id => !jsIds.has(id)));
  const modelFiles = findJsonFiles(MODELS_DIR);
  if (!modelFiles.length) throw new Error(`No model files found in ${MODELS_DIR}`);
  console.log(`JS 3-way parity: ${modelFiles.length} models; elkjs ${require('elkjs/package.json').version}`);
  console.log(`Java baseline: ${JAVA_BASELINE_DIR} (${javaManifest.size} entries)`);
  console.log(`Rust backend: ${BACKEND_PATH ? resolve(BACKEND_PATH) : 'automatic native/WASM selection'}`);
  console.log(`Rust extra algorithms: ${[...extraIds].join(', ')}`);

  const stats = {};
  const details = [];
  let javaCompared = 0, javaMatches = 0, missingJava = 0, failures = 0, processed = 0;
  try {
    for (const filePath of modelFiles) {
      const model = relative(MODELS_DIR, filePath).split('\\').join('/');
      const graph = JSON.parse(await readFile(filePath, 'utf8'));
      const results = { extraCapability: usesExtraAlgorithm(graph, extraIds) };
      const javaPath = javaManifest.get(model);
      if (javaPath) {
        try {
          results.java = stripInternal(JSON.parse(await readFile(resolve(JAVA_BASELINE_DIR, javaPath), 'utf8')));
        } catch (error) {
          throw new Error(`Unreadable Java baseline for ${model}: ${error.message}`);
        }
      }
      try { results.elkjs = stripInternal(await jsEngine.layout(JSON.parse(JSON.stringify(graph)))); }
      catch (error) { results.elkjsError = error.message || String(error); }
      try { results.rust = stripInternal(await rsEngine.layout(JSON.parse(JSON.stringify(graph)))); }
      catch (error) { results.rustError = error.message || String(error); }
      const result = classifyResults(results, ABS_TOL);
      processed++;
      stats[result.classification] = (stats[result.classification] || 0) + 1;
      if (result.hasJava) javaCompared++;
      else missingJava++;
      if (result.rustMatchesJava) javaMatches++;
      if (result.failure || (REQUIRE_JAVA && !result.hasJava)) failures++;
      if (result.classification !== 'PASS') {
        details.push({ model, classification: result.classification,
          rustError: results.rustError, elkjsError: results.elkjsError,
          sampleDiffs: (result.rustJavaDiffs.length ? result.rustJavaDiffs : result.engineDiffs).slice(0, 10) });
        console.log(`[${processed}/${modelFiles.length}] ${result.classification} ${model}`);
      }
      if (args.includes('--stop-on-error') && failures) break;
    }
  } finally {
    // elkjs's in-process FakeWorker does not implement terminate().
    if (typeof jsEngine.worker?.worker?.terminate === 'function') jsEngine.terminateWorker();
    rsEngine.terminateWorker();
  }
  const summary = { total: modelFiles.length, processed, stats, javaCompared, javaMatches, missingJava, failures,
    elkjsVersion: require('elkjs/package.json').version, javaBaseline: JAVA_BASELINE_DIR,
    rustBackend: BACKEND_PATH ? resolve(BACKEND_PATH) : 'auto', extraAlgorithms: [...extraIds] };
  await mkdir(REPORT_DIR, { recursive: true });
  await writeFile(join(REPORT_DIR, 'parity-report.json'), JSON.stringify(details, null, 2));
  await writeFile(join(REPORT_DIR, 'parity-summary.json'), JSON.stringify(summary, null, 2));
  console.log(JSON.stringify(summary, null, 2));
  process.exitCode = failures ? 1 : 0;
}
runParity().catch(error => {
  console.error(error);
  process.exitCode = 2;
});
