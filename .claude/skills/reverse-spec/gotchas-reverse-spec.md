# Gotchas: Reverse-spec

<!-- Reverse-spec extraction patterns. Updated via reflection protocol. -->

## 규칙 (3회+)

(아직 없음)

## 주의 (2회)

(아직 없음)

## 관찰 (1회)

### Rust workspace = 자연스러운 spec 경계 (2026-04-01, elk-rs)
- Cargo workspace의 크레이트 구조가 spec 분해와 1:1로 대응했음 (1 crate ≈ 1 feature 또는 _shared)
- 여러 알고리즘이 한 크레이트에 공존할 경우 (force+stress) 자연스럽게 sub-feature로 그룹화됨
- 이 패턴은 Rust workspace 기반 프로젝트에서 반복될 가능성 높음

### 스텁/미구현 코드 → 빈 섹션 허용 (2026-04-01, elk-rs)
- 외부 툴 연동이 미포팅된 알고리즘(libavoid, graphviz)은 Edge Cases 등에서 빈 섹션이 발생
- 억지로 채우지 않고 `<!-- 코드에서 추출 불가 -->` 플레이스홀더를 남기는 것이 정확함
- 이런 스텁 spec은 reverse-spec-review Phase 1에서 별도 처리 필요

### 소비자 수 교차 검증 필수 (2026-04-01, elk-rs)
- _shared spec의 "N개 크레이트가 의존" 같은 수치 주장은 개별 feature spec의 의존성 기술과 교차 검증 필요
- algorithm-commons가 "11개"라 했으나 topdownpacking이 실제로는 미의존 → 10개로 수정
- 자동화 가능: Cargo.toml의 [dependencies]를 파싱하면 정확한 소비자 수 산출 가능
