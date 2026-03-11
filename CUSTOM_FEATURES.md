# elk-rs Custom Features — elk-live

## Overview

This document describes the **elk-live** custom feature: a standalone web demonstrator for elk-rs that replaces the original Sprotty-based [elk-live](https://rtsys.informatik.uni-kiel.de/elklive/) with a lightweight Vite + Monaco + SVG implementation powered by the elk-rs WASM engine.

The original elk-live (Java/Sprotty) is preserved as a reference submodule at `external/elk-live`.

## Branch and Version

| Item | Value |
|------|-------|
| Feature branch | `custom/elk-live` |
| Base | `main` (`v0.11.0` — ELK Java 1:1 parity) |
| Package | `elk-rs-live@0.11.0` (private, not published) |
| Reference submodule | `external/elk-live` → [kieler/elk-live](https://github.com/kieler/elk-live) |

---

## Feature: elk-live Demonstrator

### Description

A standalone web application that provides two main views:

1. **Interactive Editor** (`editor.html`): ELKT/JSON editor with live layout preview. Supports mode switching (elkt↔json), URL-based model sharing via LZ-string compression, and a "Link to this model" feature.

2. **Examples Browser** (`examples.html`): Sidebar navigation of all elk-models examples (`.elkt` files with `elkex:` annotations), with live editor, SVG diagram, and Markdown description panel.

Both views share a common SVG renderer with Sprotty-compatible pan/zoom and per-element animation.

### Architecture

```
packages/elk-live/
├── src/
│   ├── editor.ts              # Interactive editor entry point
│   ├── examples.ts            # Examples browser entry point
│   ├── index.ts               # Landing page
│   ├── common/
│   │   ├── dark-mode.ts       # Dark mode toggle (localStorage)
│   │   ├── elkt-language.ts   # Monaco ELKT language definition
│   │   └── url-params.ts      # URL parameter parsing
│   ├── elk/
│   │   ├── elk-layout.ts      # WASM layout interface
│   │   └── elk-types.ts       # ELK JSON type definitions
│   ├── elkt/
│   │   └── parser.ts          # ELKT text → ELK JSON parser
│   ├── elkex/
│   │   └── parser.ts          # Example file annotation parser
│   └── render/
│       └── svg-renderer.ts    # SVG renderer with pan/zoom/animation
├── styles/
│   ├── common.css             # Shared CSS (navbar, footer, panes, dark mode)
│   └── diagram.css            # SVG diagram styling (nodes, edges, labels)
├── test/
│   ├── elkt-parser.test.ts    # ELKT parser unit tests
│   ├── elkex-parser.test.ts   # Example parser unit tests
│   └── all-examples-wasm.test.ts  # E2E: parse + layout + parity check
├── editor.html                # Interactive editor page
├── examples.html              # Examples browser page
├── index.html                 # Landing page
├── setup.mjs                  # WASM file copy script
├── vite.config.ts             # Vite build configuration
└── vitest.config.ts           # Test configuration
```

### Key Components

#### SVG Renderer (`src/render/svg-renderer.ts`)

Sprotty-compatible rendering without viewBox:

- **Viewport**: No SVG `viewBox`/`width`/`height` attributes. Root `<g>` uses `transform="scale(s) translate(tx,ty)"` — matches original Sprotty approach for consistent sub-pixel stroke rendering across different container sizes.
- **Pan**: Mouse drag adjusts `translate` by `dx/scale, dy/scale`.
- **Zoom**: Wheel zoom keeps the point under cursor fixed: `scroll += mx/scale * (1 - 1/factor)`.
- **Animation**: Per-element move (SVG `transform` attribute interpolation) + fade-in (SVG `opacity` attribute interpolation), 300ms ease-in-out. `animId` counter cancels in-flight animations on re-render.
- **Element tracking**: Every logical element wrapped in `<g data-elk-id="...">` for position snapshot/restore across re-renders.

#### ELKT Parser (`src/elkt/parser.ts`)

Full tokenizer + recursive descent parser:

- Tokenizer: whitespace, line/block comments, strings, numbers, booleans, null, keywords, identifiers (with dots for qualified IDs, `^` escape)
- Parser: nodes, ports, edges (with optional ID prefix), labels, layout options, layout sections (`size:`, `position:`), nested hierarchies
- ID qualification: local IDs qualified with parent scope (e.g., `parent$child$port`) for global uniqueness
- Edge endpoint dot notation: `n1.p1` → `n1$p1` (port reference)
- Defaults: nodes 30x30, ports 5x5, labels `text.length * 9` x 16 (matches Java `ElkGraphDiagramGenerator.applyDefaults`)

#### Example Parser (`src/elkex/parser.ts`)

Parses `elkex:` annotations from `.elkt` example files:

- Sections: `category`, `label`, `doc`, `graph`
- Builds hierarchical category tree for sidebar navigation
- Markdown documentation rendered via Showdown

### Setup

```bash
cd packages/elk-live
npm install
node setup.mjs      # copies WASM files from ../../target/wasm-dist/
npm run dev          # starts Vite dev server
```

`setup.mjs` copies the WASM glue files (`org_eclipse_elk_wasm.js`, `org_eclipse_elk_wasm_bg.wasm`, `org_eclipse_elk_wasm.d.ts`) from the workspace build output into `src/wasm/`.

### Build

```bash
npm run build        # produces dist/ with editor, examples, index pages
npm run test         # runs vitest (parser unit tests + E2E parity)
```

Build-time version injection: `__APP_VERSION__` is defined from `package.json` version via Vite `define` — no hardcoded version strings in HTML.

### Differences from Original elk-live

| Aspect | Original (Sprotty) | elk-rs (this) |
|--------|-------------------|---------------|
| Layout engine | Java ELK via WebSocket | elk-rs WASM (client-side) |
| Rendering | Sprotty framework (TypeScript) | Lightweight SVG renderer (~400 LOC) |
| Editor | Monaco | Monaco |
| Bundler | Webpack | Vite |
| Server | Eclipse Jetty + WebSocket | Static files only |
| Animation | Sprotty moveModule/fadeModule | SVG attribute interpolation (compatible) |
| Viewport | `scale(s) translate(tx,ty)` on root `<g>` | Same approach (no viewBox) |
| Dark mode | CSS filter invert | Same approach |
| Examples | Server-side file listing | Vite `import.meta.glob` at build time |

### Changed Files

| File | Description |
|------|-------------|
| `.gitmodules` | Added `external/elk-live` submodule reference |
| `external/elk-live` | Reference submodule (original Sprotty-based elk-live) |
| `packages/elk-live/` | All files listed in Architecture section above |

### Test Coverage

| Scope | Tests | File |
|-------|-------|------|
| ELKT parser unit | tokenizer + parser cases | `test/elkt-parser.test.ts` |
| Example parser unit | annotation parsing + category tree | `test/elkex-parser.test.ts` |
| E2E parity | parse → NAPI layout → compare with model parity reference | `test/all-examples-wasm.test.ts` |
