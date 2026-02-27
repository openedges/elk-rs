# elk-rs 버전 관리 및 운영 정책

## Context

elk-rs는 Java ELK (Eclipse Layout Kernel)의 Rust 포팅이며, elkjs 호환 npm 패키지도 제공한다. 현재 상태:

- **ELK Java** (`external/elk`): `v0.11.0` 태그 기준, 서브모듈은 `0.12.0-SNAPSHOT` 커밋(`c831ba46`)에 고정
- **elkjs** (`external/elkjs`): `0.11.0` (package.json), 서브모듈은 `0.9.2` 태그 이후 53 커밋(`cd3ed78`)
- **elk-rs**: 모든 Rust 크레이트 `0.1.0`, npm `elk-rs@0.1.0`, git 태그 없음
- **Parity**: 1438/1439 모델 100% 일치 (ELK 0.11.0 기준 포팅 완료)

세 프로젝트의 버전을 일치시키면서 포팅/검증/릴리즈를 체계적으로 관리하는 정책이 필요하다.

---

## 1. 버전 관리 체계

### 원칙: ELK 버전 완전 일치

elk-rs의 버전은 포팅 대상 ELK Java 버전과 `MAJOR.MINOR.PATCH` 모두 동일하게 유지한다.

```
elk-rs 0.11.0  =  ELK Java 0.11.0  =  elkjs 0.11.0
```

| 프로젝트 | 현재 | 첫 정식 릴리즈 | 다음 |
|----------|------|---------------|------|
| ELK Java | 0.11.0 (→ 0.12.0-SNAPSHOT) | — | 0.12.0 |
| elkjs | 0.11.0 | — | 0.12.0 |
| elk-rs (Rust crates) | 0.1.0 | **0.11.0** | 0.12.0 |
| elk-rs (npm) | 0.1.0 | **0.11.0** | 0.12.0 |

**규칙:**
- elk-rs `MAJOR.MINOR.PATCH`는 포팅 대상 ELK Java 버전과 항상 동일
- 현재 `0.1.0`은 프리릴리즈 → 첫 정식은 `0.11.0`으로 점프 (포팅 완료된 ELK 버전 반영)
- ELK가 `0.11.1`을 릴리즈하면 elk-rs도 해당 변경 포팅 후 `0.11.1`로 릴리즈
- Rust workspace 전체 크레이트와 npm 패키지는 동일 버전 유지

### elk-rs 자체 변경 관리

elk-rs 고유 버그픽스나 기능 추가는 **버전을 올리지 않고** 다음 체계로 관리한다.

#### 릴리즈 사이 변경: 빌드 메타데이터 + CHANGELOG

```
npm: elk-rs@0.11.0       ← 정식 릴리즈 (ELK 0.11.0 parity)
git: v0.11.0+rs.1        ← elk-rs 자체 패치 1회차
git: v0.11.0+rs.2        ← elk-rs 자체 패치 2회차
npm: elk-rs@0.11.0       ← npm 버전은 동일 유지 (재publish 금지)
```

- **git 태그**: `v{ELK_VERSION}+rs.{N}` 형식으로 elk-rs 자체 변경을 추적
  - semver 빌드 메타데이터(`+`)이므로 버전 우선순위에 영향 없음
  - 예: `v0.11.0+rs.1`, `v0.11.0+rs.2`
- **npm 재publish 금지**: 동일 ELK 버전에서는 npm 패키지를 재publish하지 않음
  - elk-rs 자체 수정이 누적되면 다음 ELK 릴리즈(예: `0.11.1` 또는 `0.12.0`)에 함께 배포
  - 긴급 수정이 필요한 경우에만 예외적으로 npm에 prerelease 배포: `0.11.0-rs.1`
- **CHANGELOG.md**: elk-rs 자체 변경은 `CHANGELOG.md`에 ELK 버전 단위로 그룹화하여 기록

#### CHANGELOG 형식

```markdown
## 0.11.0 (ELK 0.11.0)

### elk-rs 자체 변경 (v0.11.0+rs.1 ~ v0.11.0+rs.N)
- [bugfix] WASM fallback 에러 메시지 개선
- [feature] NAPI darwin-arm64 addon 추가
- [perf] layered algorithm 20% 성능 개선

### ELK 포팅
- ELK 0.11.0 전체 포팅 완료 (1438/1439 parity)
```

### 버전 동기화 파일

| 파일 | 버전 필드 |
|------|-----------|
| `Cargo.toml` (각 크레이트) | `version = "0.11.0"` |
| `plugins/org.eclipse.elk.js/package.json` | `"version": "0.11.0"` |
| `AGENTS.md` 핵심 스냅샷 | elk-rs 버전 = ELK 버전 기록 |
| `CHANGELOG.md` | ELK 버전별 변경 이력 |

### Cargo workspace 버전 통합

루트 `Cargo.toml`에 `workspace.package.version`을 도입해 모든 크레이트가 단일 소스에서 버전을 상속:

```toml
# Cargo.toml (root)
[workspace.package]
version = "0.11.0"
license = "EPL-2.0"
edition = "2021"

# 각 크레이트 Cargo.toml
[package]
name = "org-eclipse-elk-core"
version.workspace = true
```

---

## 2. 서브모듈 고정 정책

### 서브모듈 커밋 고정 규칙

| 서브모듈 | 고정 대상 | 근거 |
|----------|-----------|------|
| `external/elk` | 릴리즈 태그 (예: `v0.11.0`) | parity 기준점, 재현성 보장 |
| `external/elkjs` | 릴리즈 태그 (예: `0.11.0`) | JS parity 기준점 |

**현재 문제:** 두 서브모듈 모두 릴리즈 태그가 아닌 SNAPSHOT/dev 커밋에 고정.

**조치:**
1. `external/elk`를 `v0.11.0` 태그로 되돌림 (현재 parity가 이 버전 기준)
2. `external/elkjs`를 `0.11.0` 태그로 되돌림
3. 서브모듈 변경 시 반드시 커밋 메시지에 `chore: pin external/elk to v0.X.Y` 기록

---

## 3. 브랜치 및 태그 정책

### 브랜치 전략

```
main                    ← 안정 릴리즈 브랜치 (항상 릴리즈 가능 상태)
├─ port/0.12.0          ← ELK 0.12.0 포팅 작업 브랜치
├─ feature/*            ← elk-rs 자체 기능 개발 (NAPI, 최적화 등)
└─ fix/*                ← 버그 수정
```

**규칙:**
- `main`은 항상 빌드/테스트/parity 통과 상태
- ELK 새 버전 포팅은 `port/X.Y.Z` 브랜치에서 진행
- 포팅 완료 + 전체 검증 통과 후 `main`에 머지
- feature/fix 브랜치는 `main`에서 분기, PR로 머지

### 태그 전략

```
v0.11.0                 ← 정식 릴리즈 (ELK 0.11.0 parity, npm publish)
v0.11.0+rs.1            ← elk-rs 자체 패치 (npm 미배포)
v0.11.0+rs.2            ← elk-rs 자체 패치 추가 (npm 미배포)
v0.12.0                 ← 정식 릴리즈 (ELK 0.12.0 parity, npm publish)
```

**규칙:**
- 정식 릴리즈 태그: `v{ELK_VERSION}` — ELK 버전과 동일, npm publish 동반
- elk-rs 자체 변경 태그: `v{ELK_VERSION}+rs.{N}` — npm 미배포, git 추적 전용
- 태그 생성 시 annotated tag 사용: `git tag -a v0.11.0 -m "elk-rs 0.11.0 (ELK 0.11.0 parity)"`
- 정식 릴리즈 태그는 `main` 브랜치에만 부여
- `+rs.N` 태그는 `main`의 중간 커밋에 부여 가능

---

## 4. 포팅 워크플로우 (ELK 새 버전 대응)

### 단계별 절차

```
1. 준비          서브모듈 업데이트 + diff 분석
2. 포팅          Java 변경분을 Rust에 반영
3. 검증          parity 게이트 전체 통과
4. 릴리즈        버전 업데이트 + 태그 + publish
```

### Step 1: 준비

```sh
# 포팅 브랜치 생성
git checkout -b port/0.12.0 main

# 서브모듈을 새 릴리즈 태그로 업데이트
git -C external/elk checkout v0.12.0
git -C external/elkjs checkout 0.12.0  # elkjs도 해당 버전이 나온 경우

# Java 변경분 확인
git -C external/elk diff v0.11.0..v0.12.0 --stat
git -C external/elk diff v0.11.0..v0.12.0 -- plugins/  # 플러그인별 변경 확인

# 변경 영향도 분석 → HISTORY.md에 기록
```

### Step 2: 포팅

포팅 순서 (의존성 순):
1. `org.eclipse.elk.graph` (데이터 모델 변경)
2. `org.eclipse.elk.core` (코어 옵션/엔진 변경)
3. `org.eclipse.elk.alg.common` (공통 유틸리티)
4. 개별 알고리즘 (`layered`, `force`, `mrtree`, `radial`, `disco`, `rectpacking`, `spore`)
5. `org.eclipse.elk.graph.json` (JSON 임포터/익스포터)
6. `org.eclipse.elk.wasm` / `org.eclipse.elk.napi` (바인딩, 보통 변경 없음)
7. `org.eclipse.elk.js` (JS API, 보통 변경 없음)

각 크레이트 포팅 후:
```sh
cargo build --workspace          # 빌드 확인
cargo clippy --workspace --all-targets  # lint
cargo test --workspace           # 단위 테스트
```

### Step 3: 검증 (전체 게이트)

`RELEASE_CHECKLIST.md`의 검증 플로우 전체 실행:

```sh
# 1. Phase wiring parity
LAYERED_PHASE_WIRING_PARITY_STRICT=true sh scripts/check_layered_phase_wiring_parity.sh

# 2-4. Build + Clippy + Test
cargo build --workspace && cargo clippy --workspace --all-targets && cargo test --workspace

# 5. Full model parity (새 Java baseline 필요)
sh scripts/run_model_parity_elk_vs_rust.sh external/elk-models parity/model_parity_full

# 6. JS parity
cd plugins/org.eclipse.elk.js && npm test && npm run test:parity

# 7. Phase-step trace verification (선택, 드리프트 발생 시)
```

### Step 4: 릴리즈

```sh
# 버전 업데이트 (모든 Cargo.toml + package.json)
# 서브모듈 커밋 확정
git add -A && git commit -m "release: elk-rs 0.12.0 (ELK 0.12.0 parity)"
git tag -a v0.12.0 -m "elk-rs 0.12.0 (ELK 0.12.0 parity)"

# npm publish
cd plugins/org.eclipse.elk.js && sh build.sh && npm publish

# push
git push origin main --tags
```

---

## 5. 검증 플로우 (버전별)

### 일상 개발 (커밋 단위)

```
cargo build --workspace → cargo clippy → cargo test
```

### PR 머지 전

```
위 + model parity (skip java export) + JS test
```

### 릴리즈 전 (RELEASE_CHECKLIST.md 전체)

```
wiring parity → build → clippy → test → model parity (full) → JS parity
→ phase-step verification → performance baseline → release build
```

### 새 ELK 버전 포팅 시 (추가)

```
위 전체 + 새 Java baseline 생성 + diff 분석 + HISTORY.md 기록
```

---

## 6. 커스터마이징 정책

elk-rs는 Java ELK의 1:1 포팅이 원칙이나, 다음 세 가지 범주의 차이를 허용한다.

### 허용되는 차이 (문서화 필수)

| 범주 | 정책 | 예시 |
|------|------|------|
| **Java ELK 버그 회피** | Java 버그를 복제하지 않음. HISTORY.md에 근거 기록 | `213_componentsCompaction` NaN 전파 |
| **Rust 관용적 변환** | 동일 동작을 Rust 관용구로 표현 | Iterator, enum dispatch, ownership |
| **elk-rs 전용 기능** | upstream에 없는 추가 기능 (API 호환 유지) | NAPI addon, WASM 최적화, 추가 CLI |

### 금지되는 차이

| 범주 | 이유 |
|------|------|
| 알고리즘 동작 변경 | parity 깨짐, 사용자 예측 불가 |
| API 비호환 변경 | elkjs drop-in 대체 불가 |
| 옵션/프로퍼티 의미 변경 | 기존 그래프 설정 결과 달라짐 |

### 커스터마이징 기록 규칙

1. elk-rs 전용 변경은 반드시 `HISTORY.md`에 `[elk-rs only]` 태그로 기록
2. Java ELK 버그 회피는 `[java-bug]` 태그로 기록, Java 이슈 번호 참조
3. 향후 upstream이 같은 변경을 채택하면 태그를 `[upstream-merged]`로 갱신
4. `parity/PARITY.md`의 "Known Drifts" 섹션에 허용된 차이 목록 유지
