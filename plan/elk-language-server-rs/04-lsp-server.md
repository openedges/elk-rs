# 04. LSP 서버 구현

## 4.1 범위

`plugins/org.eclipse.elk.lsp`는 두 개의 LSP endpoint를 제공한다.

- `/elkgraph`: ELKT
- `/elkgraphjson`: ELK JSON

동일한 `SessionState`/`DocumentStore` 구조를 공유하되, 언어별 service는 분리한다.

## 4.2 지원 메서드

| 메서드 | ELKT | ELK JSON | 비고 |
|---|---|---|---|
| `initialize` | O | O | 공통 capabilities |
| `initialized` | O | O | logging only |
| `shutdown` | O | O | 공통 |
| `textDocument/didOpen` | O | O | FULL sync |
| `textDocument/didChange` | O | O | FULL sync로 시작 |
| `textDocument/didClose` | O | O | 문서 상태 제거 |
| `textDocument/completion` | O | O | 언어별 분기 |
| `textDocument/formatting` | O | - | ELKT only |
| `textDocument/publishDiagnostics` | O | O | push |

초기 범위 밖:

- code action
- rename
- references
- semantic tokens
- hover

## 4.3 capabilities

초기 구현은 단순성과 안정성을 위해 `TextDocumentSyncKind::FULL`을 사용한다.

completion trigger는 최소한 다음을 고려한다.

- ELKT: `:`, `.`, ` `, `,`
- ELK JSON: `"`, `'`, `:`, `,`

formatting은 ELKT 문서에서만 활성화한다.

## 4.4 세션 모델

### 세션 단위 상태

- `documents: HashMap<Url, DocumentState>` (세션 단위 단일 스레드 접근)
- `layout_index: Arc<LayoutOptionIndex>`
- `client: tower_lsp::Client`

### 문서 단위 상태

- `text`
- `version`
- `language`
- `elkt_tree`
- `parse_errors`

주의:

- WebSocket 세션마다 독립 `SessionState`를 사용한다.
- `/conversion` HTTP는 LSP 세션 상태를 사용하지 않는다.

## 4.5 ELKT completion

completion source는 네 갈래다.

1. 문법 keyword
   - `graph`, `node`, `port`, `label`, `edge`, `layout`, `section`
2. identifier
   - 현재 문서에서 보이는 node/port/section identifier
3. layout algorithm
   - `algorithm:` 또는 `elk.algorithm:` 값 위치
4. layout option key/value
   - `LayoutOptionIndex`
   - 가능하면 `LayoutDataContentAssist` 재사용

문맥 판별은 CST 기반으로 수행한다.

반드시 구분해야 하는 위치:

- root body
- node body
- port body
- label body
- edge body
- edge section body
- property key
- property value
- edge endpoint

## 4.6 ELKT diagnostics

### syntax diagnostics

- parser가 생성한 `ParseError`를 LSP diagnostic으로 변환한다.
- range는 token 기반으로 계산한다.

### semantic diagnostics

- undefined node/port/section reference
- duplicate identifier
- invalid option target
- invalid option value

semantic diagnostics는 syntax가 완전히 깨진 경우 일부 생략할 수 있다.

## 4.7 ELKT formatting

규칙:

- 4-space indent
- braces/brackets는 newline 기준 canonical layout
- `layout [...]`는 한 줄 유지가 가능하면 한 줄
- section/list는 deterministic ordering 유지

제약:

- parse 성공 시에만 포맷한다.
- 실패 시 `None`을 반환하고 diagnostics만 유지한다.

## 4.8 JSON completion

JSON completion은 "스키마 완전 구현"이 아니라 다음 조합으로 제공한다.

1. grammar key proposal
   - `children`, `ports`, `labels`, `edges`
   - `layoutOptions`, `properties`
   - `id`, `text`, `x`, `y`, `width`, `height`
   - `sources`, `targets`
2. layout option key/value proposal
3. 배열/객체 문맥 보정

문맥 판단은 tolerant scanner로 수행한다.

## 4.9 JSON diagnostics

두 단계로 나눈다.

1. syntax/lexical
   - comments, trailing comma, single quotes, unquoted IDs를 포함한 lenient parse
2. semantic import
   - 구문이 맞으면 `ElkGraphJson` import를 시도해 ELK graph 관점 오류를 잡는다

문서 전체가 항상 ELK model로 import 가능해야 하는 것은 아니다.
부분 입력 중에는 syntax diagnostic만 제공해도 된다.

## 4.10 layout option completion 구현 기준

문서 전체에서 layout option source of truth는 `LayoutMetaDataService`다.

- 별도 `layout_options.json`를 만들지 않는다.
- option key completion은 short/full key 모두 지원한다.
- value completion은 boolean/enum/default value를 제안한다.
- algorithm-aware filtering은 가능한 경우에만 적용한다.

## 4.11 custom notification 처리

`external/elk-live` ELKT 에디터는 Sprotty diagram을 적극적으로 사용한다.

알려진 custom notification:

- `diagram/accept`: Sprotty `ActionMessage`를 주고받는 양방향 notification
- `diagram/didClose`: diagram 세션 종료 notification

정책:

- `diagram/accept`, `diagram/didClose`는 log 후 무시 (no-op). Sprotty 다이어그램은 지원하지 않는다.
- 기타 미지원 notification은 log 후 무시
- 응답이 필요한 request는 명시적으로 error 또는 no-op result

## 4.12 테스트 기준

필수 통합 테스트:

1. initialize -> didOpen -> diagnostics
2. didChange -> diagnostics 갱신
3. ELKT completion
4. JSON completion
5. ELKT formatting
6. 세션 종료 후 상태 제거
7. unsupported custom method 처리
