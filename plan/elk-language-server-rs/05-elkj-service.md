# 05. ELK JSON 서비스

## 5.1 현재 상태

> **참고**: 현재 `external/elk-live` JSON 에디터(`client/src/json/editor.ts`)는 WebSocket
> `/elkgraphjson`을 사용하지 않고 로컬 elkjs로 레이아웃을 수행한다.
> 따라서 JSON LSP(단계 3)는 ELKT LSP(단계 2)보다 우선순위가 낮다.
> 클라이언트가 JSON LSP를 활용하려면 클라이언트 측 수정이 필요할 수 있다.

## 5.2 source of truth

JSON 편집 서비스의 기준은 다음 두 가지다.

1. grammar:
   - `external/elk/plugins/org.eclipse.elk.graph.json.text/src/org/eclipse/elk/graph/json/text/ElkGraphJson.xtext`
2. Rust import/export runtime:
   - `plugins/org.eclipse.elk.graph.json`

## 5.3 설계 원칙

1. 간이 고정 스키마를 source of truth로 두지 않는다.
2. editor 입력은 lenient하게 받아들인다.
3. ELK 의미를 갖는 키에 대해서만 ELK-aware diagnostics/completion을 제공한다.
4. grammar가 허용하는 arbitrary JSON member는 기본적으로 에러로 취급하지 않는다.

## 5.4 validation 파이프라인

### 1단계: lenient syntax validation

다음 입력을 허용한다.

- trailing comma
- single-quoted string/key
- unquoted identifier key
- comments

이 단계의 목적은 편집 중 diagnostics 위치를 빠르게 제공하는 것이다.

### 2단계: semantic import validation

구문이 유효하면 다음을 시도한다.

```text
ElkGraphJson::for_graph(text)
  .lenient(true)
  .to_elk()
```

이 단계에서 잡는 문제:

- ELK graph로 import할 수 없는 구조
- 잘못된 `sources`/`targets` 타입
- invalid layout option value

## 5.5 completion 문맥

반드시 구분할 문맥:

- top-level node object
- child node object
- edge object
- port object
- label object
- `layoutOptions` / `properties` object
- layout option value

기본 키 제안:

- node: `id`, `children`, `ports`, `labels`, `edges`, `layoutOptions`, `properties`, `x`, `y`, `width`, `height`
- edge: `id`, `sources`, `targets`, `labels`, `layoutOptions`, `properties`
- port: `id`, `labels`, `layoutOptions`, `properties`, `x`, `y`, `width`, `height`
- label: `id`, `text`, `labels`, `layoutOptions`, `x`, `y`, `width`, `height`

## 5.6 layout option 제안

`layoutOptions`와 `properties`는 동일하게 취급한다.

- key completion: `LayoutOptionIndex`
- value completion:
  - enum/enum-set choice
  - boolean
  - default value

가능하면 현재 algorithm context를 읽어 option 후보를 줄인다.

## 5.7 arbitrary member 처리

JSON grammar는 `JsonMember`를 통해 임의 JSON 멤버를 허용한다.

문서화 규칙:

- 이 멤버들은 completion의 주 대상이 아니다.
- 기본 정책은 허용이며, ELK reserved key와 충돌하지 않으면 diagnostic을 만들지 않는다.
- `/conversion`에서 ELK model에 매핑되지 않는 필드는 보존을 보장하지 않는다.

## 5.8 구현 모듈

```text
src/elkj/
├── mod.rs
├── validation.rs
├── context.rs
└── completion.rs
```

역할:

- `validation.rs`: syntax + import diagnostics
- `context.rs`: tolerant token scan으로 cursor context 계산
- `completion.rs`: key/value proposal 생성

## 5.9 테스트 기준

1. comment/trailing comma 입력
2. single quote key/value
3. unquoted key
4. `layoutOptions` 내부 key completion
5. enum value completion
6. malformed array/object diagnostic
7. arbitrary member 허용 케이스
