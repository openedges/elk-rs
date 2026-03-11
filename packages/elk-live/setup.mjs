#!/usr/bin/env node
/**
 * Setup script for elk-rs-live: copies WASM files and images from the main project.
 */
import { cpSync, mkdirSync, existsSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, '../..');

function copyIfExists(src, dest) {
  if (existsSync(src)) {
    mkdirSync(dirname(dest), { recursive: true });
    cpSync(src, dest, { recursive: true });
    console.log(`  ${src} → ${dest}`);
  } else {
    console.warn(`  WARN: ${src} not found`);
  }
}

console.log('Setting up elk-rs-live...\n');

// Copy WASM files
console.log('Copying WASM files:');
const wasmDir = resolve(root, 'plugins/org.eclipse.elk.js/dist/wasm');
const wasmDest = resolve(__dirname, 'src/wasm');
for (const f of ['org_eclipse_elk_wasm.js', 'org_eclipse_elk_wasm_bg.wasm', 'org_eclipse_elk_wasm.d.ts']) {
  copyIfExists(resolve(wasmDir, f), resolve(wasmDest, f));
}

// Copy images
console.log('\nCopying images:');
const imgSrc = resolve(root, 'external/elk-live/client/app/img');
const imgDest = resolve(__dirname, 'public/img');
if (existsSync(imgSrc)) {
  cpSync(imgSrc, imgDest, { recursive: true });
  console.log(`  ${imgSrc} → ${imgDest}`);
} else {
  console.warn(`  WARN: ${imgSrc} not found`);
}

console.log('\nSetup complete.');
