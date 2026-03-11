import * as monaco from 'monaco-editor';
import JSON5 from 'json5';
import LZString from 'lz-string';
import { setupDarkMode } from './common/dark-mode';
import { getParams } from './common/url-params';
import { registerElktLanguage } from './common/elkt-language';
import { layoutGraph, initElk } from './elk/elk-layout';
import { parseElkt, applyDefaults } from './elkt/parser';
import { SvgRenderer } from './render/svg-renderer';
import type { ElkNode } from './elk/elk-types';

// ─── Monaco worker setup ─────────────────────────────────────────────────────

self.MonacoEnvironment = {
  getWorker(_workerId: string, label: string) {
    if (label === 'json') {
      return new Worker(
        new URL('monaco-editor/esm/vs/language/json/json.worker.js', import.meta.url),
        { type: 'module' }
      );
    }
    return new Worker(
      new URL('monaco-editor/esm/vs/editor/editor.worker.js', import.meta.url),
      { type: 'module' }
    );
  },
};

// ─── State ───────────────────────────────────────────────────────────────────

const params = getParams();
let currentMode: 'elkt' | 'json' = (params.mode === 'json') ? 'json' : 'elkt';

const DEFAULT_ELKT = `algorithm: layered

node n1
node n2
node n3
edge n1 -> n2
edge n1 -> n3`;

const DEFAULT_JSON = `{
  "id": "root",
  "layoutOptions": { "elk.algorithm": "layered" },
  "children": [
    { "id": "n1", "width": 30, "height": 30 },
    { "id": "n2", "width": 30, "height": 30 },
    { "id": "n3", "width": 30, "height": 30 }
  ],
  "edges": [
    { "id": "e1", "sources": [ "n1" ], "targets": [ "n2" ] },
    { "id": "e2", "sources": [ "n1" ], "targets": [ "n3" ] }
  ]
}`;

// ─── Initialize ──────────────────────────────────────────────────────────────

setupDarkMode();

const loading = document.getElementById('loading')!;
const errorBar = document.getElementById('error-bar')!;
const modeSelect = document.getElementById('mode-select') as HTMLSelectElement;
const pageTitle = document.getElementById('page-title')!;

// Restore content from URL
let initialContent: string;
if (params.compressedContent) {
  initialContent = LZString.decompressFromEncodedURIComponent(params.compressedContent)!;
} else if (params.initialContent) {
  initialContent = decodeURIComponent(params.initialContent);
} else {
  initialContent = currentMode === 'json' ? DEFAULT_JSON : DEFAULT_ELKT;
}

// Set mode selector
modeSelect.value = currentMode;
updateTitle();

// ─── Monaco Editor ───────────────────────────────────────────────────────────

registerElktLanguage();
const editorLanguage = currentMode === 'json' ? 'json' : 'elkt';

const editor = monaco.editor.create(document.getElementById('monaco-editor')!, {
  value: initialContent,
  language: editorLanguage,
  theme: 'vs',
});
editor.updateOptions({
  minimap: { enabled: false },
});

// Hide editor loading spinner
document.getElementById('loading-editor')!.style.display = 'none';

// ─── SVG Renderer ────────────────────────────────────────────────────────────

const renderer = new SvgRenderer(document.getElementById('diagram')!);

// ─── Layout logic ────────────────────────────────────────────────────────────

let layoutTimer: ReturnType<typeof setTimeout> | null = null;

async function updateLayout() {
  loading.style.display = 'block';
  errorBar.style.display = 'none';

  try {
    const text = editor.getValue();
    let graph: ElkNode;

    if (currentMode === 'elkt') {
      graph = parseElkt(text);
      applyDefaults(graph);
    } else {
      graph = JSON5.parse(text);
    }

    // Ensure edge coordinates are PARENT-relative for correct rendering
    if (!graph.properties) graph.properties = {};
    graph.properties['org.eclipse.elk.json.edgeCoords'] = 'PARENT';

    const result = await layoutGraph(graph);
    renderer.render(result);

    // Clear Monaco markers
    const model = editor.getModel();
    if (model) monaco.editor.setModelMarkers(model, 'elk', []);
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : String(err);
    errorBar.textContent = message;
    errorBar.style.display = 'block';

    // Set marker on editor if we have line info
    const model = editor.getModel();
    if (model) {
      const lineMatch = message.match(/line (\d+)/);
      const lineNumber = lineMatch ? parseInt(lineMatch[1]) : 1;
      monaco.editor.setModelMarkers(model, 'elk', [{
        severity: monaco.MarkerSeverity.Error,
        startLineNumber: lineNumber,
        startColumn: 1,
        endLineNumber: lineNumber,
        endColumn: model.getLineMaxColumn(lineNumber),
        message,
      }]);
    }
  } finally {
    loading.style.display = 'none';
  }
}

function scheduleLayout() {
  if (layoutTimer) clearTimeout(layoutTimer);
  layoutTimer = setTimeout(updateLayout, 300);
}

// ─── Event handlers ──────────────────────────────────────────────────────────

editor.onDidChangeModelContent(() => {
  scheduleLayout();
  updateModelLink();
});

modeSelect.onchange = () => {
  const newMode = modeSelect.value as 'elkt' | 'json';
  if (newMode === currentMode) return;
  currentMode = newMode;

  // Update URL
  const url = new URL(window.location.href);
  url.searchParams.set('mode', currentMode);
  url.searchParams.delete('compressedContent');
  window.history.replaceState(null, '', url.toString());

  // Update editor language
  const model = editor.getModel();
  if (model) {
    monaco.editor.setModelLanguage(model, currentMode === 'json' ? 'json' : 'elkt');
  }

  // Set default content for the new mode
  editor.setValue(currentMode === 'json' ? DEFAULT_JSON : DEFAULT_ELKT);

  updateTitle();
};

function updateTitle() {
  pageTitle.textContent = currentMode === 'json' ? 'ELK JSON Editor' : 'ELKT Editor';
  document.title = currentMode === 'json' ? 'ELK-RS Editor (json)' : 'ELK-RS Editor (elkt)';
}

function updateModelLink() {
  const anchor = document.getElementById('model-link') as HTMLAnchorElement;
  if (!anchor) return;
  const compressed = LZString.compressToEncodedURIComponent(editor.getValue());
  const url = new URL(window.location.href);
  url.searchParams.set('mode', currentMode);
  url.searchParams.set('compressedContent', compressed);
  anchor.href = url.toString();
}

// ─── Init ────────────────────────────────────────────────────────────────────

initElk().then(() => {
  updateLayout();
  updateModelLink();
}).catch((err) => {
  errorBar.textContent = `Failed to load WASM: ${err.message}`;
  errorBar.style.display = 'block';
});

// Set version from build-time constant
document.getElementById('app-version')!.textContent = `v${__APP_VERSION__}`;

// Resize the monaco editor upon window resize (matches original elk-live behavior)
window.onresize = () => editor.layout();
