# elk-rs

ELK layout engine rewritten in Rust — drop-in replacement for [elkjs](https://github.com/kieler/elkjs) with WASM and native Node.js addon support.

## Installation

```bash
npm install elk-rs
```

## Usage

elk-rs provides an elkjs-compatible API. In most cases you can replace `elkjs` with `elk-rs` directly:

```js
const ELK = require('elk-rs');
const elk = new ELK();

const graph = {
  id: 'root',
  layoutOptions: { 'elk.algorithm': 'layered' },
  children: [
    { id: 'n1', width: 30, height: 30 },
    { id: 'n2', width: 30, height: 30 },
  ],
  edges: [
    { id: 'e1', sources: ['n1'], targets: ['n2'] }
  ]
};

elk.layout(graph).then(console.log);
```

### ESM

```js
import ELK from 'elk-rs';
const elk = new ELK();
```

### Browser

elk-rs works in the browser via WASM. Bundlers that respect the `"browser"` field in `package.json` will automatically use the browser entry point.

### Web Worker

```js
const ELK = require('elk-rs');
const elk = new ELK({
  workerUrl: './node_modules/elk-rs/js/elk-worker.js'
});
```

## API

### `new ELK(options?)`

- `defaultLayoutOptions` — default layout options applied to every `layout()` call
- `workerUrl` — URL to the worker script (enables Web Worker mode)
- `workerFactory` — custom function to create a Worker instance
- `algorithms` — list of algorithm IDs to register (all built-in by default)

### `elk.layout(graph, options?)`

Returns a `Promise<LayoutedGraph>`. The graph follows the [ELK JSON format](https://www.eclipse.dev/elk/documentation/tooldevelopers/graphdatastructure/jsonformat.html).

### `elk.knownLayoutAlgorithms()`

Returns a `Promise` with an array of registered layout algorithm descriptions.

### `elk.knownLayoutOptions()`

Returns a `Promise` with an array of available layout options.

### `elk.knownLayoutCategories()`

Returns a `Promise` with an array of layout categories.

### `elk.terminateWorker()`

Terminates the Web Worker (if one was created).

## Differences from elkjs

- **Written in Rust** — compiled to WASM instead of GWT-transpiled JavaScript
- **No GWT overhead** — faster startup, smaller memory footprint
- **Native Node.js addon** — optional NAPI binding for maximum performance (future release)
- **Same API** — elkjs-compatible `layout()`, `knownLayoutAlgorithms()`, etc.
- **Same algorithms** — layered, stress, mrtree, radial, force, disco, rectpacking, sporeOverlap, sporeCompaction

## Supported Algorithms

| Algorithm | ELK ID |
|-----------|--------|
| Layered | `org.eclipse.elk.layered` |
| Stress | `org.eclipse.elk.stress` |
| MrTree | `org.eclipse.elk.mrtree` |
| Radial | `org.eclipse.elk.radial` |
| Force | `org.eclipse.elk.force` |
| DisCo | `org.eclipse.elk.disco` |
| Rect Packing | `org.eclipse.elk.rectpacking` |
| Spore Overlap | `org.eclipse.elk.sporeOverlap` |
| Spore Compaction | `org.eclipse.elk.sporeCompaction` |

## License

[Eclipse Public License 2.0](./LICENSE)
