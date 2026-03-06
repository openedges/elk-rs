# elk-rs 개발 노트 — ELK Layout Kernel의 Pure Rust 포팅 프로젝트

> 이 문서는 elk-rs 프로젝트의 전체 개발 과정을 정리한 개발 노트입니다.
> Google Notebook LM 등의 입력 자료로 활용하기 위해 작성되었습니다.

---

## 1. 프로젝트 배경과 동기

### 1.1 ELK란 무엇인가

**ELK (Eclipse Layout Kernel)**는 Eclipse 재단에서 개발한 오픈소스 그래프 자동 레이아웃 엔진이다. 노드와 엣지로 구성된 그래프를 입력으로 받아, 각 요소의 위치(x, y 좌표)와 경로(벤드포인트)를 자동으로 계산해주는 라이브러리다.

ELK는 다양한 레이아웃 알고리즘을 제공한다:
- **Layered (Sugiyama)**: 계층형 방향 그래프. 소프트웨어 아키텍처 다이어그램, 워크플로우 등에 사용. 가장 복잡하고 핵심적인 알고리즘.
- **Force (Fruchterman-Reingold)**: 물리 시뮬레이션 기반. 네트워크 시각화에 적합.
- **Stress (Stress Majorization)**: 그래프 거리 기반 배치. 일반적인 그래프 시각화.
- **MrTree**: 트리 레이아웃. 조직도, 파일 트리 등.
- **Radial (Eades)**: 방사형 트리 레이아웃.
- **Rectpacking**: 사각형 패킹 알고리즘.
- **Disco**: 연결 컴포넌트 분리 및 polyomino compaction.
- **Spore**: 겹침 제거 알고리즘.

### 1.2 기존 생태계의 한계

ELK는 Java로 작성되어 있으며, 웹 환경에서는 **elkjs**라는 JavaScript 버전을 제공한다. elkjs는 Java 코드를 GWT(Google Web Toolkit)로 트랜스파일한 것으로, 몇 가지 근본적 한계가 있다:

1. **성능**: GWT 트랜스파일 결과물은 원본 Java보다 느리고, 최적화 여지가 제한적이다.
2. **번들 크기**: GWT 결과물은 약 1MB 이상의 JavaScript 코드를 생성한다.
3. **플랫폼 제약**: Node.js 서버사이드에서 사용할 때도 JavaScript 런타임의 오버헤드를 감수해야 한다.
4. **GWT 아티팩트**: 트랜스파일 과정에서 부동소수점 정밀도 차이, 특정 API 동작 변경 등이 발생한다. 실제로 elk-rs 검증 과정에서 elkjs만의 drift 20건이 확인되었다.
5. **유지보수**: GWT 생태계가 축소되면서 장기적 지속성이 불확실하다.

### 1.3 왜 Rust인가

Rust는 이 프로젝트에 이상적인 선택이었다:

- **네이티브 성능**: Zero-cost abstractions, 메모리 레이아웃 제어로 Java/JS보다 높은 성능 기대
- **WASM 타겟**: `wasm32-unknown-unknown` 컴파일로 브라우저에서 네이티브에 가까운 성능 달성 가능
- **NAPI 바인딩**: Node.js에서 네이티브 애드온으로 직접 호출 가능
- **안전성**: 소유권 시스템으로 메모리 안전성 보장, 레이아웃 엔진의 복잡한 그래프 조작에서 중요
- **크로스 플랫폼**: 단일 코드베이스에서 네이티브, WASM, NAPI 3가지 타겟 동시 지원

### 1.4 프로젝트 목표

elk-rs의 핵심 목표는 명확했다:

1. **기능 동등성(Parity)**: Java ELK v0.11.0과 동일한 레이아웃 출력을 생산한다. 동일 입력에 대해 동일한 노드 좌표, 엣지 경로, 라벨 위치를 보장한다.
2. **API 호환성**: elkjs의 drop-in replacement로 사용 가능한 npm 패키지를 제공한다.
3. **성능 우위**: Java보다 빠른 레이아웃 엔진을 구현한다.
4. **다중 플랫폼**: 네이티브 Rust, WASM(브라우저), NAPI(Node.js) 3가지 배포 경로를 지원한다.

---

## 2. 프로젝트 규모와 전제 조건

### 2.1 코드 규모

elk-rs는 대규모 포팅 프로젝트다:

- **전체 Rust 코드**: 약 204,000줄 (1,104개 .rs 파일)
- **19개 Cargo 워크스페이스 크레이트**
- **가장 큰 크레이트**: `org.eclipse.elk.alg.layered` — 113,000줄 (전체의 55%)
- 원본 Java ELK는 Eclipse 생태계의 대형 프로젝트로, 수십만 줄의 Java 코드로 구성

### 2.2 워크스페이스 구조

Java 패키지 구조를 그대로 유지했다:

| 크레이트 | 줄 수 | 역할 |
|---------|------|------|
| `org.eclipse.elk.alg.layered` | 113,166 | Layered(Sugiyama) 알고리즘 |
| `org.eclipse.elk.core` | 27,078 | 코어 엔진, 옵션 시스템, 서비스 |
| `org.eclipse.elk.alg.common` | 20,760 | 공통 유틸리티 (NetworkSimplex, 노드사이징) |
| `org.eclipse.elk.graph.json` | 9,086 | JSON import/export |
| `org.eclipse.elk.alg.mrtree` | 6,676 | 트리 레이아웃 |
| `org.eclipse.elk.alg.rectpacking` | 5,449 | 사각형 패킹 |
| `org.eclipse.elk.alg.force` | 3,679 | Force/Stress 알고리즘 |
| `org.eclipse.elk.alg.radial` | 3,448 | 방사형 레이아웃 |
| `org.eclipse.elk.graph` | 3,244 | 그래프 모델 |
| 기타 7개 | ~10,524 | disco, spore, vertiflex 등 |

### 2.3 검증 인프라

포팅 프로젝트의 핵심 전제조건은 "어떻게 정확성을 보장하는가"였다:

- **서브모듈 3개**: `external/elk` (Java 원본), `external/elkjs` (JS 참조), `external/elk-models` (테스트 모델)
- **모델 코퍼스**: 1,998개 테스트 모델 (.elkt, .elkg, .json 포맷)
- **2중 검증**: 최종 출력(Model Parity) + 중간 단계(Phase-Step Trace) 동시 비교
- **6-way 벤치마크**: rust_native, rust_api, java, elkjs, napi, wasm 6개 엔진 동시 성능 비교
- **자동화 스크립트**: 40개 이상의 검증/비교/회귀검사 스크립트

### 2.4 AI 코딩 에이전트와의 협업

이 프로젝트의 독특한 점 중 하나는 **AI 코딩 에이전트(Claude)와의 협업으로 진행**되었다는 것이다. 프로젝트 문서(`AGENTS.md`, `HISTORY.md`, `TESTING.md`, `VERSIONING.md`)는 사람과 AI 에이전트 모두가 참조할 수 있도록 구조화되었으며, 매 작업 단계마다 빌드/테스트/parity 검증을 자동으로 수행하는 품질 게이트가 설정되었다.

---

## 3. 개발 타임라인과 주요 이벤트

### 3.1 개발 기간

- **첫 커밋**: 2026년 2월 2일
- **최종 상태(현재)**: 2026년 3월 6일
- **총 기간**: 약 33일 (1개월)
- **총 커밋**: 약 333개

### 3.2 Phase 1: 기반 구축 (2월 초)

**핵심 그래프 모델과 코어 엔진 포팅**

첫 커밋 `786516f`에서 core/graph 스캐폴딩으로 시작했다. 이 단계에서:

- `ElkGraph` 모델 (노드, 포트, 엣지, 라벨의 기본 데이터 구조)
- `ElkCore` 서비스 (알고리즘 등록, 옵션 시스템, 프로퍼티 관리)
- `GraphAdapters` (그래프 순회 추상화)
- `JSON Importer/Exporter` (elkjs 호환 JSON 포맷)
- 옵션 파싱 시스템 (enum, KVector, IndividualSpacings 등)

### 3.3 Phase 2: Layered 알고리즘 포팅 (2월 초~중순)

**가장 크고 복잡한 핵심 알고리즘의 단계별 포팅**

Layered(Sugiyama) 알고리즘은 5개 주요 단계(Phase)로 구성된다:

1. **P1 Cycle Breaking**: 방향 그래프의 순환 제거 (Greedy, DepthFirst, Interactive)
2. **P2 Layer Assignment**: 노드를 계층에 배치 (NetworkSimplex, LongestPath, CoffmanGraham 등 8개 전략)
3. **P3 Crossing Minimization**: 엣지 교차 최소화 (LayerSweep, Barycenter, GreedySwitch)
4. **P4 Node Placement**: 노드 y좌표 결정 (BrandesKoepf, NetworkSimplex, Linear Segments)
5. **P5 Edge Routing**: 엣지 경로 결정 (Orthogonal, Polyline, Spline)

각 단계 사이에 20개 이상의 Intermediate Processor가 삽입된다 (더미 노드 삽입, 라벨 처리, 자기 루프, North/South 포트 등).

이 단계의 핵심 과제:

- **LGraph 내부 모델**: Java의 `LGraph`/`LNode`/`LPort`/`LEdge` 구조를 Rust의 `Arc<Mutex<T>>`로 표현. Java의 자유로운 참조 패턴을 Rust 소유권 모델로 변환하는 것이 가장 큰 도전.
- **NetworkSimplex 솔버**: 레이어 배정과 노드 배치에서 사용되는 그래프 이론 알고리즘. Rust 포팅 시 데드락 문제 해결 필요.
- **BrandesKoepf 정렬기**: 복잡한 align chain + compaction 로직에서 무한 루프 발생 → 루프 가드 추가.
- **Spline 엣지 라우팅**: NubSpline/자기 루프 스플라인 등 수치 계산 집약적 코드 포팅.

### 3.4 Phase 3: 나머지 알고리즘 포팅 (2월 중순)

Layered 이외의 알고리즘을 순차적으로 포팅:

- **MrTree** (트리 레이아웃): 크레이트 추가, 프로세서 파이프라인, 엣지 라우팅
- **Force/Stress**: Fruchterman-Reingold 모델, Stress Majorization, 그래프 모델
- **Rectpacking**: 폭 근사, 패킹, 화이트스페이스 제거
- **Radial**: 중심 노드 기반 방사형 배치
- **Disco**: 연결 컴포넌트 분리, Polyomino compaction
- **Spore**: 스팬닝 트리 기반 겹침 제거
- **Vertiflex**: 수직 유연 레이아웃
- **TopdownPacking**: 상하향 패킹
- **Graphviz/Libavoid**: 스텁 구현 (외부 도구 연동용)

### 3.5 Phase 4: Parity 추격전 (2월 중순~하순)

**가장 힘들었던 구간 — Java와의 출력 일치 달성**

코드 포팅 자체보다 **Java와 정확히 동일한 출력**을 만드는 것이 진짜 도전이었다. 처음에는 parity 비율이 매우 낮았고, 점진적으로 끌어올렸다:

- **52.4% (754/1439)**: 초기 parity 재실행 시점
- **99.9% (1438/1439)**: 집중 parity 작업 후
- **100% (1988/1989)**: 최종 (모델 코퍼스 확대 포함)

#### Parity 작업에서 발견한 주요 이슈들

1. **부동소수점 정밀도**: Java의 `float→double` 암묵적 변환과 Rust `f64` 사이의 미세 차이. `OER`의 xpos 계산, LGraphUtil resize에서 Java의 float 정밀도를 의도적으로 매칭.

2. **Java 버그 재현 vs 회피**: Java ELK의 `PortListSorter`에 `findPortSideRange` 버그가 있었다. 정확한 parity를 위해 동일한 버그를 Rust에도 **의도적으로 재현**해야 하는 딜레마. 반면 `213_componentsCompaction`의 NaN 전파 버그는 **재현하지 않기로** 결정 — Rust 출력이 더 정확하므로.

3. **HashMap 비결정성**: Java의 `HashMap` 순회 순서가 JVM마다 다르다. `SelfHyperLoop.computePortsPerSide()`에서 `ArrayListMultimap`의 비결정적 키 순회가 ~80개 모델에서 매 실행마다 다른 결과를 생산. 해결: Java에 determinism 패치 적용 (`MultimapBuilder.enumKeys()`).

4. **음수 그래프 크기 패턴**: Java는 `graph_size`가 음수인 중간값을 허용한다 (예: `0 - componentSpacing = -20`). Rust에서 `.max(0.0)` clamp를 추가했다가 parity가 깨짐. 원인 분석 후 clamp 제거로 해결.

5. **Inside Self-Loop 노드 처리**: Java에 없는 Rust 전용 hack이 json_importer에 존재. `has_no_children` 가드를 추가하여 Java 동작과 일치시킴.

6. **Crossing Minimization 순서**: `calculate_port_ranks`에서 포트 순서가 스위핑 중 변경 가능(mutable). CSR snapshot의 전체 적용 시 50개 모델에서 drift 발생. 부분 적용(distribute_ports/sort_ports만)으로 해결.

### 3.6 Phase 5: 모델 커버리지 확대 (2월 하순~3월 초)

- 기존 1,438개 → **1,998개** 모델로 확대
- 550개 `.json` ptolemy 모델 추가 (Java export + manifest 통합)
- Phase-step trace 도 전수 검증으로 전환: **1,997/1,997 match (100%)**
- Java determinism 패치 시스템 정식화: isolation worktree에서 자동 적용
- Maven 캐시 stale 문제 발견 및 자동 퍼지 추가

### 3.7 Phase 6: 성능 최적화 (3월 1일~4일)

**가장 극적인 변화 — 19단계의 연속 최적화로 6.35배 속도 향상**

초기 Rust는 Java보다 **3.4배 느렸다**. 이것은 예상 밖이었다. Rust가 Java보다 느린 이유는 포팅 방식에 있었다: Java의 자유로운 객체 참조를 `Arc<Mutex<T>>`로 변환했기 때문에, 매 속성 접근마다 lock/unlock 오버헤드가 발생했다.

**layered_xlarge 벤치마크 기준 최적화 여정:**

| Phase | 내용 | 시간 | Java 대비 |
|-------|------|------|-----------|
| 시작 | 최초 baseline | 1,576ms | 3.40x 느림 |
| 1-3 | LazyLock (env::var 제거) | 972ms | 2.10x |
| 4-6 | ports_by_side + CSR snapshot | 553ms | 1.19x |
| 7 | NetworkSimplex lock batching | 702ms | 1.52x |
| 8 | FxHashMap 전역 전환 | 672ms | 1.45x |
| 9-15 | Arc-shared props, port_side array, 미세 최적화 | 561ms | 1.21x |
| **16** | **P3 CSR snapshot propagation 수정** | **426ms** | **0.92x (역전!)** |
| 17 | calculate_port_ranks snapshot | 301ms | 0.65x |
| 18 | mimalloc allocator | 272ms | 0.59x |
| 19 | Property key Cow<'static, str> interning | 248ms | 0.54x |

**최종: 1,576ms → 248ms (84.3% 감소, 6.35배 가속)**

### 3.8 Phase 7: SoA 최적화와 6-way 벤치마크 (3월 3일~4일)

Layered 이외 알고리즘의 성능 최적화:

- **Force 알고리즘**: SoA(Struct-of-Arrays) 패턴으로 재작성. O(n²×300) force 루프를 lock 0회로 실행. **882ms → 141ms (6.3배)**
- **Stress 알고리즘**: Dijkstra APSP를 flat adjacency list로 lock-free 실행. **963ms → 170ms (5.7배)**
- **Radial**: Cache 추가 시도했으나 개선 없음 (RefCell/Rc 아키텍처 제약)

6-way 공정 벤치마크 프레임워크 구축:
- 동일 LCG(Linear Congruential Generator) 시드로 6개 엔진에 동일한 그래프 생성
- 26개 synthetic 시나리오 (layered, force, stress, mrtree, radial, rectpacking, routing, crossmin, hierarchy)
- Rejection sampling DAG 생성으로 공정한 그래프 구조 보장

### 3.9 Phase 8: npm 패키지 (WASM + NAPI) (3월 초)

- **WASM 패키지**: `wasm-pack`으로 빌드, 브라우저 직접 사용 가능
- **NAPI 패키지**: 6개 플랫폼별 네이티브 애드온
  - `@elk-rs/darwin-arm64`, `@elk-rs/darwin-x64`
  - `@elk-rs/linux-x64-gnu`, `@elk-rs/linux-x64-musl`, `@elk-rs/linux-arm64-gnu`
  - `@elk-rs/win32-x64-msvc`
- **로딩 전략**: 플랫폼 NAPI → 로컬 .node → WASM 폴백
- **35개 Vitest 테스트** 통과
- **JS parity**: 550/550 elk-rs vs Java 일치

---

## 4. 핵심 기술적 결정과 트레이드오프

### 4.1 `Arc<Mutex<T>>` vs Arena 기반 그래프

**결정**: Java의 객체 참조를 `Arc<Mutex<T>>`로 포팅.

**이유**: Java ELK의 그래프 모델은 노드, 포트, 엣지가 상호 참조하는 복잡한 구조다. `LNode`가 `LPort` 목록을 가지고, `LPort`는 연결된 `LEdge`를 참조하며, `LEdge`는 source/target `LPort`를 참조한다. 이 양방향 참조를 Rust로 표현하는 데 `Arc<Mutex<T>>`를 선택했다.

**트레이드오프**:
- 장점: Java 코드와 1:1 대응으로 정확한 포팅 가능, parity 달성에 유리
- 단점: 매 속성 접근마다 lock/unlock 오버헤드, 초기 성능이 Java보다 3.4배 느림

**후속 해결**: CSR(Compressed Sparse Row) snapshot 패턴으로 hot path에서 lock을 완전 제거. Phase-local snapshot을 구축하여 P3/P5 구간에서 lock-free 실행 달성.

**미래 방향**: Full Arena 전환 설계가 완료되어 있으나(HISTORY.md에 5-Phase 계획 문서화), lock/Arc 오버헤드가 이미 ~0%로 감소하여 추가 이득은 cache locality 개선(~10-20ms)에 한정. 비용 대비 효과가 낮아 보류 중.

### 4.2 Java 버그 재현 정책

**결정**: Java 버그를 3가지로 분류하여 차등 대응.

| 분류 | 대응 | 예시 |
|------|------|------|
| **정확한 재현** | 동일 버그를 Rust에 구현 | `PortListSorter.findPortSideRange` |
| **의도적 비재현** | Rust가 더 정확한 출력 생산 | `ComponentsCompactor` NaN 전파 |
| **패치** | Java에 패치 적용 후 비교 | `SelfHyperLoop` HashMap 비결정성 |

**이유**: 100% parity가 목표이지만, Java 자체에 NaN을 출력하거나 NPE를 throw하는 케이스까지 재현하는 것은 무의미하다. 이런 케이스는 `java_exclude.txt`에서 명시적으로 제외.

### 4.3 FxHashMap 전환

**결정**: 표준 `HashMap`(SipHash)을 `FxHashMap`(Fx hash)으로 전역 전환.

**이유**: 레이아웃 엔진의 HashMap 키는 대부분 `usize` 포인터 또는 `u32` ID다. SipHash는 DDoS 방어용으로 설계된 암호학적 해시인데, 레이아웃 엔진은 외부 입력을 키로 사용하지 않으므로 불필요한 오버헤드. Fx hash는 integer에 대해 ~3-5ns (SipHash ~20ns 대비 4배 빠름).

**효과**: 단일 변경으로 여러 ms씩 일관된 개선. 최종 프로파일에서 hash ops 4.3%까지 감소.

### 4.4 Property 시스템의 `Cow<'static, str>` 키

**결정**: 프로퍼티 키를 `String`에서 `Cow<'static, str>`로 변경.

**이유**: 421개 프로퍼티 상수가 모두 `&'static str` 리터럴이므로, 프로퍼티 조회 시마다 `String::clone()`을 수행하는 것은 낭비. `Cow::Borrowed`는 포인터 복사(8 bytes)로 충분.

**효과**: Phase 19에서 ~24ms 절감 (8.9%). 프로퍼티 lookup은 레이아웃 전 과정에서 수만 회 호출되므로 누적 효과가 크다.

### 4.5 프로파일 분석 기반 최적화 중단 결정

**결정**: Phase 19에서 성능 최적화를 실질적으로 종료.

**근거**:
1. 프로파일이 극도로 평탄 — 단일 함수가 전체의 2% 이상인 hotspot이 없음
2. lock/Arc overhead는 이미 ~0% (CSR snapshot 효과)
3. 남은 인프라 overhead: malloc 7.9% + hash 4.3% ≈ 12%. 나머지 88%는 순수 알고리즘 연산
4. 추가 개선은 전면 arena 전환 또는 알고리즘 수준 변경이 필요하며, 기대 효과 대비 비용이 큼

이 결정은 "더 최적화할 수 있는가"가 아니라 **"투입 대비 효과가 합리적인가"**의 관점에서 내린 것이다.

---

## 5. 검증 체계와 품질 보증

### 5.1 2중 Parity 검증

단순히 "최종 출력이 같은가"만 확인하는 것은 충분하지 않다. elk-rs는 2중 검증을 수행한다:

1. **Model Parity (최종 출력)**: Java와 Rust의 레이아웃 JSON을 비교. 노드 좌표, 엣지 벤드포인트, 라벨 위치가 모두 일치하는지 확인.
2. **Phase-Step Trace (중간 상태)**: Layered 알고리즘의 50개 이상 중간 프로세서 각각의 출력을 비교. 최종 결과가 같더라도 중간 경로가 다르면 잠재적 문제 표시.

실제로 Phase-Step Trace에서 발견된 흥미로운 케이스:
- `partitioning.elkt`: NetworkSimplexLayerer 단계에서 동일 layer 내 노드 순서가 다름. 그러나 후속 CrossingMinimizer에서 자동 정렬되어 최종 출력은 동일. 이것은 "equivalent intermediate representation" — 올바른 drift.
- `368_selfLoopLabelsIOOBE.elkt`: OrthogonalEdgeRouter 단계에서 x좌표 10.0 차이. GraphTransformer export에서 보정되어 최종 동일.

### 5.2 Parity 수치의 의미

최종 parity 결과:
- **Model Parity**: 1,988/1,989 match (drift=1, skipped=9)
- **Phase-Step Trace**: 1,997/1,997 match (0 drift)
- **Tickets Parity**: 108/109 match
- **JS Parity**: 550/550 match

유일한 drift인 `213_componentsCompaction.elkt`는 **Java ELK 자체의 버그**(NaN y좌표 73건 + 이상 x좌표 12건)로 인한 것이며, Rust 출력이 수학적으로 더 정확하다.

### 5.3 성능 벤치마크의 공정성

6-way 벤치마크는 공정성을 극도로 중시한다:

- **동일 그래프 생성**: LCG(Linear Congruential Generator) `(state * 1103515245 + 12345) & 0x7fffffff`를 모든 언어에서 동일하게 구현. 동일 시드에서 동일한 노드/엣지 구조 생성.
- **Rejection Sampling DAG**: DAG(Directed Acyclic Graph) 생성 시 `max_attempts = edges * 100`으로 동일한 rejection 횟수 보장.
- **그룹 분리**: Native(rust_native, java)와 API(rust_api, napi, wasm, elkjs) 그룹을 명확히 분리. JSON 직렬화 오버헤드를 포함한 비교와 순수 레이아웃 비교를 구분.
- **타이밍 범위**: 그래프 생성은 측정 외부, 레이아웃만 측정 내부.

---

## 6. 달성한 가치와 수치

### 6.1 기능적 완성도

| 항목 | 수치 | 의미 |
|------|------|------|
| 알고리즘 | 13개 전체 포팅 | Layered, Force, Stress, MrTree, Radial, Rectpacking, Disco, Spore, Vertiflex, TopdownPacking + 3개 스텁 |
| Model Parity | 1,988/1,989 (100%) | 유일한 drift는 Java 버그 |
| Phase-Step Trace | 1,997/1,997 (100%) | 중간 단계까지 완전 일치 |
| Rust 테스트 | 712 테스트 | 모든 테스트 통과 |
| JS Parity | 550/550 (100%) | elkjs 대비 완전 일치 |
| 코드 품질 | 0 warning | cargo build + cargo clippy 모두 0건 |

### 6.2 성능 성과

**6-way 벤치마크 결과 (26개 시나리오 평균, 2026-03-06 MacBook Pro M3 Pro 측정):**

| 엔진 | 평균 시간 | vs java |
|------|----------|---------|
| **rust_native** | **25.6ms** | **3.90x faster** |
| rust_api | 26.8ms | 3.73x faster |
| java | 100.0ms | 1.00x (baseline) |
| napi | 127.9ms | 0.78x |
| wasm | 174.3ms | 0.57x |
| elkjs | 1039.7ms | 0.10x |

**알고리즘별 상세:**

| 시나리오 | Rust | Java | Rust 우위 |
|----------|------|------|-----------|
| layered_xlarge | 245ms | 360ms | **1.47x** |
| force_xlarge | 157ms | 947ms | **6.03x** |
| stress_xlarge | 173ms | 981ms | **5.67x** |
| mrtree_xlarge | 5.82ms | 26.6ms | **4.56x** |
| routing_orthogonal | 2.61ms | 2.95ms | **1.13x** |
| radial_xlarge | 11.8ms | 17.4ms | **1.47x** |

**Rust wins 24/26 시나리오, Java wins 2/26 시나리오** (radial_medium, radial_large)

**elk-rs가 Java보다 우위인 영역**: layered, force, stress, mrtree, rectpacking, routing, crossing-min, hierarchy, radial_xlarge
**Java가 우위인 영역**: radial_medium, radial_large (RefCell/Rc 아키텍처 제약으로 per-node sorter 호출이 지배적)

### 6.3 배포 플랫폼

| 플랫폼 | 형태 | 상태 |
|--------|------|------|
| Rust Native | Cargo 크레이트 | v0.11.0 준비 완료 |
| WASM | npm 패키지 (브라우저) | 35 Vitest 통과 |
| NAPI darwin-arm64 | npm 네이티브 애드온 | 빌드 완료 |
| NAPI darwin-x64 | npm 네이티브 애드온 | 빌드 완료 |
| NAPI linux-x64-gnu | npm 네이티브 애드온 | CI 구성 완료 |
| NAPI linux-x64-musl | npm 네이티브 애드온 | CI 구성 완료 |
| NAPI linux-arm64-gnu | npm 네이티브 애드온 | CI 구성 완료 |
| NAPI win32-x64-msvc | npm 네이티브 애드온 | CI 구성 완료 |

### 6.4 프로젝트의 의의

1. **실용적 대규모 Rust 포팅 사례**: 20만 줄의 Java 코드를 1개월 만에 100% parity로 Rust로 포팅한 사례. AI 에이전트 협업의 가능성을 보여줌.
2. **성능 역전**: 초기에 3.4배 느렸던 Rust가 최적화를 통해 3.90배 빠른 결과를 달성. "Rust로 작성하면 자동으로 빠르다"가 아니라 "Rust의 도구(SoA, CSR, Cow, FxHash, mimalloc)를 올바르게 활용해야 빠르다"는 교훈.
3. **elkjs 대체제**: 웹 생태계에서 GWT 기반 elkjs의 한계를 WASM/NAPI로 극복. 동일 API, 더 빠른 성능.
4. **검증 방법론**: 2중 parity 검증(최종 출력 + 중간 단계), 6-way 공정 벤치마크, 자동화된 회귀 검사 파이프라인은 대규모 포팅 프로젝트의 모범 사례.

---

## 7. 핵심 변곡점과 교훈

### 7.1 변곡점 1: Parity 52% → 100%

단순 코드 포팅은 전체 작업의 50%에 불과했다. 나머지 50%는 "왜 Java와 다른 결과가 나오는가"를 추적하고 수정하는 작업이었다. 수십 가지 미세한 차이(부동소수점, 정렬 순서, 암묵적 변환, 비결정성)를 하나하나 잡아내야 했다.

### 7.2 변곡점 2: Phase 16 — Rust가 Java를 추월한 순간

Phase 1-15까지 Rust는 계속 Java보다 느렸다(1.21x). Phase 16에서 CSR snapshot propagation 버그를 발견하고 수정한 순간, 561ms → 426ms로 떨어지며 Java(463ms)를 처음으로 추월했다. 이것은 단일 버그 수정이 아니라 "올바른 아키텍처 위에 올바른 데이터가 흐르는가"의 문제였다.

### 7.3 변곡점 3: 프로파일 평탄화와 최적화 종료 결정

Phase 19 이후 프로파일이 극도로 평탄해졌다. 최대 hotspot이 전체의 2% 미만. 이것은 "더 최적화할 부분이 없다"가 아니라 "모든 부분이 비슷하게 최적화되었다"는 의미다. 여기서 멈추는 것이 합리적이라는 결정을 내렸다.

### 7.4 교훈: Lock은 비용이 아니었다

처음 가설: "Arc<Mutex<T>>의 lock 오버헤드가 성능 저하의 원인이다."

실제 원인: macOS의 uncontended `pthread_mutex` overhead는 ~20ns로 매우 낮았다. 진짜 비용은:
1. `Vec<Arc<T>>` clone 시 Arc refcount 증가/감소
2. 매 clone마다 메모리 할당
3. 캐시 친화적이지 않은 메모리 접근 패턴

CSR snapshot이 이 3가지를 동시에 해결했다: flat array에서 O(1) indexed lookup, Arc 없음, 연속 메모리 접근.

### 7.5 교훈: Java 버그도 spec이다

parity를 추구하면 Java의 모든 동작이 "스펙"이 된다. Java의 `PortListSorter` 버그를 Rust에 재현해야 할 때, "이것이 올바른 일인가"라는 근본적 질문이 생겼다. 결론: **parity와 correctness를 분리해서 추적한다**. 동일 입력에 동일 출력을 보장하되, Java 버그는 문서화하고 향후 독립적으로 수정할 여지를 남긴다.

---

## 8. 앞으로 할 일

### 8.1 단기

- **npm 릴리즈**: `elk-rs@0.11.0` 정식 publish (WASM + 6개 NAPI 플랫폼)
- **Radial 성능**: radial_medium/radial_large에서 Java보다 느림. PolarCoordinateSorter의 per-node 호출이 병목. 근본 해결은 sorter 아키텍처 변경 필요. (radial_xlarge는 최적화로 Rust 우위 달성)
- **CHANGELOG.md 작성**: VERSIONING.md에 정의된 형식으로 릴리즈 노트 작성

### 8.2 중기

- **ELK 0.12.0 포팅**: 새 ELK 버전 출시 시 diff 분석 → 포팅 → parity 검증 사이클
- **Full Arena 전환**: 설계 완료(5-Phase 계획). cache locality 개선으로 ~10-20ms 추가 절감 가능. 비용 대비 효과 재평가 필요.
- **WASM 최적화**: WASM 타겟 전용 최적화 (simd128, bulk memory operations)
- **API 확장**: 스트리밍 레이아웃, 부분 업데이트 등 elkjs에 없는 기능

### 8.3 장기

- **독립적 알고리즘 개선**: Java 버그를 수정한 Rust 고유 경로. parity 모드와 improved 모드를 옵션으로 분리.
- **병렬 레이아웃**: Rayon 등을 활용한 멀티코어 레이아웃 (Java는 단일 스레드)
- **시각 편집기 통합**: VS Code 확장, Web IDE에서의 실시간 레이아웃 미리보기

---

## 9. 프로젝트 메타데이터

| 항목 | 값 |
|------|-----|
| 프로젝트 이름 | elk-rs |
| 버전 | 0.11.0 (ELK Java v0.11.0 대응) |
| 라이선스 | EPL-2.0 (Eclipse Public License 2.0) |
| 언어 | Rust (2021 edition) |
| 개발 기간 | 2026-02-02 ~ 2026-03-06 (33일) |
| 코드 규모 | ~204,500 줄 Rust, 1,104 파일 |
| 크레이트 수 | 19개 |
| 커밋 수 | ~333개 |
| 테스트 | 712 Rust 단위테스트 + 35 Vitest + 1,998 parity 모델 |
| 벤치마크 | 26 synthetic 시나리오 × 6 엔진 |
| 원본 | Eclipse ELK (Java) https://www.eclipse.org/elk/ |
