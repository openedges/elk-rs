# 07. Transport 계층

## 7.1 목표

`plugins/org.eclipse.elk.lsp`는 두 가지 transport를 제공한다.

1. `websocket`
   - `external/elk-live` 브라우저 클라이언트용
2. `stdio`
   - VS Code, Neovim 등 외부 편집기용

HTTP `/conversion`과 정적 파일 서빙은 `websocket` 모드에서 함께 제공한다.

## 7.2 CLI

예시:

```bash
cargo run -p org-eclipse-elk-lsp --bin elk-ls -- \
  --mode websocket \
  --port 8080 \
  --static-dir external/elk-live/client/app
```

권장 옵션:

- `--mode websocket|stdio`
- `--port <u16>`
- `--host <ip>`
- `--static-dir <path>`
- `--log-level <level>`

## 7.3 WebSocket endpoint

라우팅:

- `GET /elkgraph`
- `GET /elkgraphjson`
- `POST /conversion`
- `GET /*` static files

두 WebSocket endpoint는 동일한 transport adapter를 쓰되,
초기 language selection은 경로 또는 `languageId`로 판별한다.

## 7.4 WebSocket <-> LSP adapter

이 문서에서는 이전처럼 깨진 예시 코드를 source of truth로 두지 않는다.

필수 요구사항만 정의한다.

### 입력

- 브라우저는 JSON-RPC payload를 text frame으로 보낸다.
- 서버는 이를 LSP backend가 소비할 수 있는 stream으로 변환한다.

### 출력

- LSP backend가 내보낸 JSON-RPC payload를 text frame으로 보낸다.

### 구현 선택지

1. `tower-lsp`용 custom `AsyncRead/AsyncWrite` adapter
2. JSON-RPC layer를 transport에서 직접 처리한 뒤 service 호출

선택 기준:

- 통합 테스트가 단순한 쪽
- reconnect/close 처리 안정성이 높은 쪽

중요:

- 문서상 권장 구현은 "별도 adapter 타입"이며 ad-hoc duplex 예시는 금지한다.

### 난이도 참고

WebSocket ↔ `tower-lsp` bridge는 이 프로젝트에서 가장 까다로운 기술 과제일 수 있다.
`tower-lsp`는 기본적으로 stdio/TCP stream을 전제하며, WebSocket text frame과의
bridge는 `AsyncRead`/`AsyncWrite` adapter를 직접 구현해야 한다.

권장 접근:

1. 단계 0에서 stdio 경로를 먼저 구현하여 `tower-lsp` 통합을 검증한다
2. 별도 PoC에서 WebSocket ↔ `tower-lsp` adapter를 실증한 뒤 본 구현에 반영한다
3. `tower-lsp`가 WebSocket bridge에 과도하면, JSON-RPC layer를 transport에서 직접 처리하는
   대안(선택지 2)을 고려한다

## 7.5 stdio

stdio 모드는 VS Code 확장 등 외부 편집기의 기본 transport다.

```text
stdin  -> LSP request stream
stdout -> LSP response/notification stream
```

이 모드에서는 정적 파일 서빙과 `/conversion` HTTP를 열지 않는다.

VS Code 확장에서의 사용:

```json
{
  "command": "elk-ls",
  "args": ["--mode", "stdio"],
  "transport": "stdio"
}
```

`editors/vscode/src/extension.ts`에서 `vscode-languageclient`의 `LanguageClient`로 연결한다.

## 7.6 HTTP/static

WebSocket 모드에서만 활성화한다.

- `POST /conversion`
- optional static file serving

정적 파일 서빙 대상은 `external/elk-live` 실제 빌드 산출물 경로로 설정한다.

## 7.7 CORS와 개발 모드

개발 환경에서는 다음을 지원할 수 있다.

- `localhost:3000` 등에서 오는 브라우저 요청 허용
- WebSocket origin 제한 완화

운영 모드에서는 기본적으로 same-origin을 권장한다.

## 7.8 세션 수명주기

```text
connect
  -> SessionState 생성
  -> initialize
  -> didOpen/didChange/didClose/completion/formatting
disconnect
  -> SessionState drop
```

종료 시 반드시 정리할 것:

- 문서 저장소
- pending task
- tracing context

### 재접속 처리

`external/elk-live` 클라이언트는 `reconnecting-websocket` 라이브러리를 사용한다.

- 최대 20회 재접속 시도
- exponential backoff: 1초 시작, 1.3x 증가, 최대 10초
- 접속 timeout: 10초

서버 측 요구사항:

- 재접속 시 이전 `SessionState`를 정리하고 새 세션을 생성해야 한다
- 클라이언트는 재접속 후 `initialize` → `didOpen`을 다시 전송한다
- 동일 클라이언트의 이전 세션 state가 남아있으면 안 된다

## 7.9 custom message 정책

- 지원하지 않는 notification: log 후 무시
- 지원하지 않는 request: JSON-RPC error 반환
- 브라우저 전용 custom action이 반드시 필요해지면 phase 4 이후 별도 명세 추가

## 7.10 테스트 기준

1. `/elkgraph` initialize 왕복
2. `/elkgraphjson` initialize 왕복
3. WebSocket close 후 세션 정리
4. malformed frame 처리
5. `/conversion` HTTP 통합
6. stdio initialize/completion 테스트
