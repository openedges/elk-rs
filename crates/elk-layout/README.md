# elk-layout

Pure Rust port of the [Eclipse Layout Kernel (ELK)](https://eclipse.dev/elk/), from the
[elk-rs](https://github.com/openedges/elk-rs) project. ELK's Java plugins are ported one to
one as the `org-eclipse-elk-*` crates; `elk-layout` re-exports all of them and the
elkjs-compatible JSON layout API, so a single dependency is enough:

```sh
cargo add elk-layout
```

```rust
let graph = r#"{
  "id": "root",
  "layoutOptions": { "elk.algorithm": "layered" },
  "children": [
    { "id": "n1", "width": 30, "height": 30 },
    { "id": "n2", "width": 30, "height": 30 }
  ],
  "edges": [ { "id": "e1", "sources": ["n1"], "targets": ["n2"] } ]
}"#;

// The second argument holds global layout options, merged as defaults.
let laid_out: String = elk_layout::layout_json(graph, "{}")?;
```

`layout_json` takes and returns the same JSON that [elkjs](https://github.com/kieler/elkjs)
uses. `known_layout_algorithms`, `known_layout_options` and `known_layout_categories` return
the registered metadata as JSON arrays.

When the graph is already a `serde_json::Value`, `layout_value` consumes it
and returns the laid-out `Value` directly:

```rust
let graph_value: serde_json::Value = serde_json::from_str(graph)?;
let laid_out_value = elk_layout::layout_value(graph_value, "{}")?;
```

Global options still use a JSON string and are merged as defaults. Authored
options and arbitrary graph fields are preserved. Both APIs use the same
layout pipeline and panic-to-error translation. Avoiding text conversion can
preserve a floating-point value that a serialize/parse round trip would round
differently; callers requiring the old text boundary can keep `layout_json`.

## Modules

| Module | Crate | ELK plugin |
|---|---|---|
| `elk_layout::graph` | `org-eclipse-elk-graph` | `org.eclipse.elk.graph` (ElkGraph data model) |
| `elk_layout::core` | `org-eclipse-elk-core` | `org.eclipse.elk.core` (options, registry, recursive layout) |
| `elk_layout::graph_json` | `org-eclipse-elk-graph-json` | `org.eclipse.elk.graph.json` |
| `elk_layout::alg::layered`, `force`, `mrtree`, `radial`, `rectpacking`, `spore`, `disco`, `topdownpacking`, `vertiflex`, `common`, `libavoid`, `graphviz_dot`, `graphviz_layouter` | `org-eclipse-elk-alg-*` | `org.eclipse.elk.alg.*` |
| `elk_layout::conn_gmf` | `org-eclipse-elk-conn-gmf` | `org.eclipse.elk.conn.gmf` |

All crates share one version and pin each other exactly, so a project never mixes two
elk-rs versions. Enable the `mimalloc-alloc` feature to use mimalloc as the global allocator.

## Parity with ELK

The port is verified against ELK Java on the full `elk-models` corpus: final layouts match
on 1,988/1,989 comparable models and the 50+ intermediate layered processor states match on
1,997/1,997 models. The same engine ships as the `@openedges/elk-layout` npm package (WASM
and native Node.js addon). See the repository for the parity reports, benchmarks and
versioning policy.

## License

EPL-2.0, like ELK itself.
