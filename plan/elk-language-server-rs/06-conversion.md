# 06. 포맷 변환 서비스

## 6.1 목적

`/conversion`은 `external/elk-live`가 사용하는 핵심 HTTP 계약이다.

최우선 경로는 다음이다.

```text
ELKT text
  -> POST /conversion?inFormat=elkt&outFormat=json
  -> browser side elk-rs layout
```

## 6.2 canonical HTTP 계약

### 요청

```text
POST /conversion?inFormat={elkt|json|elkg}&outFormat={elkt|json|elkg}
Content-Type: text/plain | application/json | application/xml
Body: source text
```

### 성공 응답

```text
HTTP 200
Content-Type: output format에 대응하는 MIME type
Body: converted text
```

### 실패 응답

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

문서 전체에서 `/conversion` 실패 형식은 이 JSON envelope 하나만 사용한다.

## 6.3 지원 범위

### 단계 1 필수

- `elkt -> json`
- `json -> elkt`

### 후속 단계

- `elkt -> elkg`
- `elkg -> elkt`
- `json -> elkg`
- `elkg -> json`

`elkg`는 phase 1의 blocker가 아니다.

## 6.4 변환 공통 모델

별도 `ElkGraph` struct를 만들지 않는다.

공통 모델:

- `org.eclipse.elk.graph`의 ELK graph model
- 실질적으로 루트는 `ElkNodeRef`

이 결정의 이유:

1. 기존 Rust import/export와 바로 연결된다.
2. layout option validation 재사용이 쉽다.
3. 커스텀 IR와 원본 model의 drift를 줄인다.

## 6.5 `elkt -> json`

파이프라인:

```text
ELKT text
  -> ELKT parser
  -> ELKT AST/CST
  -> ElkNodeRef graph model
  -> GraphIdentifierGenerator 필요 시 적용
  -> ElkGraphJson::for_elk(root).to_json()
  -> JSON text
```

세부 원칙:

- layout option key는 기존 JSON exporter 규칙을 따른다.
- pretty-print 여부는 client contract에 맞춘다.
- import 시 invalid property가 있으면 diagnostic으로 반환한다.

## 6.6 `json -> elkt`

파이프라인:

```text
JSON text
  -> ElkGraphJson::for_graph(text).lenient(true).to_elk()
  -> ElkNodeRef graph model
  -> GraphIdentifierGenerator 필요 시 적용
  -> ELKT serializer
  -> ELKT text
```

주의:

- JSON grammar가 허용하는 arbitrary custom member는 ELK model에 직접 대응되지 않을 수 있다.
- 따라서 `json -> elkt -> json`이 모든 확장 필드를 완전 보존한다고 문서화하지 않는다.
- 보존 대상은 ELK graph model과 layout property로 해석 가능한 정보다.

## 6.7 `elkg`

`elkg`는 별도 모듈 `conversion/elkg.rs`에서 다룬다.

초기 문서 원칙:

- phase 1에서는 구현 placeholder를 둘 수 있다.
- phase 4부터 기본 round-trip을 지원한다.
- 필요 시 `quick-xml`을 사용하되, Java ELK `elkg`와의 계약 테스트를 먼저 만든다.

## 6.8 진단 변환

입력 포맷별 오류를 canonical envelope로 변환한다.

### ELKT

- parser syntax error
- semantic validation error

### JSON

- lenient parse error
- ELK import error

### ELKG

- XML parse error
- graph reconstruction error

## 6.9 content type

| 포맷 | MIME type |
|---|---|
| `elkt` | `text/plain; charset=utf-8` |
| `json` | `application/json; charset=utf-8` |
| `elkg` | `application/xml; charset=utf-8` |
| error | `application/json; charset=utf-8` |

## 6.10 테스트 기준

필수:

1. `elkt -> json` minimal graph
2. `elkt -> json` nested graph / port / label / section
3. `json -> elkt` minimal graph
4. invalid ELKT input -> JSON error envelope
5. invalid JSON input -> JSON error envelope
6. HTTP handler integration test

후속:

7. `elkg` round-trip
8. `external/elk-live` 브라우저 smoke test
