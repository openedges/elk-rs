#!/usr/bin/env node
/**
 * JS Parity Test: 3-way comparison of elk-rs JS, elkjs (GWT), and Java ELK baseline.
 *
 * For each model:
 *   1. Run through elk-rs JS (NAPI/WASM)
 *   2. Run through elkjs (GWT-compiled)
 *   3. Load Java ELK baseline from parity/model_parity/java/layout/
 *   4. Compare all three and classify any diffs
 *
 * Classification:
 *   PASS         — all three match
 *   ELKJS_DRIFT  — elk-rs matches Java, elkjs diverges (GWT artifact)
 *   ELKRS_DRIFT  — elkjs matches Java, elk-rs diverges (elk-rs bug)
 *   ALL_DIFFER   — all three produce different results
 *
 * Usage: node test/parity/run.mjs [--models <dir>] [--tolerance <num>] [--stop-on-error]
 */
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import { readdirSync, statSync, existsSync, readFileSync } from 'node:fs';

const require = createRequire(import.meta.url);
const __dirname = fileURLToPath(new URL('.', import.meta.url));
const PKG_ROOT = join(__dirname, '../..');
const REPO_ROOT = join(PKG_ROOT, '../..');

// --- CLI args ---
const args = process.argv.slice(2);
function getArg(name, fallback) {
  const idx = args.indexOf(name);
  if (idx === -1) return fallback;
  return args[idx + 1];
}
const MODELS_DIR = getArg('--models', join(REPO_ROOT, 'external/elk-models'));
const ABS_TOL = parseFloat(getArg('--tolerance', '1e-6'));
const STOP_ON_ERROR = args.includes('--stop-on-error');
const REPORT_DIR = getArg('--output', join(PKG_ROOT, 'test/parity/results'));
const JAVA_BASELINE_DIR = getArg('--java-baseline', join(REPO_ROOT, 'parity/model_parity/java'));

// --- Load both ELK engines ---
const ELKjs = require('elkjs');
const ELKrs = require(join(PKG_ROOT, 'js/index.js'));

// --- Load Java manifest ---
function loadJavaManifest() {
  const manifestPath = join(JAVA_BASELINE_DIR, 'java_manifest.tsv');
  if (!existsSync(manifestPath)) return null;
  const map = {};
  const lines = readFileSync(manifestPath, 'utf-8').trim().split('\n');
  for (const line of lines) {
    const [model, input, layout, status] = line.split('\t');
    if (model.endsWith('.elkg') && status === 'ok') {
      // Map: realworld/ptolemy/.../xxx.elkg -> layout path
      const jsonName = model.replace('.elkg', '.json');
      map[jsonName] = layout;
    }
  }
  return map;
}

// --- Utilities ---

/** Recursively find all .json files */
function findJsonFiles(dir) {
  const results = [];
  let entries;
  try {
    entries = readdirSync(dir);
  } catch {
    return results;
  }
  for (const entry of entries) {
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
  return results;
}

/** Keys that are input metadata, not layout output — skip in comparison */
const SKIP_KEYS = new Set(['$H', 'properties', 'layoutOptions']);

/** Deep-clone and strip non-layout properties for comparison */
function stripInternal(obj) {
  if (obj === null || obj === undefined) return obj;
  if (typeof obj !== 'object') return obj;
  if (Array.isArray(obj)) return obj.map(stripInternal);
  const result = {};
  for (const key of Object.keys(obj).sort()) {
    if (key.startsWith('$')) continue;
    if (SKIP_KEYS.has(key)) continue;
    result[key] = stripInternal(obj[key]);
  }
  return result;
}

/** Deep-compare two values with numeric tolerance. Returns list of diffs. */
function deepCompare(a, b, path = '') {
  const diffs = [];
  if (a === b) return diffs;

  if (typeof a === 'number' && typeof b === 'number') {
    if (Number.isNaN(a) && Number.isNaN(b)) return diffs;
    if (Math.abs(a - b) > ABS_TOL) {
      diffs.push({ path, expected: a, actual: b, type: 'value' });
    }
    return diffs;
  }

  if (typeof a !== typeof b) {
    diffs.push({ path, expected: a, actual: b, type: 'type' });
    return diffs;
  }

  if (a === null || b === null) {
    if (a !== b) diffs.push({ path, expected: a, actual: b, type: 'null' });
    return diffs;
  }

  if (Array.isArray(a) || Array.isArray(b)) {
    if (!Array.isArray(a) || !Array.isArray(b)) {
      diffs.push({ path, expected: `array(${Array.isArray(a)})`, actual: `array(${Array.isArray(b)})`, type: 'type' });
      return diffs;
    }
    const sa = stableSort(a);
    const sb = stableSort(b);
    if (sa.length !== sb.length) {
      diffs.push({ path, expected: `length=${sa.length}`, actual: `length=${sb.length}`, type: 'array_length' });
    }
    const len = Math.min(sa.length, sb.length);
    for (let i = 0; i < len; i++) {
      diffs.push(...deepCompare(sa[i], sb[i], `${path}[${i}]`));
    }
    return diffs;
  }

  if (typeof a === 'object') {
    const allKeys = [...new Set([...Object.keys(a), ...Object.keys(b)])].sort();
    for (const key of allKeys) {
      if (!(key in a)) {
        diffs.push({ path: `${path}.${key}`, expected: '<missing>', actual: b[key], type: 'missing_in_expected' });
      } else if (!(key in b)) {
        diffs.push({ path: `${path}.${key}`, expected: a[key], actual: '<missing>', type: 'missing_in_actual' });
      } else {
        diffs.push(...deepCompare(a[key], b[key], `${path}.${key}`));
      }
    }
    return diffs;
  }

  if (a !== b) {
    diffs.push({ path, expected: a, actual: b, type: 'value' });
  }
  return diffs;
}

/** Sort arrays by .id if all elements have one */
function stableSort(arr) {
  if (arr.length === 0) return arr;
  if (arr.every(item => item && typeof item === 'object' && 'id' in item)) {
    return [...arr].sort((a, b) => String(a.id).localeCompare(String(b.id)));
  }
  if (arr.every(item => typeof item === 'string')) {
    return [...arr].sort();
  }
  return arr;
}

// --- Main ---

async function runParity() {
  console.log('JS 3-Way Parity Test: elk-rs vs elkjs vs Java ELK');
  console.log(`Models:    ${MODELS_DIR}`);
  console.log(`Java base: ${JAVA_BASELINE_DIR}`);
  console.log(`Tolerance: ${ABS_TOL}`);
  console.log('');

  const javaManifest = loadJavaManifest();
  const hasJava = javaManifest && Object.keys(javaManifest).length > 0;
  if (hasJava) {
    console.log(`Java baseline: ${Object.keys(javaManifest).length} models loaded`);
  } else {
    console.log('Java baseline: NOT AVAILABLE (falling back to 2-way comparison)');
  }
  console.log('');

  const modelFiles = findJsonFiles(MODELS_DIR);
  console.log(`Found ${modelFiles.length} model files\n`);

  if (modelFiles.length === 0) {
    console.error('No model files found. Check --models path.');
    process.exit(1);
  }

  const stats = {
    pass: 0,
    elkjsDrift: 0,     // elk-rs matches Java, elkjs diverges
    elkrsDrift: 0,      // elkjs matches Java, elk-rs diverges
    allDiffer: 0,       // all three differ
    noJavaBaseline: 0,  // 2-way fallback (no Java baseline)
    bothErrored: 0,
    elkjsOnlyErr: 0,
    elkrsOnlyErr: 0,
  };
  const failures = [];

  for (let i = 0; i < modelFiles.length; i++) {
    const filePath = modelFiles[i];
    const modelName = relative(MODELS_DIR, filePath);
    const progress = `[${i + 1}/${modelFiles.length}]`;

    let graph;
    try {
      graph = JSON.parse(await readFile(filePath, 'utf-8'));
    } catch (err) {
      console.log(`${progress} SKIP  ${modelName} (invalid JSON)`);
      continue;
    }

    const graphForElkjs = JSON.parse(JSON.stringify(graph));
    const graphForElkrs = JSON.parse(JSON.stringify(graph));

    let elkjsResult, elkrsResult;
    let elkjsError, elkrsError;

    // Run elkjs
    try {
      elkjsResult = await new ELKjs().layout(graphForElkjs);
    } catch (err) {
      elkjsError = err;
    }

    // Run elk-rs
    try {
      elkrsResult = await new ELKrs().layout(graphForElkrs);
    } catch (err) {
      elkrsError = err;
    }

    // Both errored
    if (elkjsError && elkrsError) {
      stats.bothErrored++;
      process.stdout.write(`${progress} BOTH_ERR  ${modelName}\r`);
      continue;
    }

    // Only elkjs errored
    if (elkjsError && !elkrsError) {
      stats.elkjsOnlyErr++;
      process.stdout.write(`${progress} ELKJS_ERR ${modelName}\r`);
      continue;
    }

    // Only elk-rs errored
    if (!elkjsError && elkrsError) {
      stats.elkrsOnlyErr++;
      const msg = elkrsError.message || String(elkrsError);
      console.log(`${progress} ELKRS_ERR ${modelName}: ${msg.slice(0, 120)}`);
      failures.push({ model: modelName, classification: 'ELKRS_ERROR', message: msg });
      if (STOP_ON_ERROR) break;
      continue;
    }

    // Both succeeded — clean outputs
    const cleanElkjs = stripInternal(elkjsResult);
    const cleanElkrs = stripInternal(elkrsResult);

    // Load Java baseline if available
    let cleanJava = null;
    if (hasJava && javaManifest[modelName]) {
      try {
        const javaData = JSON.parse(await readFile(javaManifest[modelName], 'utf-8'));
        cleanJava = stripInternal(javaData);
      } catch { /* Java baseline unreadable */ }
    }

    // Compare elk-rs vs elkjs
    const diffsRsVsJs = deepCompare(cleanElkjs, cleanElkrs, '');

    if (diffsRsVsJs.length === 0) {
      stats.pass++;
      process.stdout.write(`${progress} PASS          ${modelName}\r`);
      continue;
    }

    // There are diffs — classify using Java baseline
    if (cleanJava) {
      const diffsRsVsJava = deepCompare(cleanJava, cleanElkrs, '');
      const diffsJsVsJava = deepCompare(cleanJava, cleanElkjs, '');

      const rsMatchesJava = diffsRsVsJava.length === 0;
      const jsMatchesJava = diffsJsVsJava.length === 0;

      if (rsMatchesJava && !jsMatchesJava) {
        // elk-rs matches Java, elkjs diverges → GWT artifact
        stats.elkjsDrift++;
        console.log(`${progress} ELKJS_DRIFT   ${modelName} (${diffsRsVsJs.length} diffs, elkjs≠Java)`);
        failures.push({
          model: modelName,
          classification: 'ELKJS_DRIFT',
          ndiffs: diffsRsVsJs.length,
          note: 'elk-rs matches Java ELK; elkjs (GWT) diverges',
          sampleDiffs: diffsRsVsJs.slice(0, 5),
        });
      } else if (!rsMatchesJava && jsMatchesJava) {
        // elkjs matches Java, elk-rs diverges → elk-rs bug
        stats.elkrsDrift++;
        console.log(`${progress} ELKRS_DRIFT   ${modelName} (${diffsRsVsJs.length} diffs, elk-rs≠Java)`);
        failures.push({
          model: modelName,
          classification: 'ELKRS_DRIFT',
          ndiffs: diffsRsVsJs.length,
          note: 'elkjs matches Java ELK; elk-rs diverges — needs investigation',
          sampleDiffs: diffsRsVsJava.slice(0, 10),
        });
      } else if (!rsMatchesJava && !jsMatchesJava) {
        // Both differ from Java
        stats.allDiffer++;
        console.log(`${progress} ALL_DIFFER    ${modelName} (rs≠Java: ${diffsRsVsJava.length}, js≠Java: ${diffsJsVsJava.length})`);
        failures.push({
          model: modelName,
          classification: 'ALL_DIFFER',
          note: 'Both elk-rs and elkjs differ from Java baseline',
          rsVsJavaDiffs: diffsRsVsJava.length,
          jsVsJavaDiffs: diffsJsVsJava.length,
          sampleDiffs: diffsRsVsJava.slice(0, 5),
        });
      } else {
        // Both match Java but differ from each other? Shouldn't happen with same tolerance
        stats.pass++;
        process.stdout.write(`${progress} PASS          ${modelName}\r`);
      }
    } else {
      // No Java baseline — 2-way only
      stats.noJavaBaseline++;
      console.log(`${progress} FAIL(no-java) ${modelName} (${diffsRsVsJs.length} diffs)`);
      failures.push({
        model: modelName,
        classification: 'UNKNOWN',
        ndiffs: diffsRsVsJs.length,
        note: 'No Java baseline available to classify',
        sampleDiffs: diffsRsVsJs.slice(0, 5),
      });
    }

    if (STOP_ON_ERROR) break;
  }

  // Clear line
  process.stdout.write('\n');

  // Summary
  const total = modelFiles.length;
  const realFailures = stats.elkrsDrift + stats.elkrsOnlyErr;

  console.log('\n' + '='.repeat(56));
  console.log('JS 3-Way Parity Report');
  console.log('='.repeat(56));
  console.log(`Total models:          ${total}`);
  console.log('');
  console.log(`  PASS (all match):    ${stats.pass}`);
  console.log(`  ELKJS_DRIFT:         ${stats.elkjsDrift}   (elkjs GWT artifact, elk-rs correct)`);
  console.log(`  ELKRS_DRIFT:         ${stats.elkrsDrift}   (elk-rs bug, needs fix)`);
  console.log(`  ALL_DIFFER:          ${stats.allDiffer}`);
  console.log(`  No Java baseline:    ${stats.noJavaBaseline}`);
  console.log(`  Both errored:        ${stats.bothErrored}`);
  console.log(`  elkjs only error:    ${stats.elkjsOnlyErr}`);
  console.log(`  elk-rs only error:   ${stats.elkrsOnlyErr}`);
  console.log('');
  console.log(`elk-rs vs Java match:  ${stats.pass + stats.elkjsDrift}/${stats.pass + stats.elkjsDrift + stats.elkrsDrift + stats.allDiffer} (${(((stats.pass + stats.elkjsDrift) / (stats.pass + stats.elkjsDrift + stats.elkrsDrift + stats.allDiffer)) * 100).toFixed(1)}%)`);
  console.log(`Real elk-rs failures:  ${realFailures}`);
  console.log('='.repeat(56) + '\n');

  // Write report
  await mkdir(REPORT_DIR, { recursive: true });
  const reportPath = join(REPORT_DIR, 'parity-report.json');
  await writeFile(reportPath, JSON.stringify(failures, null, 2));
  console.log(`Detailed report: ${relative(PKG_ROOT, reportPath)}`);

  if (failures.length > 0) {
    console.log('\nFailure details:');
    for (const f of failures.slice(0, 15)) {
      console.log(`  [${f.classification}] ${f.model}`);
      if (f.sampleDiffs) {
        for (const d of f.sampleDiffs.slice(0, 3)) {
          console.log(`    ${d.path}: expected=${JSON.stringify(d.expected)} actual=${JSON.stringify(d.actual)}`);
        }
      }
      if (f.message) console.log(`    ${f.message.slice(0, 150)}`);
    }
  }

  // Exit code: only real elk-rs failures count
  process.exit(realFailures > 0 ? 1 : 0);
}

runParity().catch(err => {
  console.error('Fatal error:', err);
  process.exit(2);
});
