# @openedges/elk-layout

ELK layout engine rewritten in Rust — drop-in replacement for [elkjs](https://github.com/kieler/elkjs) with WASM and native Node.js addon support.

## Installation

Version 0.12.0 requires **Node.js 18 or later**. The Node.js WASM fallback uses CommonJS bindings; browser imports use ESM bindings. Node.js 16 is unsupported because the generated WASM requires WebAssembly reference types, which need an experimental flag on that runtime. Development and tests use the pinned Node.js 24.

```bash
npm install @openedges/elk-layout
```

## Usage

elk-rs provides an elkjs-compatible API. In most cases you can replace `elkjs` with `@openedges/elk-layout` directly:

```js
const ELK = require('@openedges/elk-layout');
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
import ELK from '@openedges/elk-layout';
const elk = new ELK();
```

### Browser

elk-rs works in the browser via WASM. Bundlers that respect the `"browser"` field in `package.json` will automatically use the browser entry point. Layout and metadata promises wait for WASM initialization, including concurrent first requests.

### Web Worker

```js
const ELK = require('@openedges/elk-layout');
const elk = new ELK({
  workerUrl: './node_modules/@openedges/elk-layout/js/elk-worker.js'
});
```

## API

### `new ELK(options?)`

- `defaultLayoutOptions` — default layout options applied to every `layout()` call
- `workerUrl` — URL to the worker script (enables Web Worker mode)
- `workerFactory` — custom function to create a Worker instance
- `algorithms` — accepted for elkjs constructor compatibility. All Rust algorithms remain available in direct and worker modes; this hint does not restrict the registry.

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
- **Native Node.js addon** — optional NAPI binding for maximum performance
- **Same API** — elkjs-compatible `layout()`, `knownLayoutAlgorithms()`, etc.
- **ELK 0.12.0 compatibility** — common algorithms follow Java ELK 0.12.0. VertiFlex and DisCo remain available as additional Java capabilities; elkjs 0.12.0 does not register these algorithms.

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
| VertiFlex (Java capability beyond elkjs 0.12.0) | `org.eclipse.elk.vertiflex` |

## Parity verification

Build the local NAPI/WASM backend from the current Rust sources and export a fresh Java ELK baseline of the same version before running parity. Java exports retain the original model extension, so `graph.json` has a layout file named `graph.json.json`.

```bash
node test/parity/run.mjs --models ../../external/elk-models \
  --java-baseline /path/to/fresh/java --require-java \
  --backend ./dist/elk-layout.darwin-arm64.node
```

Use the backend filename built for your platform, or pass `./dist/wasm/org_eclipse_elk_wasm.js` to verify WASM. An explicit `--backend` avoids selecting an older installed platform package. The report checks Java even when both JavaScript engines agree. `RUST_EXTRA` identifies a capability absent from elkjs only after its output matches Java. Missing baselines never count toward the Java match rate, and `--require-java` makes them fail the run. `parity-summary.json` records baseline and backend paths, elkjs version, classification counts, and the actual Java comparison denominator.

## License

[Eclipse Public License 2.0 with conditional GPL-3.0-or-later secondary licensing](./LICENSE.md).
