/**
 * ELKT text format parser → ELK JSON graph.
 *
 * Supports: nodes, ports, edges, labels, layout options, nested hierarchies.
 * Does NOT support: layout sections (position/size syntax), elkg format.
 */
import type { ElkNode, ElkPort, ElkEdge, ElkLabel, ElkShape } from '../elk/elk-types';

// ─── Tokenizer ───────────────────────────────────────────────────────────────

enum TokenType {
  KEYWORD,    // node, port, edge, label
  ID,         // identifier (may contain dots for qualified IDs)
  STRING,     // "..." or '...'
  NUMBER,     // integer or decimal
  BOOLEAN,    // true, false
  NULL,       // null
  COLON,      // :
  ARROW,      // ->
  LBRACE,     // {
  RBRACE,     // }
  LBRACKET,   // [
  RBRACKET,   // ]
  COMMA,      // ,
  EOF,
}

interface Token {
  type: TokenType;
  value: string;
  line: number;
  col: number;
}

const KEYWORDS = new Set(['node', 'port', 'edge', 'label', 'layout']);

function tokenize(input: string): Token[] {
  const tokens: Token[] = [];
  let pos = 0;
  let line = 1;
  let col = 1;

  function advance(n = 1) {
    for (let i = 0; i < n; i++) {
      if (input[pos] === '\n') { line++; col = 1; }
      else { col++; }
      pos++;
    }
  }

  function peek(offset = 0): string {
    return input[pos + offset] || '';
  }

  while (pos < input.length) {
    // Skip whitespace
    if (/\s/.test(peek())) {
      advance();
      continue;
    }

    // Line comment
    if (peek() === '/' && peek(1) === '/') {
      while (pos < input.length && peek() !== '\n') advance();
      continue;
    }

    // Block comment
    if (peek() === '/' && peek(1) === '*') {
      advance(2);
      while (pos < input.length && !(peek() === '*' && peek(1) === '/')) advance();
      if (pos < input.length) advance(2);
      continue;
    }

    const startLine = line;
    const startCol = col;

    // Arrow ->
    if (peek() === '-' && peek(1) === '>') {
      tokens.push({ type: TokenType.ARROW, value: '->', line: startLine, col: startCol });
      advance(2);
      continue;
    }

    // Single-char tokens
    if (peek() === '{') { tokens.push({ type: TokenType.LBRACE, value: '{', line: startLine, col: startCol }); advance(); continue; }
    if (peek() === '}') { tokens.push({ type: TokenType.RBRACE, value: '}', line: startLine, col: startCol }); advance(); continue; }
    if (peek() === '[') { tokens.push({ type: TokenType.LBRACKET, value: '[', line: startLine, col: startCol }); advance(); continue; }
    if (peek() === ']') { tokens.push({ type: TokenType.RBRACKET, value: ']', line: startLine, col: startCol }); advance(); continue; }
    if (peek() === ':') { tokens.push({ type: TokenType.COLON, value: ':', line: startLine, col: startCol }); advance(); continue; }
    if (peek() === ',') { tokens.push({ type: TokenType.COMMA, value: ',', line: startLine, col: startCol }); advance(); continue; }

    // String
    if (peek() === '"' || peek() === "'") {
      const quote = peek();
      advance();
      let str = '';
      while (pos < input.length && peek() !== quote) {
        if (peek() === '\\') {
          advance();
          const esc = peek();
          if (esc === 'n') str += '\n';
          else if (esc === 't') str += '\t';
          else if (esc === '\\') str += '\\';
          else if (esc === quote) str += quote;
          else str += esc;
          advance();
        } else {
          str += peek();
          advance();
        }
      }
      if (pos < input.length) advance(); // closing quote
      tokens.push({ type: TokenType.STRING, value: str, line: startLine, col: startCol });
      continue;
    }

    // Number (including negative)
    if (/[0-9]/.test(peek()) || (peek() === '-' && /[0-9]/.test(peek(1)))) {
      let num = '';
      if (peek() === '-') { num += '-'; advance(); }
      while (pos < input.length && /[0-9]/.test(peek())) { num += peek(); advance(); }
      if (peek() === '.' && /[0-9]/.test(peek(1))) {
        num += '.'; advance();
        while (pos < input.length && /[0-9]/.test(peek())) { num += peek(); advance(); }
      }
      tokens.push({ type: TokenType.NUMBER, value: num, line: startLine, col: startCol });
      continue;
    }

    // Identifier or keyword (allows dots for qualified IDs like elk.algorithm, ^ for elk.^position)
    if (/[a-zA-Z_$^]/.test(peek())) {
      let id = '';
      while (pos < input.length && /[a-zA-Z0-9_$.^]/.test(peek())) {
        id += peek();
        advance();
      }
      // Remove trailing dots
      while (id.endsWith('.')) {
        id = id.slice(0, -1);
        pos--; col--;
      }

      if (id === 'true' || id === 'false') {
        tokens.push({ type: TokenType.BOOLEAN, value: id, line: startLine, col: startCol });
      } else if (id === 'null') {
        tokens.push({ type: TokenType.NULL, value: id, line: startLine, col: startCol });
      } else if (KEYWORDS.has(id)) {
        tokens.push({ type: TokenType.KEYWORD, value: id, line: startLine, col: startCol });
      } else {
        tokens.push({ type: TokenType.ID, value: id, line: startLine, col: startCol });
      }
      continue;
    }

    // Unknown character - skip
    advance();
  }

  tokens.push({ type: TokenType.EOF, value: '', line, col });
  return tokens;
}

// ─── Parser ──────────────────────────────────────────────────────────────────

class Parser {
  private tokens: Token[];
  private pos = 0;
  private edgeCounter = 0;
  private labelCounter = 0;

  constructor(tokens: Token[]) {
    this.tokens = tokens;
  }

  private peek(): Token {
    return this.tokens[this.pos];
  }

  private advance(): Token {
    return this.tokens[this.pos++];
  }

  private expect(type: TokenType, value?: string): Token {
    const tok = this.advance();
    if (tok.type !== type || (value !== undefined && tok.value !== value)) {
      throw new Error(`Expected ${TokenType[type]}${value ? ` '${value}'` : ''} at line ${tok.line}:${tok.col}, got ${TokenType[tok.type]} '${tok.value}'`);
    }
    return tok;
  }

  private isAt(type: TokenType, value?: string): boolean {
    const tok = this.peek();
    return tok.type === type && (value === undefined || tok.value === value);
  }

  parseRoot(): ElkNode {
    const node: ElkNode = { id: 'root' };
    this.parseBody(node, true, '');
    // Java ELKT parser sets randomSeed: 1 as default on all graphs
    if (!node.layoutOptions) node.layoutOptions = {};
    if (!node.layoutOptions['randomSeed']) {
      node.layoutOptions['randomSeed'] = '1';
    }
    return node;
  }

  /** Qualify a local ID with the parent scope to ensure global uniqueness. */
  private qualifyId(scope: string, localId: string): string {
    return scope ? `${scope}$${localId}` : localId;
  }

  /** Qualify an edge endpoint reference, converting dot notation to $ separators.
   *  e.g., n1.p1 in scope "parent" → parent$n1$p1 (port p1 on node n1)
   *  outside.inside.n1 at root → outside$inside$n1 (hierarchical reference) */
  private qualifyEdgeRef(scope: string, ref: string): string {
    const parts = ref.split('.');
    const qualifiedFirst = this.qualifyId(scope, parts[0]);
    return parts.length > 1
      ? [qualifiedFirst, ...parts.slice(1)].join('$')
      : qualifiedFirst;
  }

  private parseBody(parent: ElkNode, isRoot: boolean, scope: string) {
    while (!this.isAt(TokenType.EOF) && (isRoot || !this.isAt(TokenType.RBRACE))) {
      if (this.isAt(TokenType.KEYWORD, 'node')) {
        this.parseNode(parent, scope);
      } else if (this.isAt(TokenType.KEYWORD, 'port')) {
        this.parsePort(parent, scope);
      } else if (this.isAt(TokenType.KEYWORD, 'edge')) {
        this.parseEdge(parent, scope);
      } else if (this.isAt(TokenType.KEYWORD, 'label')) {
        this.parseLabel(parent, scope);
      } else if (this.isAt(TokenType.KEYWORD, 'layout')) {
        this.parseLayoutSection(parent);
      } else if (this.isAt(TokenType.ID) || this.isAt(TokenType.STRING)) {
        this.parseProperty(parent);
      } else {
        // Skip unexpected tokens
        this.advance();
      }
    }
  }

  /**
   * Parse `layout [ size: W, H ]` and `layout [ position: X, Y ]` sections.
   * These set width/height and x/y on the shape.
   */
  private parseLayoutSection(shape: ElkShape) {
    this.expect(TokenType.KEYWORD, 'layout');
    this.expect(TokenType.LBRACKET);

    while (!this.isAt(TokenType.EOF) && !this.isAt(TokenType.RBRACKET)) {
      if (this.isAt(TokenType.ID, 'size')) {
        this.advance();
        this.expect(TokenType.COLON);
        const w = parseFloat(this.expect(TokenType.NUMBER).value);
        this.expect(TokenType.COMMA);
        const h = parseFloat(this.expect(TokenType.NUMBER).value);
        shape.width = w;
        shape.height = h;
      } else if (this.isAt(TokenType.ID, 'position')) {
        this.advance();
        this.expect(TokenType.COLON);
        const x = parseFloat(this.expect(TokenType.NUMBER).value);
        this.expect(TokenType.COMMA);
        const y = parseFloat(this.expect(TokenType.NUMBER).value);
        shape.x = x;
        shape.y = y;
      } else {
        // Skip unknown layout properties
        this.advance();
      }
    }

    this.expect(TokenType.RBRACKET);
  }

  private parseNode(parent: ElkNode, scope: string) {
    this.expect(TokenType.KEYWORD, 'node');
    const localId = this.advance().value;
    const qualifiedId = this.qualifyId(scope, localId);
    const node: ElkNode = { id: qualifiedId };

    if (this.isAt(TokenType.LBRACE)) {
      this.advance();
      this.parseBody(node, false, qualifiedId);
      this.expect(TokenType.RBRACE);
    }

    if (!parent.children) parent.children = [];
    parent.children.push(node);
  }

  private parsePort(parent: ElkNode, scope: string) {
    this.expect(TokenType.KEYWORD, 'port');
    const localId = this.advance().value;
    const qualifiedId = this.qualifyId(scope, localId);
    const port: ElkPort = { id: qualifiedId };

    if (this.isAt(TokenType.LBRACE)) {
      this.advance();
      this.parseShapeBody(port, qualifiedId);
      this.expect(TokenType.RBRACE);
    }

    if (!parent.ports) parent.ports = [];
    parent.ports.push(port);
  }

  private parseShapeBody(shape: ElkShape, scope: string) {
    while (!this.isAt(TokenType.EOF) && !this.isAt(TokenType.RBRACE)) {
      if (this.isAt(TokenType.KEYWORD, 'label')) {
        this.parseLabelOnShape(shape, scope);
      } else if (this.isAt(TokenType.KEYWORD, 'layout')) {
        this.parseLayoutSection(shape);
      } else if (this.isAt(TokenType.ID) || this.isAt(TokenType.STRING)) {
        this.parsePropertyOnShape(shape);
      } else {
        this.advance();
      }
    }
  }

  private parseEdge(parent: ElkNode, scope: string) {
    this.expect(TokenType.KEYWORD, 'edge');

    let edgeId: string | undefined;

    // Read first ID - could be edge ID or first source
    const firstId = this.advance().value;

    // If followed by ':', it's the edge ID
    if (this.isAt(TokenType.COLON)) {
      this.advance();
      edgeId = firstId;
    }

    // Read sources (qualify local IDs with scope, handling dot notation for ports)
    const sources: string[] = [];
    if (edgeId !== undefined) {
      sources.push(this.qualifyEdgeRef(scope, this.advance().value));
    } else {
      sources.push(this.qualifyEdgeRef(scope, firstId));
    }
    while (this.isAt(TokenType.COMMA)) {
      this.advance();
      sources.push(this.qualifyEdgeRef(scope, this.advance().value));
    }

    // Arrow
    this.expect(TokenType.ARROW);

    // Read targets (qualify local IDs with scope, handling dot notation for ports)
    const targets: string[] = [];
    targets.push(this.qualifyEdgeRef(scope, this.advance().value));
    while (this.isAt(TokenType.COMMA)) {
      this.advance();
      targets.push(this.qualifyEdgeRef(scope, this.advance().value));
    }

    const edge: ElkEdge = {
      id: this.qualifyId(scope, edgeId || `e${this.edgeCounter++}`),
      sources,
      targets,
    };

    if (this.isAt(TokenType.LBRACE)) {
      this.advance();
      this.parseEdgeBody(edge, scope);
      this.expect(TokenType.RBRACE);
    }

    if (!parent.edges) parent.edges = [];
    parent.edges.push(edge);
  }

  private parseEdgeBody(edge: ElkEdge, scope: string) {
    while (!this.isAt(TokenType.EOF) && !this.isAt(TokenType.RBRACE)) {
      if (this.isAt(TokenType.KEYWORD, 'label')) {
        const label = this.parseLabelElement(scope);
        if (!edge.labels) edge.labels = [];
        edge.labels.push(label);
      } else if (this.isAt(TokenType.KEYWORD, 'layout')) {
        this.parseLayoutSection(edge as unknown as ElkShape);
      } else if (this.isAt(TokenType.ID) || this.isAt(TokenType.STRING)) {
        this.parsePropertyOnEdge(edge);
      } else {
        this.advance();
      }
    }
  }

  private parseLabel(parent: ElkNode, scope: string) {
    const label = this.parseLabelElement(scope);
    if (!parent.labels) parent.labels = [];
    parent.labels.push(label);
  }

  private parseLabelOnShape(shape: ElkShape, scope: string) {
    const label = this.parseLabelElement(scope);
    if (!shape.labels) shape.labels = [];
    shape.labels.push(label);
  }

  private parseLabelElement(scope: string): ElkLabel {
    this.expect(TokenType.KEYWORD, 'label');

    let labelId: string | undefined;
    let text: string;

    // Check if there's an ID: prefix
    if (this.isAt(TokenType.ID)) {
      const savedPos = this.pos;
      const id = this.advance().value;
      if (this.isAt(TokenType.COLON)) {
        this.advance();
        labelId = id;
        text = this.advance().value;
      } else {
        // Not an ID prefix, restore and read as text
        this.pos = savedPos;
        text = this.advance().value;
      }
    } else {
      text = this.advance().value;
    }

    const label: ElkLabel = {
      id: this.qualifyId(scope, labelId || `l${this.labelCounter++}`),
      text,
    };

    if (this.isAt(TokenType.LBRACE)) {
      this.advance();
      this.parseShapeBody(label, scope);
      this.expect(TokenType.RBRACE);
    }

    return label;
  }

  private parseProperty(parent: ElkNode) {
    const rawKey = this.advance().value;
    this.expect(TokenType.COLON);
    const value = this.parseValue();

    // Special handling for size properties (before resolving key)
    if (rawKey === 'width' || rawKey === 'nodeSize.minimum.width') {
      parent.width = parseFloat(value);
    } else if (rawKey === 'height' || rawKey === 'nodeSize.minimum.height') {
      parent.height = parseFloat(value);
    } else {
      const key = this.resolvePropertyKey(rawKey);
      if (!parent.layoutOptions) parent.layoutOptions = {};
      parent.layoutOptions[key] = value;
    }
  }

  private parsePropertyOnShape(shape: ElkShape) {
    const rawKey = this.advance().value;
    this.expect(TokenType.COLON);
    const value = this.parseValue();

    if (rawKey === 'width') {
      shape.width = parseFloat(value);
    } else if (rawKey === 'height') {
      shape.height = parseFloat(value);
    } else {
      const key = this.resolvePropertyKey(rawKey);
      if (!shape.layoutOptions) shape.layoutOptions = {};
      shape.layoutOptions[key] = value;
    }
  }

  private parsePropertyOnEdge(edge: ElkEdge) {
    const key = this.resolvePropertyKey(this.advance().value);
    this.expect(TokenType.COLON);
    const value = this.parseValue();

    if (!edge.layoutOptions) edge.layoutOptions = {};
    edge.layoutOptions[key] = value;
  }

  private parseValue(): string {
    const tok = this.advance();
    switch (tok.type) {
      case TokenType.STRING:
      case TokenType.ID:
      case TokenType.NUMBER:
      case TokenType.BOOLEAN:
      case TokenType.NULL:
        return tok.value;
      default:
        throw new Error(`Unexpected value token ${TokenType[tok.type]} '${tok.value}' at line ${tok.line}:${tok.col}`);
    }
  }

  private resolvePropertyKey(key: string): string {
    // Strip ^ escape characters — ELKT convention for non-keyword identifiers.
    // e.g., ^port.side → port.side, elk.^position → elk.position
    // Then pass raw keys through — the WASM engine resolves by suffix matching.
    return key.replace(/\^/g, '');
  }
}

/**
 * Apply default sizes matching the original elk-live Java server
 * (ElkGraphDiagramGenerator.applyDefaults):
 *   - Nodes: 30x30 if width/height <= 0
 *   - Ports: 5x5 if width/height <= 0
 *   - Labels: width = text.length * 9, height = 16 if not set
 */
function applyDefaults(node: ElkNode): void {
  if (node.ports) {
    for (const port of node.ports) {
      if (!port.width || port.width <= 0) port.width = 5;
      if (!port.height || port.height <= 0) port.height = 5;
      computeLabelSizes(port);
    }
  }
  if (node.children) {
    for (const child of node.children) {
      if (!child.width || child.width <= 0) child.width = 30;
      if (!child.height || child.height <= 0) child.height = 30;
      computeLabelSizes(child);
      applyDefaults(child);
    }
  }
  if (node.edges) {
    for (const edge of node.edges) {
      computeLabelSizes(edge);
    }
  }
}

function computeLabelSizes(element: { labels?: ElkLabel[] }): void {
  if (!element.labels) return;
  for (const label of element.labels) {
    if (label.text && label.text.length > 0) {
      if (!label.width || label.width <= 0) label.width = label.text.length * 9;
      if (!label.height || label.height <= 0) label.height = 16;
    }
  }
}

/**
 * Parse an ELKT text string into an ELK JSON graph.
 */
export function parseElkt(input: string): ElkNode {
  const tokens = tokenize(input);
  const parser = new Parser(tokens);
  return parser.parseRoot();
}

export { applyDefaults };

// Exported for testing
export { tokenize, TokenType };
export type { Token };
