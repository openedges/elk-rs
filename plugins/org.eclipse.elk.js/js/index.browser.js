'use strict';

/**
 * @openedges/elk-layout (elk-rs) Browser entry point.
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
      // Keep the constructor synchronous; every public operation already returns
      // a Promise and waits for the same lazy WASM initialization.
      var ready;
      var call = function(method, args) {
        if (!ready) {
          ready = import('../dist/wasm/org_eclipse_elk_wasm.js').then(function(wasm) {
            if (typeof wasm.default === 'function') {
              return Promise.resolve(wasm.default()).then(function() { return wasm; });
            }
            return wasm;
          });
        }
        return ready.then(function(wasm) { return wasm[method].apply(wasm, args); });
      };
      optionsClone.backend = {
        layout_json: function(graph, layoutOptions) { return call('layout_json', [graph, layoutOptions]); },
        known_layout_algorithms: function() { return call('known_layout_algorithms', []); },
        known_layout_options: function() { return call('known_layout_options', []); },
        known_layout_categories: function() { return call('known_layout_categories', []); },
      };
    }

    super(optionsClone);
  }
}

Object.defineProperty(module.exports, "__esModule", { value: true });
module.exports = ELKBrowser;
ELKBrowser.default = ELKBrowser;
