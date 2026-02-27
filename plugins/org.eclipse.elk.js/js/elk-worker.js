'use strict';

/**
 * elk-rs Worker script — runs in Web Worker or Node.js worker_threads.
 *
 * Loads the WASM module and handles the elkjs-compatible message protocol.
 * Also exports a `Worker` class for in-process use (like elkjs's fake worker).
 */

// Detect environment
var isWebWorker = typeof self !== 'undefined' && typeof self.postMessage === 'function' && typeof window === 'undefined';

// --- In-process (fake) Worker for Node.js direct use ---

function FakeWorker() {
  var _this = this;
  this._backend = null;
  this._initPromise = null;
}

FakeWorker.prototype._ensureBackend = function() {
  if (this._backend) return Promise.resolve(this._backend);
  if (this._initPromise) return this._initPromise;

  var self = this;
  this._initPromise = new Promise(function(resolve, reject) {
    try {
      // Try native addon first, then WASM
      var backend;
      try {
        backend = require('../dist/elk-rs.node');
      } catch (e1) {
        try {
          backend = require('../dist/wasm/org_eclipse_elk_wasm.js');
        } catch (e2) {
          throw new Error('elk-rs: Could not load native addon or WASM module. ' + e1.message);
        }
      }
      self._backend = backend;
      resolve(backend);
    } catch (err) {
      reject(err);
    }
  });
  return this._initPromise;
};

FakeWorker.prototype.postMessage = function(msg) {
  var self = this;
  setTimeout(function() {
    self._ensureBackend().then(function(backend) {
      self._handleMessage(backend, msg);
    }).catch(function(err) {
      if (self.onmessage) {
        self.onmessage({ data: { id: msg.id, error: convertError(err) } });
      }
    });
  }, 0);
};

FakeWorker.prototype._handleMessage = function(backend, msg) {
  var result;
  try {
    result = handleCommand(backend, msg);
    if (this.onmessage) {
      this.onmessage({ data: { id: msg.id, data: result } });
    }
  } catch (err) {
    if (this.onmessage) {
      this.onmessage({ data: { id: msg.id, error: convertError(err) } });
    }
  }
};

FakeWorker.prototype.terminate = function() {
  // Nothing to clean up for in-process worker
};

// --- Command handling (shared between Web Worker and FakeWorker) ---

function handleCommand(backend, msg) {
  switch (msg.cmd) {
    case 'register':
      // All algorithms are built-in; nothing to register
      return null;

    case 'layout': {
      var graphJson = JSON.stringify(msg.graph);
      var optionsJson = JSON.stringify(msg.layoutOptions || {});
      var resultJson = backend.layout_json(graphJson, optionsJson);
      return JSON.parse(resultJson);
    }

    case 'algorithms':
      return JSON.parse(backend.known_layout_algorithms());

    case 'options':
      return JSON.parse(backend.known_layout_options());

    case 'categories':
      return JSON.parse(backend.known_layout_categories());

    default:
      throw new Error('Unknown command: ' + msg.cmd);
  }
}

function convertError(err) {
  if (err instanceof Error) {
    return { message: err.message };
  }
  if (typeof err === 'string') {
    return { message: err };
  }
  return { message: String(err) };
}

// --- Worker mode ---

// Detect Node.js worker_threads (parentPort is available when running as a worker thread)
var _parentPort = null;
try {
  var _wt = require('worker_threads');
  if (!_wt.isMainThread && _wt.parentPort) {
    _parentPort = _wt.parentPort;
  }
} catch (e) { /* not in Node.js or not a worker thread */ }

if (_parentPort) {
  // Node.js worker_threads — use require-based backend loading and parentPort
  var nodeBackend = null;
  try {
    nodeBackend = require('../dist/elk-rs.node');
  } catch (e1) {
    try {
      nodeBackend = require('../dist/wasm/org_eclipse_elk_wasm.js');
    } catch (e2) {
      // Will report error when messages arrive
    }
  }

  _parentPort.on('message', function(msg) {
    if (!nodeBackend) {
      _parentPort.postMessage({ id: msg.id, error: { message: 'elk-rs: Could not load backend in worker.' } });
      return;
    }
    try {
      var result = handleCommand(nodeBackend, msg);
      _parentPort.postMessage({ id: msg.id, data: result });
    } catch (err) {
      _parentPort.postMessage({ id: msg.id, error: convertError(err) });
    }
  });
} else if (isWebWorker) {
  // Pure browser Web Worker — use WASM via dynamic import
  var wasmBackend = null;
  var wasmReady = null;

  wasmReady = (function() {
    return import('../dist/wasm/org_eclipse_elk_wasm.js').then(function(module) {
      if (module.default && typeof module.default === 'function') {
        return module.default().then(function() {
          wasmBackend = module;
          return module;
        });
      }
      wasmBackend = module;
      return module;
    });
  })();

  self.onmessage = function(e) {
    var msg = e.data;
    wasmReady.then(function(backend) {
      try {
        var result = handleCommand(backend, msg);
        self.postMessage({ id: msg.id, data: result });
      } catch (err) {
        self.postMessage({ id: msg.id, error: convertError(err) });
      }
    }).catch(function(err) {
      self.postMessage({ id: msg.id, error: convertError(err) });
    });
  };
}

// --- Exports ---

if (typeof module !== 'undefined' && module.exports) {
  module.exports.Worker = FakeWorker;
}
