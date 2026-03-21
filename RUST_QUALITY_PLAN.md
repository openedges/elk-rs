# Rust 코드 품질 개선 계획

## 원칙

모든 개선 작업은 다음 제약을 따른다:

1. **Parity 불변**: 각 단계 완료 후 model parity + phase-step trace 100% 유지
2. **점진적 적용**: 한 번에 한 패턴씩, 기계적으로 변환 가능한 것부터
3. **동작 무변경**: 리팩토링만 수행, 로직 변경 없음

## 현황 (2026-03-21 측정)

| 패턴 | 건수 | 파일 수 | 심각도 |
|------|------|---------|--------|
| `.lock().ok()` 체인 | 600 | 135 | 가독성/의미론 |
| `Arc::ptr_eq` identity 비교 | 367 | 106 | 성능/관용성 |
| `ELK_TRACE_*` / `ELK_DEBUG_*` 환경변수 | 40+ | 48 | 코드 위생 |
| `#[allow(clippy::too_many_arguments)]` | 25+ | 20+ | 구조적 |
| `#[allow(clippy::mutable_key_type)]` | 파일 수준 | 2 | 안전성 |

---

## Phase 1: Lock 접근 패턴 정리 (저위험, 고효과)

### 1-A. `elk_mutex` API 변경 + `.lock()` 통일

**대상**: 600건 `.lock().ok()` + 46건 `.lock().unwrap()` / 135+ 파일
**Parity 위험**: 없음

**문제**: `parking_lot::Mutex`는 poisoning이 없어 `lock()`이 절대 실패하지 않는다.
`.ok()`는 "실패할 수 있다"는 잘못된 의미를 전달하고, `Option` 체인을 유발하여 가독성을 심각하게 해친다.

```rust
// Before: 의미론적으로 기만적, Option 전파로 가독성 저하
let source = self.edge.lock().ok().and_then(|edge| edge.source());
let node = source.lock().ok().and_then(|port| port.node());
let node_type = node.lock().ok().map(|node| node.node_type())?;

// After: 직접 접근
let source = self.edge.lock().source();
let node = source.lock().node();
let node_type = node.lock().node_type();
```

**구현**: `elk_mutex.rs`의 `lock()` 반환 타입을 `Result<MutexGuard>` → `MutexGuard`로 변경한다.

```rust
// elk_mutex.rs 변경
impl<T: ?Sized> Mutex<T> {
    #[inline]
    pub fn lock(&self) -> MutexGuard<'_, T> {
        MutexGuard(self.0.lock())
    }
}
```

이후 전체 코드에서 기계적 치환:
- `.lock().ok().and_then(|x| ...)` → `.lock()` + 직접 접근
- `.lock().ok().map(|x| ...)` → `.lock()` + 직접 접근
- `.lock().unwrap()` → `.lock()`

**검증**: `cargo build --workspace` + `cargo test --workspace`

### 1-B. 중복 lock guard 재사용

**대상**: 핫패스 우선 (compound/mod.rs, spline_edge_router.rs, network_simplex_placer.rs 등)
**Parity 위험**: 없음 (lock 순서 변경에만 주의)

**문제**: 같은 객체를 같은 스코프에서 여러 번 lock하는 패턴이 빈번하다.

```rust
// Before: 동일 node를 3번 lock
let node_type = node.lock().node_type();
let origin = node.lock().get_property(InternalProperties::ORIGIN);
let graph = node.lock().graph();

// After: 1번 lock, guard 재사용
let guard = node.lock();
let node_type = guard.node_type();
let origin = guard.get_property(InternalProperties::ORIGIN);
let graph = guard.graph();
```

**우선 대상 파일** (lock 빈도 상위):
- `compound/mod.rs` (38건)
- `spline_edge_router.rs` (36건)
- `network_simplex_placer.rs` (31건)
- `barycenter_heuristic.rs` (12건)
- `elk_graph_layout_transferrer.rs` (13건)
- `horizontal_graph_compactor.rs` (11건)

**검증**: `cargo build --workspace` + `cargo test --workspace`

---

## Phase 2: 디버그 인프라 정리 (저위험, 중효과)

### 2-A. `ELK_TRACE_*` 환경변수를 통합 trace 모듈로 전환

**대상**: 40+ 개별 `LazyLock<bool>` / 48 파일
**Parity 위험**: 없음 (trace 출력은 레이아웃 결과에 영향 없음)

**현황**: 파일마다 독립적 선언

```rust
// 현재: 파일마다 개별 선언
static TRACE_CROSSMIN: LazyLock<bool> = LazyLock::new(|| std::env::var_os("ELK_TRACE_CROSSMIN").is_some());
static TRACE_CROSSMIN_TIMING: LazyLock<bool> = LazyLock::new(|| ...);
static TRACE_CROSSMIN_STATS: LazyLock<bool> = LazyLock::new(|| ...);
```

**구현**: 중앙 trace 모듈 신규 생성

```rust
// elk_trace.rs (신규, core crate 내)
pub struct ElkTrace {
    pub crossmin: bool,
    pub crossmin_timing: bool,
    pub crossmin_stats: bool,
    pub sizing: bool,
    pub stress: bool,
    pub ortho: bool,
    // ... 전체 통합
}

static INSTANCE: LazyLock<ElkTrace> = LazyLock::new(|| ElkTrace {
    crossmin: std::env::var_os("ELK_TRACE_CROSSMIN").is_some(),
    // ...
});

impl ElkTrace {
    pub fn global() -> &'static ElkTrace { &INSTANCE }
}

// 사용처 변경
if ElkTrace::global().crossmin { ... }
```

**분류 기준** — 두 종류를 분리한다:
- **parity 검증용** (`ELK_TRACE_DIR`): 유지 — phase-step trace 인프라의 핵심
- **디버그용** (`ELK_TRACE_CROSSMIN`, `ELK_TRACE_SIZING` 등): 통합 모듈로 이동

**검증**: 기존 환경변수 동작 호환성 확인 + `cargo test --workspace`

### 2-B. `#[allow(clippy::*)]` 억제 정리

**대상**: 25+ 건
**Parity 위험**: 없음

| 억제 | 건수 | 대응 |
|------|------|------|
| `too_many_arguments` | 25 | 파라미터 구조체(config/context struct) 도입 |
| `mutable_key_type` | 2 | `Arc` 키의 의도를 newtype wrapper로 명시 |
| `inherent_to_string` | 1 | `Display` trait 구현으로 교체 |

**예시** (`too_many_arguments`):

```rust
// Before
#[allow(clippy::too_many_arguments)]
fn route_edge(source: &NodeId, target: &NodeId, offset: f64, spacing: f64,
              direction: Direction, bend_type: BendType, ...) { }

// After
struct EdgeRoutingContext {
    source: NodeId,
    target: NodeId,
    offset: f64,
    spacing: f64,
    direction: Direction,
    bend_type: BendType,
}
fn route_edge(ctx: &EdgeRoutingContext) { }
```

**검증**: `cargo clippy --workspace --all-targets` (0 warning) + `cargo test --workspace`

---

## Phase 3: Identity/Collection 패턴 개선 (중위험, 고효과)

### 3-A. `Arc::ptr_eq` 기반 제거/검색 개선

**대상**: 367건 / 106파일 (핫패스 우선)
**Parity 위험**: 낮음 (제거 순서 동일 보장 필요)

**문제**: `remove_arc`가 O(n) 선형 탐색으로 `Arc::ptr_eq`를 사용한다.

```rust
// 현재: O(n) 선형 탐색
pub(crate) fn remove_arc<T>(items: &mut Vec<Arc<Mutex<T>>>, target: &Arc<Mutex<T>>) -> bool {
    if let Some(pos) = items.iter().position(|item| Arc::ptr_eq(item, target)) {
        items.remove(pos);
        true
    } else { false }
}
```

**개선 방안**:
1. 호출 빈도가 높은 곳에 `HashMap<usize, usize>` (ptr → index) 보조 인덱스 도입
2. `element_id` 기반 검색 헬퍼 제공 (LArena의 타입화된 인덱스 활용)
3. `Vec::remove` → `swap_remove`는 순서 의존 알고리즘에 영향 가능하므로 **phase-step trace로 반드시 검증**

**검증**: model parity + phase-step trace 전체 재실행

### 3-B. 불필요한 `.clone()` 제거

**대상**: 프로파일링 기반 선별
**Parity 위험**: 낮음

`Arc::clone()`은 atomic increment라 저비용이지만, `Vec<Arc<T>>.clone()` 같은 컬렉션 수준 clone이 반복 루프 안에서 발생하는 경우가 있다. 핫패스의 프로파일링 결과를 기반으로 선별 적용한다.

**검증**: `cargo test --workspace` + 벤치마크 비교

---

## Phase 4: 타입 안전성 점진적 강화 (중위험, 장기 효과)

### 4-A. Property 시스템에 typed wrapper 도입

**대상**: 빈번 사용 property 상위 20~30개
**Parity 위험**: 없음 (기존 경로의 wrapper)

**현재**: 모든 property 접근이 `Arc<dyn Any>` 다운캐스트를 거친다.

```rust
// 런타임 타입 오류 가능
let value: Option<f64> = node.get_property(SomeProperty::SPACING);
```

**개선**: 자주 사용되는 property에 대해 typed accessor를 추가한다.

```rust
impl LNode {
    pub fn spacing(&self) -> f64 {
        self.get_property(LayeredOptions::SPACING).unwrap_or(10.0)
    }
}
```

**검증**: `cargo test --workspace`

### 4-B. `Cow<'static, str>` 키를 `PropertyId` newtype으로

**대상**: `properties/mod.rs` 1파일 + 사용처
**Parity 위험**: 없음

```rust
// Before: 문자열 기반 - 오타에 취약
property_map: FxHashMap<Cow<'static, str>, PropertyValue>

// After: 타입화된 키
#[derive(Clone, Hash, Eq, PartialEq)]
pub struct PropertyId(Cow<'static, str>);
```

**검증**: `cargo build --workspace` + `cargo test --workspace`

---

## Phase 5: 구조적 개선 (고위험, 장기 효과) -- 선택적

이 Phase는 parity에 미치는 영향이 크므로 **필요 시에만** 진행한다.

### 5-A. Force/Stress 그래프의 `Arc<Mutex<T>>` -> owned struct 전환

**대상**: `org.eclipse.elk.alg.force` crate 내 FGraph/FNode/FEdge
**Parity 위험**: 중간 -- force/stress 전용 parity 테스트로 검증 가능

Force/Stress 알고리즘의 그래프는 이미 SoA 최적화되어 있지만, 기반 타입이 여전히 `Arc<Mutex<T>>`이다. 이 모듈은 다른 알고리즘과 독립적이므로 owned struct으로 전환해도 영향 범위가 제한된다.

**검증**: force/stress model parity 전체 + phase-step trace

### 5-B. ElkGraph 레이어의 `Rc<RefCell<T>>` -> arena 전환

**대상**: `org.eclipse.elk.graph` + `org.eclipse.elk.graph.json` crate
**Parity 위험**: 높음 -- import/export 전체 경로 변경

ElkGraph 입출력 레이어에서 사용되는 `Rc<RefCell<T>>`는 단일 스레드 전용이며, 모듈 범위가 한정적이므로 arena 기반으로 전환 가능하다.

**검증**: 전체 model parity + phase-step trace + JS parity 재검증 필수

---

## 실행 순서 요약

| Phase | 작업 | 건수 | Parity 위험 | 효과 | 선행 조건 |
|-------|------|------|------------|------|-----------|
| **1-A** | `elk_mutex` API 변경 + `.lock()` 통일 | 646건 / 135파일 | 없음 | 가독성 대폭 개선 | 없음 |
| **1-B** | 중복 lock guard 재사용 | 핫패스 우선 | 없음 | 가독성 + 미세 성능 | 1-A |
| **2-A** | trace 환경변수 통합 모듈 | 48파일 | 없음 | 코드 위생 | 없음 |
| **2-B** | clippy allow 억제 정리 | 25+건 | 없음 | 구조 품질 | 없음 |
| **3-A** | `Arc::ptr_eq` 기반 검색 개선 | 핫패스 우선 | 낮음 | 성능 + 관용성 | 1-A |
| **3-B** | 불필요한 clone 제거 | 프로파일 기반 | 낮음 | 성능 | 1-A |
| **4-A** | Property typed wrapper | 상위 20~30개 | 없음 | 타입 안전성 | 없음 |
| **4-B** | PropertyId newtype | 1파일 | 없음 | 타입 안전성 | 없음 |
| **5-A** | Force/Stress owned struct | 모듈 한정 | 중간 | 관용성 + 성능 | 1-A, 3-A |
| **5-B** | ElkGraph arena 전환 | 모듈 한정 | 높음 | 관용성 + 성능 | 5-A |

Phase 2~4는 Phase 1 이후 병렬 또는 순차 진행, Phase 5는 필요에 따라 선택적으로 진행한다.

## 완료 이력

### Phase 1-A 완료 (2026-03-21)

- `elk_mutex.rs`: `lock()` 반환 타입 `Result<MutexGuard>` -> `MutexGuard` 직접 반환
- `lock_ok()` 호환 메서드 추가 (`Option<MutexGuard>`, 항상 Some) -- Option 체인 필요 시 사용
- `try_lock()` 반환 타입 `Result<MutexGuard, TryLockError>` -> `Option<MutexGuard>`
- 282 파일 변경, +5,158 / -7,420 줄 (순감 -2,262줄)
- 품질 게이트: build 0 warning, clippy 0 warning, 802 tests passed / 0 failed

### Phase 1-B 완료 (2026-03-21)

- 핫패스 3개 파일에서 중복 lock guard 통합
- compound/mod.rs: 150 -> 131 lock_ok() (19건 제거)
- network_simplex_placer.rs: 144 -> 123 lock_ok() (21건 제거)
- spline_edge_router.rs: 85 -> 60 lock_ok() (25건 제거)
- 총 65건 중복 lock 제거, 3 파일 변경, -70줄

### Phase 2-A 완료 (2026-03-21)

- `core::util::elk_trace::ElkTrace` 통합 모듈 신규 생성 (매크로 기반)
- 60+ bool/string 환경변수 플래그를 단일 struct로 통합, `ElkTrace::global()` 접근
- 44 파일에서 ~90개 `LazyLock<bool>` static 선언 제거
- `ELK_TRACE_DIR` (parity 인프라)과 `ELK_TRACE_SIZING` (graph crate) 의도적 유지
- 45 파일 변경, -31줄

### Phase 2-B 완료 (2026-03-21)

- 6개 `#[allow(clippy::inherent_to_string)]` 제거 -> `impl fmt::Display` 교체
- 대상: LEdge, LGraph, LLabel, LNode, LPort, Layer
- `&mut self` -> `&self` 정리 (designation/get_designation 등 비수정 메서드)
- 24 파일 변경
- `too_many_arguments` (25건): meta_data_provider 등록 코드는 유지, 알고리즘 코드 개선은 추후

### Phase 4-B 완료 (2026-03-21)

- `PropertyId` newtype 도입 — `Cow<'static, str>` 대신 타입화된 키
- `MapPropertyHolder`의 HashMap 키를 `PropertyId`로 교체
- `Hash`, `Eq`, `Borrow<str>`, `From<&str>`, `From<String>`, `Into<String>`, `Display` 구현
- 5 파일 변경 (+101 / -22)

### 추후 진행 항목

- **Phase 4-A** (Property typed wrapper): 상위 20~30개 property에 typed accessor 추가 — 광범위한 설계 작업, 점진적 적용 필요
- **Phase 3-A** (Arc::ptr_eq 검색 개선): 분석 결과 `remove_arc`/`index_of_arc`는 그래프 구조 변경 시에만 호출, 레이아웃 핫루프는 arena/CSR 사용 — 실제 성능 영향 낮아 보류
- **Phase 3-B** (불필요한 clone 제거): 프로파일링 기반 선별 적용 필요
- **Phase 5-A/5-B** (구조적 개선): 고위험, 필요 시 선택적 진행

## 각 Phase 완료 시 품질 게이트

1. `cargo build --workspace` (error/warning 0건)
2. `cargo clippy --workspace --all-targets` (warning 0건)
3. `cargo test --workspace` (failure 0건)
4. Phase 3 이상: model parity + phase-step trace 재검증
5. Phase 5: JS parity 포함 전체 재검증
