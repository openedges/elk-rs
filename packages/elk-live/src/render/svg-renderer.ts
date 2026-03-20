/**
 * Renders a laid-out ELK JSON graph as SVG with pan/zoom support.
 *
 * Animation model (Sprotty-compatible):
 * - moveModule: persisted elements (matched by data-elk-id) animate from old
 *   position to new position via JS requestAnimationFrame interpolation on
 *   SVG transform attribute (not CSS transform — avoids viewBox/px mismatch).
 * - fadeModule: new elements fade in (opacity 0→1), removed elements fade out
 *   (opacity 1→0) via SVG opacity attribute interpolation.
 *
 * Structure: every logical element is wrapped in a <g data-elk-id="...">
 * with SVG transform="translate(x,y)". Child shapes are at local origin.
 */
import type { ElkNode, ElkPort, ElkEdge, ElkLabel, ElkEdgeSection, ElkPoint } from '../elk/elk-types';

const NS = 'http://www.w3.org/2000/svg';
const ANIM_MS = 300;

function svgEl<K extends keyof SVGElementTagNameMap>(tag: K, attrs: Record<string, string | number> = {}): SVGElementTagNameMap[K] {
  const el = document.createElementNS(NS, tag);
  for (const [k, v] of Object.entries(attrs)) {
    el.setAttribute(k, String(v));
  }
  return el;
}

function easeInOut(t: number): number {
  return t < 0.5 ? 2 * t * t : 1 - (-2 * t + 2) ** 2 / 2;
}

/** Snapshot of a positioned <g>'s translate. */
interface Snap { tx: number; ty: number; }

/** Read SVG transform="translate(x,y)" from a <g>. */
function readTranslate(g: SVGGElement): Snap {
  const t = g.getAttribute('transform') || '';
  const m = t.match(/translate\(\s*([-\d.e]+)[,\s]+([-\d.e]+)\s*\)/);
  return m ? { tx: parseFloat(m[1]), ty: parseFloat(m[2]) } : { tx: 0, ty: 0 };
}

export class SvgRenderer {
  private svg: SVGSVGElement;
  private content: SVGGElement;
  private scale = 1;
  private scroll = { x: 0, y: 0 };
  private isPanning = false;
  private panStart = { x: 0, y: 0 };
  private animId = 0; // cancel in-flight animation
  private onMouseMove: (e: MouseEvent) => void = () => {};
  private onMouseUp: () => void = () => {};

  constructor(container: HTMLElement) {
    this.svg = svgEl('svg');
    this.svg.style.cursor = 'grab';
    this.content = svgEl('g');
    this.content.setAttribute('transform', 'scale(1) translate(0,0)');
    this.svg.appendChild(this.content);
    container.appendChild(this.svg);

    this.setupInteraction();
  }

  render(graph: ElkNode, fitToScreen = false): void {
    // Cancel any running animation
    this.animId++;
    const thisAnim = this.animId;

    // 1. Snapshot old element positions
    const snapshot = new Map<string, Snap>();
    for (const g of this.content.querySelectorAll<SVGGElement>('[data-elk-id]')) {
      snapshot.set(g.getAttribute('data-elk-id')!, readTranslate(g));
    }

    // 2. Clear and render new graph
    while (this.content.firstChild) {
      this.content.removeChild(this.content.firstChild);
    }

    this.renderNode(graph, this.content, 0, 0);
    this.renderEdgesRecursive(graph, this.content, 0, 0);

    // 3. Set viewport
    if (fitToScreen) {
      this.fitToScreen(graph);
    } else {
      this.fitToContent(graph);
    }

    // 4. Collect new element positions and build animation targets
    interface AnimTarget {
      g: SVGGElement;
      fromTx: number; fromTy: number;
      toTx: number; toTy: number;
      fadeIn: boolean; // true = new element (opacity 0→1)
    }
    const targets: AnimTarget[] = [];

    for (const g of this.content.querySelectorAll<SVGGElement>('[data-elk-id]')) {
      const id = g.getAttribute('data-elk-id')!;
      const to = readTranslate(g);
      const old = snapshot.get(id);
      if (old) {
        // Persisted element → move from old to new
        if (old.tx !== to.tx || old.ty !== to.ty) {
          targets.push({ g, fromTx: old.tx, fromTy: old.ty, toTx: to.tx, toTy: to.ty, fadeIn: false });
          // Set to old position immediately
          g.setAttribute('transform', `translate(${old.tx},${old.ty})`);
        }
      } else {
        // New element → fade in
        targets.push({ g, fromTx: to.tx, fromTy: to.ty, toTx: to.tx, toTy: to.ty, fadeIn: true });
        g.setAttribute('opacity', '0');
      }
    }

    // 5. No targets → no animation needed
    if (targets.length === 0) return;

    // 6. Animate via requestAnimationFrame (SVG attributes, not CSS)
    const start = performance.now();
    const step = (now: number) => {
      if (thisAnim !== this.animId) return; // cancelled
      const t = Math.min((now - start) / ANIM_MS, 1);
      const e = easeInOut(t);

      for (const { g, fromTx, fromTy, toTx, toTy, fadeIn } of targets) {
        if (fadeIn) {
          g.setAttribute('opacity', String(e));
        } else {
          const cx = fromTx + (toTx - fromTx) * e;
          const cy = fromTy + (toTy - fromTy) * e;
          g.setAttribute('transform', `translate(${cx},${cy})`);
        }
      }

      if (t < 1) {
        requestAnimationFrame(step);
      } else {
        // Ensure final values are exact
        for (const { g, toTx, toTy, fadeIn } of targets) {
          if (fadeIn) {
            g.removeAttribute('opacity');
          } else {
            g.setAttribute('transform', `translate(${toTx},${toTy})`);
          }
        }
      }
    };
    requestAnimationFrame(step);
  }

  // ─── Render methods ───────────────────────────────────────────────────────

  private renderNode(node: ElkNode, parent: SVGGElement, offsetX: number, offsetY: number): void {
    const x = (node.x ?? 0) + offsetX;
    const y = (node.y ?? 0) + offsetY;
    const w = node.width ?? 0;
    const h = node.height ?? 0;

    if (node.id !== 'root') {
      const g = svgEl('g');
      g.setAttribute('data-elk-id', `node:${node.id}`);
      g.setAttribute('transform', `translate(${x},${y})`);

      if (w > 0 || h > 0) {
        g.appendChild(svgEl('rect', { x: 0, y: 0, width: w, height: h, class: 'elknode' }));
      } else {
        g.appendChild(svgEl('rect', { x: -2, y: -2, width: 4, height: 4, class: 'elknode elknode-marker' }));
      }
      parent.appendChild(g);
    }

    if (node.labels) {
      node.labels.forEach((label, i) => {
        this.renderLabel(label, parent, x, y, `label:${node.id}:${i}`);
      });
    }

    if (node.ports) {
      for (const port of node.ports) {
        this.renderPort(port, parent, x, y);
      }
    }

    if (node.children) {
      for (const child of node.children) {
        this.renderNode(child, parent, x, y);
      }
    }
  }

  private renderPort(port: ElkPort, parent: SVGGElement, offsetX: number, offsetY: number): void {
    const x = (port.x ?? 0) + offsetX;
    const y = (port.y ?? 0) + offsetY;
    const w = port.width ?? 5;
    const h = port.height ?? 5;

    const g = svgEl('g');
    g.setAttribute('data-elk-id', `port:${port.id}`);
    g.setAttribute('transform', `translate(${x},${y})`);
    g.appendChild(svgEl('rect', { x: 0, y: 0, width: w, height: h, class: 'elkport' }));
    parent.appendChild(g);

    if (port.labels) {
      port.labels.forEach((label, i) => {
        this.renderLabel(label, parent, x, y, `portlabel:${port.id}:${i}`);
      });
    }
  }

  private renderLabel(label: ElkLabel, parent: SVGGElement, offsetX: number, offsetY: number, elkId: string): void {
    if (!label.text) return;
    const x = (label.x ?? 0) + offsetX;
    const y = (label.y ?? 0) + offsetY;

    const g = svgEl('g');
    g.setAttribute('data-elk-id', elkId);
    g.setAttribute('transform', `translate(${x},${y})`);
    const text = svgEl('text', { x: 0, y: 0, class: 'elklabel' });
    text.textContent = label.text;
    g.appendChild(text);
    parent.appendChild(g);
  }

  private renderEdgesRecursive(node: ElkNode, parent: SVGGElement, offsetX: number, offsetY: number): void {
    const x = (node.x ?? 0) + offsetX;
    const y = (node.y ?? 0) + offsetY;
    const edgeOffsetX = node.id === 'root' ? offsetX : x;
    const edgeOffsetY = node.id === 'root' ? offsetY : y;

    if (node.edges) {
      for (const edge of node.edges) {
        this.renderEdge(edge, parent, edgeOffsetX, edgeOffsetY);
      }
    }

    if (node.children) {
      for (const child of node.children) {
        this.renderEdgesRecursive(child, parent, edgeOffsetX, edgeOffsetY);
      }
    }
  }

  private renderEdge(edge: ElkEdge, parent: SVGGElement, offsetX: number, offsetY: number): void {
    if (!edge.sections || edge.sections.length === 0) return;

    edge.sections.forEach((section, si) => {
      this.renderEdgeSection(section, parent, offsetX, offsetY, edge.id, si);
    });

    if (edge.junctionPoints) {
      edge.junctionPoints.forEach((jp, ji) => {
        const g = svgEl('g');
        g.setAttribute('data-elk-id', `junction:${edge.id}:${ji}`);
        g.setAttribute('transform', `translate(${jp.x + offsetX},${jp.y + offsetY})`);
        g.appendChild(svgEl('circle', { cx: 0, cy: 0, r: 2, class: 'elkjunction' }));
        parent.appendChild(g);
      });
    }

    if (edge.labels) {
      edge.labels.forEach((label, li) => {
        this.renderLabel(label, parent, offsetX, offsetY, `edgelabel:${edge.id}:${li}`);
      });
    }
  }

  private renderEdgeSection(
    section: ElkEdgeSection, parent: SVGGElement,
    offsetX: number, offsetY: number,
    edgeId: string, sectionIndex: number,
  ): void {
    const points: ElkPoint[] = [section.startPoint];
    if (section.bendPoints) points.push(...section.bendPoints);
    points.push(section.endPoint);

    // Edge path — wrap in <g> with translate(0,0) for fade-in animation tracking
    const g = svgEl('g');
    g.setAttribute('data-elk-id', `edge:${edgeId}:s${sectionIndex}`);
    g.setAttribute('transform', 'translate(0,0)');

    let d = `M ${points[0].x + offsetX},${points[0].y + offsetY}`;
    for (let i = 1; i < points.length; i++) {
      d += ` L ${points[i].x + offsetX},${points[i].y + offsetY}`;
    }
    g.appendChild(svgEl('path', { d, class: 'elkedge' }));
    parent.appendChild(g);

    // Arrowhead
    const last = points[points.length - 1];
    const prev = points[points.length - 2];
    const angle = Math.atan2(prev.y - last.y, prev.x - last.x) * 180 / Math.PI;
    const tx = last.x + offsetX;
    const ty = last.y + offsetY;

    const arrowG = svgEl('g');
    arrowG.setAttribute('data-elk-id', `edge:${edgeId}:s${sectionIndex}:arrow`);
    arrowG.setAttribute('transform', 'translate(0,0)');
    arrowG.appendChild(svgEl('path', {
      d: 'M 0,0 L 8,-3 L 8,3 Z',
      class: 'elkedge arrow',
      transform: `rotate(${angle} ${tx} ${ty}) translate(${tx} ${ty})`,
    }));
    parent.appendChild(arrowG);
  }

  // ─── Viewport ─────────────────────────────────────────────────────────────

  private fitToScreen(graph: ElkNode): void {
    const gw = graph.width ?? 0;
    const gh = graph.height ?? 0;
    if (gw === 0 && gh === 0) return;

    const padding = 20;
    const svgRect = this.svg.getBoundingClientRect();
    const containerW = svgRect.width || 800;
    const containerH = svgRect.height || 600;

    const scaleX = containerW / (gw + padding * 2);
    const scaleY = containerH / (gh + padding * 2);
    this.scale = Math.min(scaleX, scaleY);
    this.scroll = {
      x: (containerW / this.scale - gw) / 2,
      y: (containerH / this.scale - gh) / 2,
    };
    this.updateTransform();
  }

  /** Sprotty-compatible: scale(1), graph at origin. */
  private fitToContent(_graph: ElkNode): void {
    this.scale = 1;
    this.scroll = { x: 0, y: 0 };
    this.updateTransform();
  }

  private updateTransform(): void {
    this.content.setAttribute('transform',
      `scale(${this.scale}) translate(${this.scroll.x},${this.scroll.y})`);
  }

  // ─── Interaction ──────────────────────────────────────────────────────────

  private setupInteraction(): void {
    this.svg.addEventListener('mousedown', (e) => {
      if (e.button === 0) {
        this.isPanning = true;
        this.panStart = { x: e.clientX, y: e.clientY };
        this.svg.style.cursor = 'grabbing';
      }
    });

    this.onMouseMove = (e: MouseEvent) => {
      if (!this.isPanning) return;
      const dx = e.clientX - this.panStart.x;
      const dy = e.clientY - this.panStart.y;
      this.panStart = { x: e.clientX, y: e.clientY };

      // Pan: convert screen pixels to SVG units
      this.scroll.x += dx / this.scale;
      this.scroll.y += dy / this.scale;
      this.updateTransform();
    };

    this.onMouseUp = () => {
      this.isPanning = false;
      this.svg.style.cursor = 'grab';
    };

    window.addEventListener('mousemove', this.onMouseMove);
    window.addEventListener('mouseup', this.onMouseUp);

    this.svg.addEventListener('wheel', (e) => {
      e.preventDefault();
      const factor = e.deltaY > 0 ? 0.9 : 1.1;
      const svgRect = this.svg.getBoundingClientRect();
      const mx = e.clientX - svgRect.left;
      const my = e.clientY - svgRect.top;

      // Zoom around cursor: keep the SVG point under cursor fixed
      // Before: screenPt = scale * (scroll + svgPt)
      // After:  screenPt = newScale * (newScroll + svgPt)
      const newScale = this.scale * factor;
      this.scroll.x += mx / this.scale * (1 - 1 / factor);
      this.scroll.y += my / this.scale * (1 - 1 / factor);
      this.scale = newScale;
      this.updateTransform();
    }, { passive: false });
  }

  destroy(): void {
    window.removeEventListener('mousemove', this.onMouseMove);
    window.removeEventListener('mouseup', this.onMouseUp);
    this.svg.remove();
  }
}
