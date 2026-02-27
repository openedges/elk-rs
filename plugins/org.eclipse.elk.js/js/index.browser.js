'use strict';

/**
 * elk-rs Browser entry point.
 *
 * Uses WASM module directly (direct mode) or via Web Worker.
 */
var ELK = require('./elk-api.js');

class ELKBrowser extends ELK {
  constructor(options = {}) {
    var optionsClone = Object.assign({}, options);

    // If no worker requested, try direct WASM
    if (typeof optionsClone.workerUrl === 'undefined'
        && typeof optionsClone.workerFactory === 'undefined') {
      // In browser without worker: try to load WASM directly
      try {
        var wasm = require('../dist/wasm/org_eclipse_elk_wasm.js');
        optionsClone.backend = wasm;
      } catch (e) {
        throw new Error("elk-rs: Could not load WASM module. "
          + "Provide a 'workerUrl' or 'workerFactory' to use Web Worker mode.");
      }
    }

    super(optionsClone);
  }
}

Object.defineProperty(module.exports, "__esModule", { value: true });
module.exports = ELKBrowser;
ELKBrowser.default = ELKBrowser;
