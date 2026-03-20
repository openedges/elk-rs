import * as monaco from 'monaco-editor';
import showdown from 'showdown';
import { setupDarkMode } from './common/dark-mode';
import { getParams } from './common/url-params';
import { registerElktLanguage } from './common/elkt-language';
import { layoutGraph, initElk } from './elk/elk-layout';
import { parseElkt, applyDefaults } from './elkt/parser';
import { SvgRenderer } from './render/svg-renderer';
import { parseElkExample, buildCategoryTree } from './elkex/parser';
import type { ElkExample, ExampleCategory } from './elkex/parser';

// ─── Monaco worker setup ─────────────────────────────────────────────────────

self.MonacoEnvironment = {
  getWorker(_workerId: string, _label: string) {
    return new Worker(
      new URL('monaco-editor/esm/vs/editor/editor.worker.js', import.meta.url),
      { type: 'module' }
    );
  },
};

// ─── Setup ───────────────────────────────────────────────────────────────────

setupDarkMode();

showdown.setFlavor('github');
const mdConverter = new showdown.Converter({ simpleLineBreaks: false });

const sidebar = document.getElementById('sidebar')!;
const titleEl = document.getElementById('example-title')!;
const categoryPathEl = document.getElementById('category-path')!;
const descriptionEl = document.getElementById('example-description')!;
const loading = document.getElementById('loading')!;
const errorBar = document.getElementById('error-bar')!;

// ─── Monaco Editor ───────────────────────────────────────────────────────────

registerElktLanguage();

const editor = monaco.editor.create(document.getElementById('monaco-editor')!, {
  value: '',
  language: 'elkt',
  theme: 'vs',
});
editor.updateOptions({
  minimap: { enabled: false },
  scrollBeyondLastLine: false,
});

// Hide editor loading spinner
document.getElementById('loading-editor')!.style.display = 'none';

// ─── SVG Renderer ────────────────────────────────────────────────────────────

const renderer = new SvgRenderer(document.getElementById('diagram')!);

// ─── Load examples ───────────────────────────────────────────────────────────

const exampleFiles = import.meta.glob(
  '../../../external/elk-models/examples/**/*.elkt',
  { query: '?raw', import: 'default', eager: true }
) as Record<string, string>;

const examples: ElkExample[] = [];
for (const [filePath, content] of Object.entries(exampleFiles)) {
  try {
    // Extract path relative to examples/
    const match = filePath.match(/examples\/(.+)\.elkt$/);
    if (!match) continue;
    const path = match[1];
    examples.push(parseElkExample(path, content));
  } catch {
    // Skip malformed examples
  }
}

const categoryTree = buildCategoryTree(examples);

// ─── Build navigation ────────────────────────────────────────────────────────

let activeBtn: HTMLButtonElement | null = null;

function buildNav(category: ExampleCategory, depth = 0, namePrefix = '') {
  const name = category.name;
  if (name !== 'root') {
    const heading = document.createElement('h6');
    heading.className = 'sidebar-heading';
    heading.style.paddingLeft = `${depth * 8}px`;
    heading.textContent = namePrefix + name;
    sidebar.appendChild(heading);
  }

  const sortedElements = [...category.elements].sort((a, b) => a.label.localeCompare(b.label));
  for (const ex of sortedElements) {
    const li = document.createElement('li');
    const btn = document.createElement('button');
    btn.type = 'button';
    btn.className = 'sidebar-link';
    btn.style.paddingLeft = `${depth * 8 + 12}px`;
    btn.textContent = ex.label;
    btn.onclick = () => loadExample(ex, btn);
    li.appendChild(btn);
    sidebar.appendChild(li);
  }

  const newPrefix = name !== 'root' ? `${namePrefix}${name} > ` : namePrefix;
  const sortedSubs = [...category.subCategories].sort((a, b) => a.name.localeCompare(b.name));
  for (const sub of sortedSubs) {
    buildNav(sub, depth + 1, newPrefix);
  }
}

buildNav(categoryTree);

// ─── Layout logic ────────────────────────────────────────────────────────────

let layoutTimer: ReturnType<typeof setTimeout> | null = null;

async function runLayout() {
  loading.style.display = 'block';
  errorBar.style.display = 'none';
  try {
    const text = editor.getValue();
    const graph = parseElkt(text);
    applyDefaults(graph);
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
  layoutTimer = setTimeout(runLayout, 400);
}

editor.onDidChangeModelContent(() => scheduleLayout());

// ─── Load example ────────────────────────────────────────────────────────────

function loadExample(example: ElkExample, btn?: HTMLButtonElement) {
  titleEl.textContent = example.label;
  categoryPathEl.textContent = example.category.join(' > ');
  descriptionEl.innerHTML = mdConverter.makeHtml(example.doc);
  editor.setValue(example.graph);
  editor.setPosition({ lineNumber: 1, column: 1 });

  if (activeBtn) activeBtn.classList.remove('active');
  if (btn) { btn.classList.add('active'); activeBtn = btn; }

  // Update URL
  const url = new URL(window.location.href);
  url.searchParams.set('e', encodeURIComponent(example.path));
  window.history.pushState({ e: example.path }, '', url.toString());

  scheduleLayout();
}

// ─── Initial load ────────────────────────────────────────────────────────────

const params = getParams();
const initialPath = params.e ? decodeURIComponent(params.e) : null;

initElk().then(() => {
  if (initialPath) {
    const found = examples.find(e => e.path === initialPath);
    if (found) {
      // Find and click the corresponding button
      const buttons = sidebar.querySelectorAll<HTMLButtonElement>('.sidebar-link');
      for (const btn of buttons) {
        if (btn.textContent === found.label) {
          loadExample(found, btn);
          btn.scrollIntoView({ block: 'center' });
          return;
        }
      }
      loadExample(found);
      return;
    }
  }
  // Load random example
  if (examples.length > 0) {
    const idx = Math.floor(Math.random() * examples.length);
    const buttons = sidebar.querySelectorAll<HTMLButtonElement>('.sidebar-link');
    if (buttons[idx]) {
      loadExample(examples[idx], buttons[idx]);
    } else {
      loadExample(examples[idx]);
    }
  }
}).catch(err => {
  console.error('Failed to init ELK:', err);
});

// Set version from build-time constant
document.getElementById('app-version')!.textContent = `v${__APP_VERSION__}`;

// Resize Monaco editor on window resize
window.onresize = () => editor.layout();

// Browser back/forward
window.onpopstate = (event) => {
  if (event.state?.e) {
    const found = examples.find(e => e.path === event.state.e);
    if (found) loadExample(found);
  }
};
