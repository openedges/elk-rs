# 02. 전체 아키텍처

## 2.1 workspace 배치

새 구현은 workspace 멤버 `plugins/org.eclipse.elk.lsp`로 추가한다.

```text
elk-rs/
├── Cargo.toml
├── editors/
│   └── vscode/                # 신규 — VS Code 확장
├── external/
│   ├── elk/
│   └── elk-live/              # 존재한다고 가정
└── plugins/
    ├── org.eclipse.elk.core
    ├── org.eclipse.elk.graph
    ├── org.eclipse.elk.graph.json
    ├── org.eclipse.elk.wasm
    ├── org.eclipse.elk.napi
    └── org.eclipse.elk.lsp    # 신규
```

root `Cargo.toml`에는 다음 멤버를 추가한다.

```toml
[workspace]
members = [
  # ...
  "plugins/org.eclipse.elk.lsp",
]
```

## 2.2 새 crate 구조

```text
plugins/org.eclipse.elk.lsp/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── bin/
│   │   └── elk_ls.rs
│   ├── app.rs
│   ├── document_store.rs
│   ├── session.rs
│   ├── diagnostics.rs
│   ├── transport/
│   │   ├── mod.rs
│   │   ├── http.rs
│   │   ├── websocket.rs
│   │   └── stdio.rs
│   ├── elkt/
│   │   ├── mod.rs
│   │   ├── lexer.rs
│   │   ├── parser.rs
│   │   ├── ast.rs
│   │   ├── validation.rs
│   │   ├── formatter.rs
│   │   └── serializer.rs
│   ├── elkj/
│   │   ├── mod.rs
│   │   ├── validation.rs
│   │   ├── context.rs
│   │   └── completion.rs
│   ├── conversion/
│   │   ├── mod.rs
│   │   ├── errors.rs
│   │   ├── elkt_to_graph.rs
│   │   ├── graph_to_elkt.rs
│   │   └── elkg.rs
│   └── layout_options/
│       ├── mod.rs
│       └── index.rs
└── tests/
    ├── elkt_parser.rs
    ├── elkt_formatter.rs
    ├── elkj_completion.rs
    ├── conversion_api.rs
    ├── websocket_lsp.rs
    └── stdio_lsp.rs
```

## 2.3 재사용 전략

### 기존 crate 재사용

| 영역 | 재사용 대상 | 용도 |
|---|---|---|
| graph model | `org.eclipse.elk.graph` | 변환의 공통 모델 |
| JSON import/export | `org.eclipse.elk.graph.json` | `json <-> ElkNodeRef` |
| metadata | `org.eclipse.elk.core::LayoutMetaDataService` | option/algorithm source of truth |
| content assist | `org.eclipse.elk.core::layout_data_content_assist` | option/algorithm 제안 |
| value validation | `org.eclipse.elk.core::validation` | layout option 값 검증 |
| JS-facing metadata | `org.eclipse.elk.graph.json::layout_api` | snapshot/contract 테스트 |

### 새로 구현할 것

| 영역 | 이유 |
|---|---|
| ELKT lexer/parser/CST | Rust 버전이 아직 없음 |
| ELKT formatter/serializer | text round-trip과 formatting 필요 |
| LSP session/document management | 편집기 런타임 전용 |
| WebSocket/stdio transport | Java 서버 대체 필요 |
| `/conversion` HTTP adapter | `external/elk-live` 계약 유지 필요 |

## 2.4 데이터 흐름

### ELKT 편집

```text
textDocument/didOpen
  -> DocumentStore 저장
  -> ELKT parse
  -> syntax diagnostics
  -> semantic validation
  -> publishDiagnostics

textDocument/completion
  -> parse tree + cursor context 분석
  -> keyword / identifier / layout option proposal 생성

textDocument/formatting
  -> parse 성공 시 formatter 실행
```

### JSON 편집

```text
textDocument/didChange
  -> lenient JSON parse
  -> 필요 시 ElkGraphJson import 시도
  -> diagnostics

completion
  -> tolerant token scan
  -> grammar key + layout option 제안
```

### `/conversion`

```text
ELKT text
  -> ELKT parser
  -> ElkNodeRef graph model
  -> ElkGraphJson exporter
  -> JSON text

JSON text
  -> ElkGraphJson importer
  -> ElkNodeRef graph model
  -> ELKT serializer
  -> ELKT text
```

중요:

- 별도 커스텀 `ElkGraph` struct는 만들지 않는다.
- 변환의 공통 IR은 기존 ELK graph model이다.

## 2.5 핵심 런타임 타입

### `AppState`

프로세스 전체에서 공유되는 읽기 전용 상태다.

- `layout_index: Arc<LayoutOptionIndex>`
- `static_dir: PathBuf`
- `server_mode: ServerMode`

### `SessionState`

WebSocket 또는 stdio LSP 세션마다 새로 생성한다.

- `documents: HashMap<Url, DocumentState>` (세션 단위 단일 스레드 접근이므로 `DashMap` 불필요)
- `layout_index: Arc<LayoutOptionIndex>`

### `DocumentState`

- `text: String`
- `version: i32`
- `language: Language`
- `elkt_tree: Option<GreenNode>`
- `parse_errors: Vec<ParseError>`

## 2.6 Cargo 설계

crate 이름은 path 규칙에 맞춰 `org-eclipse-elk-lsp`로 둔다.

```toml
[package]
name = "org-eclipse-elk-lsp"
version.workspace = true
edition.workspace = true
license.workspace = true

[lib]
path = "src/lib.rs"

[[bin]]
name = "elk-ls"
path = "src/bin/elk_ls.rs"
```

주요 의존성:

- `tower-lsp`
- `tokio`
- `axum`
- `serde`, `serde_json`, `json5`
- `logos`
- `rowan`
- `quick-xml`
- `dashmap`
- `thiserror`
- `tracing`
- `org-eclipse-elk-core`
- `org-eclipse-elk-graph`
- `org-eclipse-elk-graph-json`

## 2.7 설계상 금지 사항

1. 별도 `data/layout_options.json`를 source of truth로 두지 않는다.
2. `external/elk-live`와 무관한 가상 `client/`, `server/` 경로를 문서에 쓰지 않는다.
3. ELKT 문법을 축약해서 "대충 비슷한 DSL"로 구현하지 않는다.
4. `/conversion` 에러 형식을 문서마다 다르게 쓰지 않는다.

## 2.8 정적 파일 서빙

WebSocket 모드에서는 선택적으로 정적 파일을 함께 서빙한다.

- 기본값은 비워 두고 CLI 인자로 받는다.
- 예시:
  - `--static-dir external/elk-live/client/app`
  - 또는 `--static-dir external/elk-live/dist`

확인 결과 정적 파일 경로는 `external/elk-live/client/app/`이다.
이 디렉터리에 `index.html`, `common.css`, `diagram.css`, `img/`, webpack 번들(`*.bundle.js`)이 위치한다.

## 2.9 VS Code 확장 구조

```text
editors/vscode/
├── package.json
├── tsconfig.json
├── src/
│   └── extension.ts
├── syntaxes/
│   ├── elkt.tmLanguage.json
│   └── elkj.tmLanguage.json
└── language-configuration.json
```

역할:

- `package.json`: 확장 manifest — language 등록 (`.elkt`, `.elkj`), language server 설정
- `extension.ts`: `elk-ls --mode stdio`를 child process로 실행, `vscode-languageclient` 연결
- `syntaxes/`: TextMate grammar로 syntax highlighting 제공
- `language-configuration.json`: bracket matching, comment toggle, auto-close 설정

주요 의존성:

- `vscode-languageclient`
- `vscode-languageserver-protocol`

빌드/패키징:

- `npm run compile` (TypeScript → JS)
- `vsce package` (VSIX 생성)

## 2.10 동시성 모델

- `LayoutOptionIndex`는 startup 시 1회 구축 후 공유한다.
- 문서 저장소는 세션 단위 `DashMap`이다.
- HTTP `/conversion`은 stateless다.
- WebSocket 세션은 독립적인 `SessionState`를 갖는다.
