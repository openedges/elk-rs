# 10. 마이그레이션 계획

## 10.1 목표 상태

`external/elk-live`는 계속 사용하되, language server 및 conversion 백엔드는
`plugins/org.eclipse.elk.lsp`가 제공한다.

```text
external/elk-live client
  ├─ /elkgraph      -> org.eclipse.elk.lsp
  ├─ /elkgraphjson  -> org.eclipse.elk.lsp
  └─ /conversion    -> org.eclipse.elk.lsp
```

## 10.2 단계별 이전

### 단계 0: crate 추가와 no-op 실행 경로

- workspace member `plugins/org.eclipse.elk.lsp` 추가
- `elk-ls --mode websocket|stdio` CLI 뼈대 추가
- `/healthz` 또는 startup 로그 정도의 최소 실행 확인

완료 기준:

- `cargo build --workspace`
- `cargo test --workspace`
- 새 crate가 workspace에 정상 편입

### 단계 1: `elkt <-> json` conversion

구현 범위:

- ELKT lexer/parser/CST
- ELKT -> `ElkNodeRef`
- `ElkNodeRef` -> JSON export
- JSON import -> ELKT serializer
- `/conversion` HTTP handler

`external/elk-live` 확인 항목:

- `external/elk-live/client/src/conversion/editor.ts` — `/conversion` HTTP 호출 로직
- `external/elk-live/client/src/elkgraph/editor.ts` — ELKT 에디터 WebSocket 연결
- `external/elk-live`에서 호출하는 query/body/content-type
- 오류 표시 로직이 canonical JSON envelope를 소비하는지 확인

성공 기준:

- `POST /conversion?inFormat=elkt&outFormat=json`
- `POST /conversion?inFormat=json&outFormat=elkt`
- `external/elk-live` 브라우저에서 ELKT layout 동작

### 단계 2: ELKT LSP

구현 범위:

- `/elkgraph` WebSocket
- syntax/semantic diagnostics
- keyword/layout option/identifier completion
- formatting

제한사항:

- Sprotty `diagram/accept`, `diagram/didClose` notification은 log 후 무시 (no-op)
- **다이어그램 미리보기는 이 단계에서 동작하지 않음**

성공 기준:

- `external/elk-live` ELKT 에디터에서 completion 동작
- invalid edge reference diagnostics
- formatting 동작

### 단계 3: ELK JSON LSP

> **참고**: 현재 `external/elk-live` JSON 에디터는 WebSocket `/elkgraphjson`을 사용하지 않고
> 로컬 elkjs로 레이아웃을 수행한다. 이 단계는 클라이언트 측 수정이 동반되어야 실효성이 있다.
> 단계 2(ELKT LSP)와 단계 4(elkg/contract) 대비 우선순위가 낮다.

구현 범위:

- `/elkgraphjson` WebSocket
- lenient JSON diagnostics
- key/value/layout option completion

성공 기준:

- JSON 에디터에서 completion
- JSON syntax diagnostics

### 단계 4: stdio transport + VS Code 확장

구현 범위:

- `--mode stdio` transport
- `editors/vscode/` VS Code 확장
  - `package.json`: language 등록 (`.elkt`, `.elkj`), language server 설정
  - `extension.ts`: `elk-ls --mode stdio` child process 연결
  - TextMate grammar: `.elkt` syntax highlighting
  - language-configuration: bracket matching, comment toggle

성공 기준:

- stdio 통합 테스트 (initialize/didOpen/completion/shutdown)
- VS Code에서 `.elkt` 파일 열기 → syntax highlighting 동작
- VS Code에서 completion, diagnostics, formatting 동작
- `vsce package`로 VSIX 빌드 성공

### 단계 5: `elkg`와 계약 보강

구현 범위:

- `elkg` import/export
- `/conversion` 계약 테스트 확대
- unsupported custom notification 명시 처리

성공 기준:

- `elkg` round-trip 기본 케이스
- HTTP/WebSocket contract tests 통과

### 단계 6: 선택적 WASM/worker 재사용

이 단계는 필수가 아니다.

- 목표는 language service의 일부를 worker에서 재사용하는 것이다.
- `external/elk-live` 현재 운영에 필요한 범위를 넘어서므로 후순위다.

## 10.3 `external/elk-live` 연동 체크리스트

### 브라우저 경로

- `/elkgraph`
- `/elkgraphjson`
- `/conversion`
- 정적 파일 루트

### 클라이언트 계약

- WebSocket이 raw JSON-RPC text frame을 주고받는지
- reconnection 정책이 있는지
- `/conversion` 오류 응답 파서가 JSON envelope를 그대로 소비하는지

### 수동 시나리오

1. ELKT 편집
   - `algorithm: layered`
   - 두 노드와 한 엣지 입력
   - layout 수행
2. ELKT diagnostics
   - 존재하지 않는 노드 ID를 edge target으로 사용
3. ELKT formatting
   - 들여쓰기 무너진 문서 포맷
4. JSON 편집
   - `layoutOptions` 내부에서 option key/value completion

## 10.4 위험 요소와 대응

### 위험 1: ELKT 문법 누락

대응:

- 구현 시작 전에 Xtext grammar를 체크리스트로 고정
- parser regression corpus에 실제 ELK 예제 포함

### 위험 2: layout option 완성 drift

대응:

- checked-in JSON DB를 만들지 않고 `LayoutMetaDataService`를 source of truth로 사용
- `known_layout_options()` 결과와 snapshot 비교 테스트 추가

### 위험 3: `/conversion` 계약 mismatch

대응:

- `external/elk-live` 실제 요청/응답을 계약 테스트로 고정
- 문서/구현 모두 canonical JSON envelope만 사용

### 위험 4: WebSocket bridge 불안정

대응:

- adapter를 별도 모듈로 분리
- 재연결/세션 종료/partial message에 대한 통합 테스트 작성

## 10.5 성공 기준

### 기능

- `external/elk-live`가 Java 서버 없이 동작
- ELKT/JSON 편집 기능 유지
- `/conversion` 호환 유지

### 저장소 규칙

- `cargo build --workspace`
- `cargo clippy --workspace --all-targets`
- `cargo test --workspace`
- parity 절차는 기존 `TESTING.md`와 `AGENTS.md` 규칙을 따른다

### 문서

- `HISTORY.md`에 단계별 결과 기록
- 필요 시 `AGENTS.md` 핵심 스냅샷 갱신

## 10.6 구현 순서 제안

1. crate 뼈대
2. ELKT parser + serializer
3. `/conversion`
4. ELKT LSP
5. stdio transport + VS Code 확장
6. JSON LSP
7. `elkg`
