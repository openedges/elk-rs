# 09. 빌드 시스템 및 테스트 전략

## 9.1 workspace 기준

이 계획은 독립 `elk-language-server/` 저장소를 만들지 않는다.

새 대상은 workspace member `plugins/org.eclipse.elk.lsp`다.

```text
elk-rs/
├── Cargo.toml
├── plugins/
│   └── org.eclipse.elk.lsp
├── external/
│   ├── elk
│   └── elk-live
├── TESTING.md
└── HISTORY.md
```

## 9.2 Cargo 설계

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

권장 의존성:

- `tower-lsp`
- `tokio`
- `axum`
- `tower-http`
- `serde`, `serde_json`, `json5`
- `logos`
- `rowan`
- `quick-xml`
- `dashmap`
- `thiserror`
- `tracing`, `tracing-subscriber`
- `org-eclipse-elk-core`
- `org-eclipse-elk-graph`
- `org-eclipse-elk-graph-json`

## 9.3 build.rs 정책

이 crate에는 metadata 생성용 `build.rs`를 두지 않는다.

이유:

1. layout option source of truth는 이미 `LayoutMetaDataService`다.
2. checked-in JSON/생성 코드 DB는 drift 위험만 늘린다.
3. snapshot 테스트는 런타임 API 결과 비교로 충분하다.

## 9.4 테스트 피라미드

### 단위 테스트

- ELKT lexer
- ELKT parser
- ELKT formatter
- ELKT serializer
- JSON context analyzer
- LayoutOptionIndex lookup

### crate 통합 테스트

- `/conversion` HTTP
- WebSocket initialize/didOpen/completion
- stdio initialize/completion

### 외부 계약 테스트

- `external/elk-live`가 실제 호출하는 `/conversion` payload
- `external/elk-live` ELKT editor와 JSON editor smoke test

### workspace 품질 게이트

- `cargo build --workspace`
- `cargo clippy --workspace --all-targets`
- `cargo test --workspace`
- parity 절차는 `TESTING.md`와 `AGENTS.md` 규칙을 따른다

## 9.5 ELKT 테스트 기준

필수 fixture:

1. minimal graph
2. root `graph id`
3. nested node/port/label
4. multi-source / multi-target edge
5. edge layout section
6. signed/float/exponent number
7. `null` property value
8. error recovery

검증 포인트:

- parse success/failure
- CST shape
- diagnostics range
- formatter stability
- serializer round-trip

## 9.6 JSON 테스트 기준

필수 fixture:

1. trailing comma
2. single-quoted key/value
3. comment 포함 문서
4. `layoutOptions` / `properties`
5. malformed object/array
6. arbitrary custom member

검증 포인트:

- lenient syntax diagnostics
- import diagnostics
- completion context
- option key/value proposal

## 9.7 `/conversion` 테스트 기준

### 필수

- `elkt -> json`
- `json -> elkt`
- invalid input -> canonical JSON envelope
- content-type mapping

### 후속

- `elkg` 경로
- large model conversion

## 9.8 WebSocket / stdio 테스트 기준

### WebSocket

1. initialize
2. didOpen -> diagnostics
3. completion
4. formatting
5. didClose
6. malformed JSON-RPC

### stdio

1. initialize
2. didOpen
3. completion
4. shutdown

## 9.9 `external/elk-live` 연동 테스트

문서상 전제:

- `external/elk-live`가 checkout 되어 있다.
- 정적 자산 또는 개발 서버 경로를 알고 있다.

권장 수동 검증:

```bash
cargo run -p org-eclipse-elk-lsp --bin elk-ls -- \
  --mode websocket \
  --port 8080 \
  --static-dir external/elk-live/client/app
```

브라우저 확인:

1. ELKT editor에서 layout 수행
2. ELKT completion 확인
3. ELKT formatting 확인
4. JSON editor completion/diagnostics 확인

## 9.10 CI 제안

### 필수 job

1. `cargo build --workspace`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

### 선택 job

4. `cargo test -p org-eclipse-elk-lsp`
5. `external/elk-live` 연동 smoke test
6. metadata snapshot drift test
7. VS Code 확장 빌드 (`cd editors/vscode && npm run compile && vsce package`)

### parity

LSP crate 추가 이후에도 기존 workspace parity gate는 유지한다.

- model parity
- phase-step trace

상세 절차는 `TESTING.md`를 따른다.

## 9.11 개발 워크플로우

### 로컬 개발

```bash
# workspace 전체 빌드
cargo build --workspace

# lsp crate만 빠르게 확인
cargo test -p org-eclipse-elk-lsp

# 전체 품질 게이트
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

### 브라우저 연동

```bash
cargo run -p org-eclipse-elk-lsp --bin elk-ls -- \
  --mode websocket \
  --port 8080 \
  --static-dir external/elk-live/client/app
```

실제 static 자산 경로는 `external/elk-live` 구조에 맞게 조정한다.

## 9.12 문서/이력 규칙

구현 단계 완료 후:

1. `HISTORY.md` 갱신
2. 필요 시 `AGENTS.md` 핵심 스냅샷 갱신
3. 품질 게이트 결과 기록

이 규칙은 현재 저장소 운영 원칙과 동일해야 한다.

## 9.13 필수 커맨드 요약

| 작업 | 커맨드 |
|---|---|
| workspace build | `cargo build --workspace` |
| workspace clippy | `cargo clippy --workspace --all-targets -- -D warnings` |
| workspace test | `cargo test --workspace` |
| lsp crate test | `cargo test -p org-eclipse-elk-lsp` |
| websocket 실행 | `cargo run -p org-eclipse-elk-lsp --bin elk-ls -- --mode websocket --port 8080 --static-dir <path>` |
| stdio 실행 | `cargo run -p org-eclipse-elk-lsp --bin elk-ls -- --mode stdio` |
| VS Code 확장 빌드 | `cd editors/vscode && npm install && npm run compile` |
| VS Code 확장 패키징 | `cd editors/vscode && vsce package` |
