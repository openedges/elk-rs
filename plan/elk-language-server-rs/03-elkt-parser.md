# 03. ELKT 파서 상세

## 3.1 source of truth

ELKT 구현의 기준은 아래 Java Xtext grammar다.

- `external/elk/plugins/org.eclipse.elk.graph.text/src/org/eclipse/elk/graph/text/ElkGraph.xtext`

이 문서는 구현 편의를 위한 요약일 뿐이며, 문법 충돌 시 항상 Xtext 원본을 따른다.

## 3.2 구현 목표

1. 유효한 Java ELK `.elkt` 입력을 Rust parser가 동일하게 수용한다.
2. completion/formatting에 필요한 trivia와 구문 경계를 보존한다.
3. syntax error가 있어도 가능한 한 CST를 복구하여 diagnostics와 completion을 계속 제공한다.
4. serializer가 ELK graph model을 다시 안정적인 ELKT 텍스트로 출력할 수 있어야 한다.

## 3.3 실제로 지원해야 하는 문법

### 루트

```ebnf
Root
  = 'graph' Identifier?
    ShapeLayout?
    Property*
    (Label | Port | Node | Edge)* ;
```

중요:

- 루트는 빈 문서가 아니라 `ElkNode`에 대응한다.
- `graph` 키워드와 루트 identifier가 선택적이다.
- 루트에도 `layout [position|size]`, `labels`, `ports`, `children`, `edges`가 올 수 있다.

### 노드, 포트, 라벨

```ebnf
Node
  = 'node' Identifier
    ('{' ShapeLayout? Property* (Label | Port | Node | Edge)* '}')? ;

Port
  = 'port' Identifier
    ('{' ShapeLayout? Property* Label* '}')? ;

Label
  = 'label' (Identifier ':')? StringLiteral
    ('{' ShapeLayout? Property* Label* '}')? ;
```

수정 포인트:

- `label`의 본문 텍스트는 `Identifier`가 아니라 `StringLiteral`이다.
- 라벨에 선택적 식별자는 `label id: "text"` 형태다.

### Shape layout

```ebnf
ShapeLayout
  = 'layout' '['
      ( ('position' ':' Number ',' Number)? & ('size' ':' Number ',' Number)? )
    ']' ;
```

수정 포인트:

- 하나의 `layout [...]` 안에서 `position`과 `size`는 둘 다 올 수 있다.
- 순서는 고정이 아니라 Xtext의 unordered-group semantics를 따른다.

### 엣지

```ebnf
Edge
  = 'edge' (Identifier ':')?
    QualifiedId (',' QualifiedId)* '->'
    QualifiedId (',' QualifiedId)*
    ('{' EdgeLayout? Property* Label* '}')? ;
```

수정 포인트:

- source/target은 각각 여러 개를 가질 수 있다.
- 단일 `QualifiedId -> QualifiedId`만 허용하는 축약 문법은 불충분하다.

### 엣지 layout / section

```ebnf
EdgeLayout
  = 'layout' '[' (SingleEdgeSection | NamedEdgeSection+) ']' ;

SingleEdgeSection
  = (('incoming' ':' QualifiedId)? &
     ('outgoing' ':' QualifiedId)? &
     ('start' ':' Number ',' Number)? &
     ('end' ':' Number ',' Number)?)
    ('bends' ':' BendPoint ('|' BendPoint)*)?
    Property* ;

NamedEdgeSection
  = 'section' Identifier
    ('->' Identifier (',' Identifier)*)?
    '['
      (('incoming' ':' QualifiedId)? &
       ('outgoing' ':' QualifiedId)? &
       ('start' ':' Number ',' Number)? &
       ('end' ':' Number ',' Number)?)
      ('bends' ':' BendPoint ('|' BendPoint)*)?
      Property*
    ']' ;
```

수정 포인트:

- named section은 `{ ... }`가 아니라 `[ ... ]`를 사용한다.
- `incoming`, `outgoing`, `start`, `end`, `bends`를 모두 지원해야 한다.
- bend points는 공백 나열이 아니라 `|` 구분이다.

### Property / literal

```ebnf
Property
  = PropertyKey ':' (StringLiteral | QualifiedId | Number | Boolean | 'null') ;

PropertyKey
  = Identifier ('.' Identifier)* ;

QualifiedId
  = Identifier ('.' Identifier)* ;

Number
  = SignedInt | Float ;
```

수정 포인트:

- property 값에 `null`이 올 수 있다.
- 숫자는 `+1`, `-1`, `1.0`, `1e3`, `-1.2E-3` 등을 지원해야 한다.

## 3.4 lexer 요구사항

반드시 처리해야 하는 토큰:

- 키워드: `graph`, `node`, `port`, `label`, `edge`, `layout`, `section`,
  `incoming`, `outgoing`, `position`, `size`, `start`, `end`, `bends`, `true`, `false`, `null`
- 구두점: `{`, `}`, `[`, `]`, `:`, `,`, `.`, `->`, `|`
- 리터럴: `ID`, `STRING`, `SIGNED_INT`, `FLOAT`
- trivia: whitespace, line comment, block comment

주석:

- `// ...`
- `/* ... */`

## 3.5 parser 구조

### 권장 방식

- lexer: `logos`
- CST: `rowan`
- AST wrapper: typed node API
- semantic pass: 별도 validation 단계

### 반드시 보존할 것

- token range
- comment/whitespace trivia
- 오류 복구 후의 tree shape

### 복구 전략

- 블록 경계(`}`, `]`)에서 재동기화
- top-level declaration 시작 키워드에서 재동기화
- property/section 내부에서는 `,`, `]`, `}`를 기준으로 재동기화

## 3.6 AST 계층

필요한 typed node:

- `Root`
- `NodeDecl`
- `PortDecl`
- `LabelDecl`
- `EdgeDecl`
- `ShapeLayout`
- `EdgeLayout`
- `EdgeSection`
- `Property`
- `QualifiedId`

필요한 helper:

- 현재 노드가 parent/node/port/label/edge 중 어느 scope인지 판별
- 문서 내 identifier 수집
- cursor offset -> syntax node/context 매핑

## 3.7 formatter와 serializer의 구분

### formatter

- 입력 문서가 parse 가능할 때만 실행한다.
- 목표는 stable pretty-print다.
- trivia를 보존하지 않고 canonical formatting을 출력한다.

### serializer

- 입력은 `ElkNodeRef` graph model이다.
- `/conversion?outFormat=elkt` 경로에서 사용한다.
- formatter와 동일한 출력 규칙을 공유할 수 있지만, graph model 기준으로 전체 문서를 재구성해야 한다.

## 3.8 semantic validation

ELKT validation은 syntax validation과 분리한다.

1. undefined reference
   - edge source/target
   - section incoming/outgoing
2. duplicate identifier
3. invalid layout option target
4. invalid layout option value
5. serializer 불가능 상태

layout option 값 검증은 가능하면 기존 `LayoutOptionValidator`를 재사용한다.

## 3.9 테스트 기준

필수 회귀 케이스:

1. 루트 `graph id`
2. 중첩 노드/포트/라벨
3. 다중 source/target edge
4. `layout [position..., size...]`
5. single edge section
6. named edge section + outgoing section list
7. `null` property value
8. signed/exponent float
9. comment preservation
10. parse error recovery

테스트 소스:

- `external/elk` example/grammar fixture
- 현재 parity 모델의 `.elkt` 샘플
- 수동 edge case fixture
