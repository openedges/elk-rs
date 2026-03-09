# ELK Language Server (Rust) - 설계 문서

## 전제

- `external/elk-live`가 존재하며 브라우저 클라이언트 소스와 정적 자산을 제공한다고 가정한다.
- 새 구현은 별도 저장소가 아니라 이 workspace의 새 멤버 `plugins/org.eclipse.elk.lsp`에 추가한다.
- 문법과 편집기 동작의 source of truth는 Java ELK/Xtext 원본과 `external/elk-live` 클라이언트 계약이다.

## 문서 목록

| 문서 | 설명 |
|---|---|
| [01-overview.md](./01-overview.md) | 목표, 범위, 외부 계약, 호환성 기준 |
| [02-architecture.md](./02-architecture.md) | workspace 구조, 모듈 구성, 재사용 전략 |
| [03-elkt-parser.md](./03-elkt-parser.md) | ELKT 파서/포매터/시리얼라이저 설계 |
| [04-lsp-server.md](./04-lsp-server.md) | LSP 서비스, diagnostics, completion, formatting |
| [05-elkj-service.md](./05-elkj-service.md) | JSON 편집 서비스와 검증/완성 전략 |
| [06-conversion.md](./06-conversion.md) | `/conversion` API와 포맷 변환 설계 |
| [07-transport.md](./07-transport.md) | WebSocket, stdio, HTTP/static transport |
| [08-layout-option-db.md](./08-layout-option-db.md) | Layout metadata 인덱스와 완성 전략 |
| [09-build-and-test.md](./09-build-and-test.md) | workspace 빌드, 테스트, 품질 게이트 |
| [10-migration.md](./10-migration.md) | `external/elk-live`와 단계별 마이그레이션 계획 |

## 핵심 결정

1. `plugins/org.eclipse.elk.lsp`를 workspace member로 추가한다.
2. 별도 `layout_options.json` 데이터베이스를 만들지 않는다.
   런타임 source of truth는 `org.eclipse.elk.core::LayoutMetaDataService`다.
3. 별도 커스텀 `ElkGraph` IR를 만들지 않는다.
   변환의 공통 모델은 기존 ELK graph model(`ElkNodeRef` 등)이다.
4. ELKT 파서는 Java Xtext 문법을 축약 복제하지 않고 실제 문법을 기준으로 구현한다.
5. `/conversion` 에러 형식은 문서 전체에서 하나의 JSON envelope로 통일한다.

## 구현 우선순위

1. `plugins/org.eclipse.elk.lsp` crate 뼈대 + `/conversion`의 `elkt <-> json`
2. ELKT LSP (`/elkgraph`)
3. `stdio` transport + VS Code 확장 (`editors/vscode/`)
4. JSON LSP (`/elkgraphjson`)
5. `elkg` 지원 강화
6. 선택적 WASM/worker 재사용
