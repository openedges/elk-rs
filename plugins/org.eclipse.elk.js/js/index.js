'use strict';

/**
 * elk-rs Node.js entry point.
 *
 * Tries to load the native addon first, then falls back to WASM.
 * Unlike elkjs, works without a Worker by default (direct mode).
 * Worker mode is available via workerUrl/workerFactory.
 */
var ELK = require('./elk-api.js');

function loadBackend() {
  // Try native addon
  try {
    return require('../dist/elk-rs.node');
  } catch (e) {
    // Native addon not available
  }

  // Fall back to WASM (Node.js compatible build)
  try {
    return require('../dist/wasm/org_eclipse_elk_wasm.js');
  } catch (e) {
    // WASM not available either
  }

  throw new Error(
    'elk-rs: Could not load native addon or WASM module.\n'
    + 'Ensure the package was installed correctly and dist/wasm/ exists.'
  );
}

class ELKNode extends ELK {
  constructor(options = {}) {
    var optionsClone = Object.assign({}, options);

    // If user explicitly requested a worker
    if (options.workerUrl) {
      var workerThreadsExist = false;
      try {
        require('worker_threads');
        workerThreadsExist = true;
      } catch (e) { }

      if (workerThreadsExist) {
        var NodeWorker = require('worker_threads').Worker;
        optionsClone.workerFactory = function(url) {
          var worker = new NodeWorker(url);
          // Adapt worker_threads API to Web Worker API for PromisedWorker
          // worker_threads uses .on('message', data) instead of .onmessage({data})
          worker.on('message', function(data) {
            if (typeof worker.onmessage === 'function') {
              worker.onmessage({ data: data });
            }
          });
          worker.on('error', function(err) {
            if (typeof worker.onerror === 'function') {
              worker.onerror(err);
            }
          });
          return worker;
        };
      } else {
        console.warn("Worker requested but 'worker_threads' is not available.\n"
          + "Pass your own 'workerFactory' to ELK's constructor.\n"
          + "... Falling back to non-worker version.");
      }
    }

    // If no workerFactory yet, try direct mode (native/WASM)
    if (!optionsClone.workerFactory && !optionsClone.workerUrl) {
      optionsClone.backend = loadBackend();
    } else if (!optionsClone.workerFactory) {
      // workerUrl provided but no workerFactory and web-worker not installed
      var FakeWorker = require('./elk-worker.js').Worker;
      optionsClone.workerFactory = function(_url) { return new FakeWorker(); };
    }

    super(optionsClone);
  }
}

Object.defineProperty(module.exports, "__esModule", { value: true });
module.exports = ELKNode;
ELKNode.default = ELKNode;
