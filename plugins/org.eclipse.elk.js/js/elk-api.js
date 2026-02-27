'use strict';

/**
 * ELK layout API — elkjs-compatible interface backed by elk-rs (WASM/native).
 *
 * Supports two modes:
 *  - Worker mode: workerUrl or workerFactory provided → delegates to a Web Worker / worker_threads
 *  - Direct mode: neither provided → calls the backend synchronously and wraps in a Promise
 */
class ELK {

  constructor({
    defaultLayoutOptions = {},
    algorithms = [
      'layered',
      'stress',
      'mrtree',
      'radial',
      'force',
      'disco',
      'sporeOverlap',
      'sporeCompaction',
      'rectpacking'
    ],
    workerFactory,
    workerUrl,
    // elk-rs direct mode: backend object with layout_json, known_layout_algorithms, etc.
    backend
  } = {}) {
    this.defaultLayoutOptions = defaultLayoutOptions;
    this.initialized = false;

    // Direct mode: backend provided, no worker
    if (backend && typeof workerUrl === 'undefined' && typeof workerFactory === 'undefined') {
      this.backend = backend;
      this.initialized = true;
      return;
    }

    // Worker mode (elkjs-compatible)
    if (typeof workerUrl === 'undefined' && typeof workerFactory === 'undefined') {
      throw new Error("Cannot construct an ELK without both 'workerUrl' and 'workerFactory'.");
    }
    let factory = workerFactory;
    if (typeof workerUrl !== 'undefined' && typeof workerFactory === 'undefined') {
      factory = function(url) { return new Worker(url); };
    }

    let worker = factory(workerUrl);
    if (typeof worker.postMessage !== 'function') {
      throw new TypeError("Created worker does not provide"
        + " the required 'postMessage' function.");
    }

    this.worker = new PromisedWorker(worker);

    // Register algorithms
    this.worker.postMessage({
      cmd: 'register',
      algorithms: algorithms
    })
      .then((r) => this.initialized = true)
      .catch(console.err);
  }

  layout(graph, {
    layoutOptions = this.defaultLayoutOptions,
    logging = false,
    measureExecutionTime = false,
  } = {}) {
    if (!graph) {
      return Promise.reject(new Error("Missing mandatory parameter 'graph'."));
    }

    // Direct mode
    if (this.backend) {
      try {
        const graphJson = JSON.stringify(graph);
        const optionsJson = JSON.stringify(layoutOptions || {});
        const resultJson = this.backend.layout_json(graphJson, optionsJson);
        return Promise.resolve(JSON.parse(resultJson));
      } catch (err) {
        return Promise.reject(err);
      }
    }

    // Worker mode
    return this.worker.postMessage({
      cmd: 'layout',
      graph: graph,
      layoutOptions: layoutOptions,
      options: {
        logging: logging,
        measureExecutionTime: measureExecutionTime,
      }
    });
  }

  knownLayoutAlgorithms() {
    if (this.backend) {
      try {
        return Promise.resolve(JSON.parse(this.backend.known_layout_algorithms()));
      } catch (err) {
        return Promise.reject(err);
      }
    }
    return this.worker.postMessage({ cmd: 'algorithms' });
  }

  knownLayoutOptions() {
    if (this.backend) {
      try {
        return Promise.resolve(JSON.parse(this.backend.known_layout_options()));
      } catch (err) {
        return Promise.reject(err);
      }
    }
    return this.worker.postMessage({ cmd: 'options' });
  }

  knownLayoutCategories() {
    if (this.backend) {
      try {
        return Promise.resolve(JSON.parse(this.backend.known_layout_categories()));
      } catch (err) {
        return Promise.reject(err);
      }
    }
    return this.worker.postMessage({ cmd: 'categories' });
  }

  terminateWorker() {
    if (this.worker) {
      this.worker.terminate();
    }
  }

}

class PromisedWorker {

  constructor(worker) {
    if (worker === undefined) {
      throw new Error("Missing mandatory parameter 'worker'.");
    }
    this.resolvers = {};
    this.worker = worker;
    this.worker.onmessage = (answer) => {
      setTimeout(() => {
        this.receive(this, answer);
      }, 0);
    };
  }

  postMessage(msg) {
    let id = this.id || 0;
    this.id = id + 1;
    msg.id = id;
    let self = this;
    return new Promise(function(resolve, reject) {
      self.resolvers[id] = function(err, res) {
        if (err) {
          reject(err);
        } else {
          resolve(res);
        }
      };
      self.worker.postMessage(msg);
    });
  }

  receive(self, answer) {
    let json = answer.data;
    let resolver = self.resolvers[json.id];
    if (resolver) {
      delete self.resolvers[json.id];
      if (json.error) {
        resolver(json.error);
      } else {
        resolver(null, json.data);
      }
    }
  }

  terminate() {
    if (this.worker) {
      this.worker.terminate();
    }
  }

}

// Support both CJS and ESM
if (typeof module !== 'undefined' && module.exports) {
  module.exports = ELK;
  module.exports.default = ELK;
}
if (typeof exports !== 'undefined') {
  exports.default = ELK;
}
