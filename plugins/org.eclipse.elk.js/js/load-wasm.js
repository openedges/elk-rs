'use strict';

// Node.js only: the release web-target glue needs synchronous initialization.
// wasm-bindgen's nodejs-target glue has no initSync and initializes itself.
var backend;
module.exports = function loadWasm() {
  if (backend) return backend;
  var wasm;
  try {
    wasm = require('../dist/wasm-node/org_eclipse_elk_wasm.js');
  } catch (error) {
    // Older development builds contain only the web-target glue.
    wasm = require('../dist/wasm/org_eclipse_elk_wasm.js');
  }
  if (typeof wasm.initSync === 'function') {
    var bytes = require('fs').readFileSync(require.resolve('../dist/wasm/org_eclipse_elk_wasm_bg.wasm'));
    wasm.initSync({ module: bytes });
  }
  backend = wasm;
  return backend;
};
