# Rust 코드 품질 개선 계획 V2 — Arc<Mutex> 아키텍처 한계 극복

## 배경

V1 계획 실행 결과, 남은 모든 품질 이슈가 **`Arc<Mutex<T>>` 그래프 모델의 근본 한계**에서 비롯됨을 확인:

| 문제 | 현황 | 근본 원인 |
|------|------|-----------|
| `lock_ok()` Option 체인 | 3,810건 | lock()이 MutexGuard 반환 → Option chain 필요 |
| Vec clone (데드락 방지) | 338건 | guard 보유 중 자식 lock 불가 → Vec 복사 필수 |
| `get_property(&mut self)` | 1,538건 | Proxy 해석이 map mutate → &mut self 필수 |
| typed wrapper 불가 | - | get_property가 &mut self라서 보일러플레이트 동일 |
| Arc::ptr_eq 검색 | 367건 | identity 기반 O(n) 탐색 |

**해결 전략**: 하향식(top-down)이 아닌 **상향식(bottom-up)** — 가장 영향 범위가 작은 변경부터 시작하여 점진적으로 아키텍처를 개선한다.

---

## Phase A: get_property `&self` 전환 (기반 정비)

### A-1. `get_property` → `get_property_immut` 통합

**현황**: `get_property_immut(&self)` 메서드가 이미 존재하지만 사용처가 단 2건.

**문제**: `get_property(&mut self)`는 Proxy 해석 결과를 캐시하기 위해 `&mut self`를 요구한다. 그러나:
- Proxy는 JSON import 시에만 생성되고, 첫 번째 `get_property` 호출에서 해석 후 캐시
- 레이아웃 알고리즘 실행 시점에는 모든 Proxy가 이미 해석된 상태
- `get_property_immut`는 Proxy를 해석하되 캐시하지 않음 — 동작은 동일하나 map mutate 없음

**작업**:
1. `get_property_immut`를 `get_property`로 이름 변경 (기존 `get_property`를 `get_property_cached`로)
2. 전체 1,538건의 `get_property` 호출 → 새 `get_property(&self)` 사용 (기계적 치환)
3. `get_property_cached(&mut self)` 호출은 JSON import/export 경로에만 유지

**효과**:
- 모든 property 접근이 `&self`로 통일
- `&mut self` 전파 사슬 제거 → 수백 개 함수에서 `&mut` 불필요
- Phase A-2 (typed wrapper) 언블록

**Parity 위험**: 없음 — 프록시 해석 결과 동일, 캐시 여부만 차이
**파일 수**: ~200파일 (기계적 치환)

### A-2. Property typed wrapper 도입

**선행**: A-1 완료 (`get_property(&self)`)

`get_property(&self)` 기반으로 빈번 사용 property 상위 20~30개에 typed accessor 추가.

```rust
impl LNode {
    pub fn origin(&self) -> Option<Origin> {
        self.shape().graph_element().properties().get_property(&InternalProperties::ORIGIN)
    }
    pub fn port_constraints(&self) -> PortConstraints {
        self.shape().graph_element().properties()
            .get_property(&LayeredOptions::PORT_CONSTRAINTS)
            .unwrap_or(PortConstraints::Undefined)
    }
}
```

**효과**: 43+41+26+24+22 = 상위 5개만으로 156건의 `get_property` 호출을 1-word 메서드로 교체
**Parity 위험**: 없음 (기존 경로의 wrapper)

---

## Phase B: lock_ok() → lock() 대규모 전환 (3,810건)

### B-1. `lock_ok().and_then(|g| g.method())` → `lock().method()` 패턴

**현황**: 3,810건의 `lock_ok()` 중 대부분이 이 패턴.

```rust
// Before (Option chain)
let source = edge.lock_ok().and_then(|e| e.source());

// After (direct)
let source = edge.lock().source();
```

`lock_ok().and_then(|g| g.method())` 에서 `and_then`은 `Option<MutexGuard>` → `Option<T>`로 변환.
`lock().method()` 에서 `method()`가 `Option<T>`를 반환하면 결과 타입이 동일하다.

**작업**: crate별로 진행, 각 단계에서 build + test 검증
1. 단순 패턴 (and_then + 단일 메서드) → 기계적 치환
2. 복합 패턴 (and_then + 다중 연산) → guard 변수 도입
3. map + unwrap_or 패턴 → guard 변수 + 직접 접근

**예상 제거**: ~2,500건 (나머지 ~1,300건은 closures/다른 스코프에서 필요)
**Parity 위험**: 없음

### B-2. 잔여 lock_ok() 중 guard 재사용 가능 패턴

Phase 1-B의 확장. 상위 10개 파일에서 동일 객체 반복 lock_ok() → 단일 lock() + guard 재사용.

**대상 파일** (lock_ok 빈도 상위):
- label_and_node_size_processor.rs (92)
- hierarchical_port_orthogonal_edge_router.rs (89)
- l_graph_util.rs (85)
- l_graph_adapters.rs (73)
- components_processor.rs (71)
- abstract_barycenter_port_distributor.rs (60)
- breaking_point_processor.rs (56)
- elk_graph_importer.rs (56)
- final_spline_bendpoints_calculator.rs (53)
- horizontal_graph_compactor.rs (52)

---

## Phase C: Force/Stress Arena 파일럿 (중위험)

### C-1. Arena/Index 설계

Force 그래프(FGraph/FNode/FEdge/FBendpoint/FLabel)를 `Arc<Mutex<T>>` → arena + typed index로 전환.

```rust
// Before
pub type FNodeRef = Arc<Mutex<FNode>>;
pub type FEdgeRef = Arc<Mutex<FEdge>>;

// After
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FNodeId(u32);
pub struct FArena {
    nodes: Vec<FNode>,       // indexed by FNodeId
    edges: Vec<FEdge>,       // indexed by FEdgeId
    labels: Vec<FLabel>,
    bendpoints: Vec<FBendpoint>,
}
```

**작업**:
1. `FArena` 구조체 설계 (SoA + typed index)
2. FNode/FEdge/FLabel/FBendpoint를 owned struct으로 전환 (self-ref 제거)
3. ElkGraphImporter → FArena 빌더
4. 모든 알고리즘 코드 (EadesModel, FruchtermanReingold, StressMajorization) 업데이트
5. ElkGraph layout transferrer 업데이트

**범위**: ~15파일, ~2,500줄
**Parity 위험**: 중간 — Force/Stress model parity 재검증 필수
**성능 기대**: lock overhead 제거, cache locality 개선

### C-2. 검증

- Force/Stress 전용 model parity (모든 force/stress 모델)
- Phase-step trace 비교
- 벤치마크: before/after 비교 (force_medium/large/xlarge, stress_medium/large/xlarge)

---

## Phase D: LGraph Arena 확장 (고위험, 장기)

### D-1. LArena 확장 범위 분석

현재 LArena(1,315줄)는 CSR 기반 읽기 전용 스냅샷으로, 핫루프에서만 사용.
Phase D는 LArena를 **쓰기 가능한 주 저장소**로 승격시키는 것이 목표.

**전제 조건**:
- Phase C 성공 (arena 패턴 검증)
- Phase A, B 완료 (`&self` 기반, lock_ok 최소화)

**작업**:
1. LArenaBuilder를 mutation-capable arena로 확장
2. 프로세서별로 점진 전환 (hot → cold 순서)
3. Arc<Mutex<LNode>> 타입 별칭을 arena index로 교체
4. 338건 Vec clone 자연 소멸

**범위**: ~150파일, 수만 줄
**Parity 위험**: 높음 — 전체 model parity + phase-step trace 재검증 필수
**별도 브랜치에서 진행 권장**

### D-2. ElkGraph Arena (Phase 5-B)

LGraph arena 안정화 후 ElkGraph 레이어(Rc<RefCell<T>>) → arena 전환.
JSON import/export 전체 경로 변경. 별도 프로젝트.

---

## 실행 순서 및 의존성

```
Phase A-1 (get_property &self)
    ↓
Phase A-2 (typed wrapper)     Phase B-1 (lock_ok → lock 기계적 치환)
                                  ↓
                              Phase B-2 (guard 재사용 확장)
                                  ↓
                              Phase C-1 (Force arena 파일럿)
                                  ↓
                              Phase C-2 (검증)
                                  ↓
                              Phase D-1 (LGraph arena 확장) — 별도 브랜치
                                  ↓
                              Phase D-2 (ElkGraph arena) — 별도 프로젝트
```

## 우선순위 권장

| 순위 | Phase | 효과 | 난이도 | 위험 |
|------|-------|------|--------|------|
| 1 | **A-1** | &self 통일, 함수 시그니처 단순화 | 중 (기계적 치환) | 없음 |
| 2 | **B-1** | 3,810 → ~1,300 lock_ok, 가독성 대폭 개선 | 중 (기계적 치환) | 없음 |
| 3 | **A-2** | 상위 156건 property 접근 단순화 | 소 | 없음 |
| 4 | **B-2** | 추가 lock_ok 제거, guard 통합 | 중 | 없음 |
| 5 | **C** | 아키텍처 전환 파일럿, lock 제거 | 대 | 중간 |
| 6 | **D** | 전체 아키텍처 전환 | 대규모 | 높음 |

## lock_ok() → lock() 변환 규칙 (2026-03-22 교훈)

### 근본 원인

`lock_ok().and_then(|guard| ...)` 에서 `and_then` closure가 guard 수명을 제어한다.
`lock().method_chain()` 으로 변환하면 temporary guard가 전체 표현식 끝까지 유지되어,
같은 mutex를 재잠금하는 코드에서 **데드락** 또는 **동작 차이**가 발생한다.

### 안전한 변환 패턴

```rust
// BEFORE (lock_ok)
x.lock_ok().and_then(|g| g.method())

// SAFE: 명시적 변수 바인딩 (guard 범위 명확)
{ let g = x.lock(); g.method() }

// DANGEROUS: temporary chain (guard가 표현식 끝까지 유지)
x.lock().method().and_then(|y| y.lock()...)  // 데드락 위험!
```

### 필수 규칙

1. **guard를 항상 명시적 변수에 바인딩**: `let guard = x.lock();`
2. **guard 범위를 블록으로 제한**: `{ let guard = x.lock(); ... }`
3. **체인에서 temporary guard 금지**: `x.lock().ports().and_then(...)` 대신 `{ let g = x.lock(); let ports = g.ports().clone(); drop(g); ... }`
4. **같은 객체 재잠금 전 guard 명시적 해제**: 부모 노드 lock → 자식에서 `absolute_anchor()` 등 부모 재잠금 시 반드시 `drop(guard)` 선행
5. **매 batch 변환 후 model parity 검증** (Java baseline 대비)

### 실패 사례 (롤백됨)

- `d5bb558` ~ `5b25f1f` (17 커밋): 기계적 sed로 `lock_ok().and_then(|g| g.method())` → `lock().method()` 변환
- `LongEdgeJoiner::join_at`에서 `node.lock().ports().first().and_then(|p| p.lock().absolute_anchor())` → 데드락
- 1988/1989 → 1940/1974 parity 회귀 (34 drift)
- 원인: temporary guard 수명 연장으로 인한 미묘한 lock 순서/타이밍 변화

## 각 Phase 완료 시 품질 게이트

1. `cargo build --workspace` (error/warning 0건)
2. `cargo clippy --workspace --all-targets` (warning 0건)
3. `cargo test --workspace` (failure 0건)
4. **Phase B 이상: model parity 재검증 필수** (lock_ok 변환은 parity에 영향)
5. Phase C 이상: phase-step trace 재검증
6. Phase D: JS parity 포함 전체 재검증
