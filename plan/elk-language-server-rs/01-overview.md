# 01. 프로젝트 개요

## 1.1 대상 시스템

이 문서는 `external/elk-live`가 존재하는 환경에서, 현재 Java/Xtext 기반 language server 역할을
Rust workspace 내부의 새 crate `plugins/org.eclipse.elk.lsp`로 대체하는 계획이다.

핵심 구조는 다음과 같다.

```text
Browser / external/elk-live client
  ├─ WebSocket /elkgraph      -> ELKT LSP
  ├─ WebSocket /elkgraphjson  -> ELK JSON LSP
  ├─ POST /conversion         -> 포맷 변환
  └─ GET /*                   -> 정적 파일

Rust workspace
  ├─ plugins/org.eclipse.elk.lsp         <- 신규
  ├─ plugins/org.eclipse.elk.core        <- metadata, validation 재사용
  ├─ plugins/org.eclipse.elk.graph       <- ELK graph model 재사용
  ├─ plugins/org.eclipse.elk.graph.json  <- JSON import/export 재사용
  ├─ plugins/org.eclipse.elk.wasm        <- 브라우저 layout API 유지
  └─ plugins/org.eclipse.elk.napi        <- Node/NAPI 유지
```

## 1.2 현재 계약

`external/elk-live`가 의존하는 서버 계약은 다음 세 가지다.

1. `POST /conversion`
   - `elkt -> json`은 브라우저 layout 전의 필수 단계다.
   - `json -> elkt`, `elkg` 변환은 도구성 기능이다.
2. `WebSocket /elkgraph`
   - ELKT 편집기의 completion, diagnostics, formatting을 제공한다.
3. `WebSocket /elkgraphjson`
   - JSON 편집기의 completion, diagnostics를 제공한다.

추가 가정:

- 정적 파일은 `external/elk-live`에서 빌드되며, Rust 서버는 이를 서빙할 수 있어야 한다.
- Sprotty diagram 전송(`diagram/accept`, `diagram/didClose`)은 `external/elk-live`에서 사용되지만,
  이 계획에서는 Sprotty 지원을 구현하지 않는다 (no-op 처리).
- 대신 VS Code 확장을 통한 편집기 지원을 우선한다.
  - `elk-ls --mode stdio`를 language server로 사용하는 VS Code 확장
  - `.elkt` / `.elkj` 파일의 syntax highlighting, completion, diagnostics, formatting
  - 확장 위치: `editors/vscode/`
- 클라이언트 경로의 source of truth는 다음 파일들이다.
  - `external/elk-live/client/src/elkgraph/editor.ts` — ELKT 에디터, WebSocket `/elkgraph` 연결
  - `external/elk-live/client/src/json/editor.ts` — JSON 에디터 (현재 로컬 elkjs 사용, WebSocket 미사용)
  - `external/elk-live/client/src/conversion/editor.ts` — `POST /conversion` HTTP 호출
  - `external/elk-live/client/src/common/creators.ts` — WebSocket/에디터 팩토리
  - `external/elk-live/client/src/common/language-diagram-server.ts` — Sprotty diagram 서버 기반 클래스

## 1.3 목표

1. Java/Gradle 기반 language server 의존성을 제거한다.
2. 구현 위치를 `plugins/org.eclipse.elk.lsp`로 두어 현재 workspace 규칙을 유지한다.
3. ELKT/ELK JSON 편집 경험을 기존 클라이언트와 호환되게 유지한다.
4. 기존 Rust 포팅 자산을 최대한 재사용한다.
5. workspace 품질 게이트를 깨지 않고 도입한다.
6. VS Code 확장을 제공하여 `.elkt`/`.elkj` 파일의 편집 경험을 지원한다.

## 1.4 비목표

- 새 ELK graph runtime을 다시 설계하지 않는다.
- layout metadata를 별도 JSON 파일로 중복 관리하지 않는다.
- `external/elk-live` 전체 프런트엔드를 이 저장소로 복사하지 않는다.
- 서버 사이드 layout 자체를 새로 구현하지 않는다.

## 1.5 반드시 지킬 호환성 기준

### ELKT 문법

- source of truth는
  `external/elk/plugins/org.eclipse.elk.graph.text/src/org/eclipse/elk/graph/text/ElkGraph.xtext`
  이다.
- 계획 문서의 축약 문법이 아니라 실제 Xtext 문법을 기준으로 parser/formatter/serializer를 구현한다.

### ELK JSON 문법

- source of truth는
  `external/elk/plugins/org.eclipse.elk.graph.json.text/src/org/eclipse/elk/graph/json/text/ElkGraphJson.xtext`
  이다.
- JSON 편집 서비스는 "간이 스키마"가 아니라 실제 JSON grammar와 ELK import semantics를 기준으로 한다.

### `/conversion` 에러 응답

- 서버 문서 전체에서 하나의 형식만 사용한다.
- canonical contract는 다음 JSON envelope다.

```json
{
  "message": "Failed to load input graph.",
  "type": "input",
  "diagnostics": [
    {
      "message": "mismatched input 'xxx'",
      "startLineNumber": 3,
      "endLineNumber": 3,
      "startColumn": 1,
      "endColumn": 5
    }
  ],
  "causes": ["ParseException: ..."]
}
```

- 실제 클라이언트가 다른 형식을 더 허용하더라도, 서버 구현과 계획 문서의 기준은 이 JSON envelope다.

## 1.6 핵심 설계 원칙

1. workspace first
   - 독립 `elk-language-server/` 저장소를 새로 만들지 않는다.
2. reuse first
   - `LayoutMetaDataService`, `LayoutDataContentAssist`, `ElkGraphJson`을 우선 재사용한다.
3. grammar parity first
   - 축약 문법/간이 스키마보다 원본 Xtext grammar와 동작 parity를 우선한다.
4. contract first
   - `external/elk-live`가 소비하는 URL, payload, diagnostics 계약을 문서와 구현에서 단일화한다.

## 1.7 단계별 산출물

1. `plugins/org.eclipse.elk.lsp` crate와 workspace 편입
2. ELKT parser + serializer + formatter
3. `/conversion`의 `elkt <-> json` 경로
4. `/elkgraph` ELKT LSP
5. `stdio` transport + VS Code 확장 (`editors/vscode/`)
6. `/elkgraphjson` JSON LSP
7. `elkg` 보강
