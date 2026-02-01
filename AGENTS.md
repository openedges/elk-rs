# ELK Layout Kernel for Pure Rust

- ELK Layout Java Version을 Pure Rust 버전으로 Porting
- Java Version의 모든 기능, API, 테스트를 동일하게 Rust 버전으로 이식
- 데이터 모델, 파일 스트럭처 모두 동일하게 유지
- 구현하면서 Clippy 테스트하고 빌드가 항상 통과하도록 유지
- 구현하면서 성능 측정하고 Java Version보다 항상 빠르게 동작하도록 유지
- 라이센스는 Java Version과 동일하게 유지

## 진행된 작업
- `.gitignore` 추가, `external/elk` 서브모듈 추가
- core/math, graph, util, options, data 기본 구조 및 일부 기능 포팅
- `ElkUtil` 확장: 절대/상대 좌표, 벡터 체인, junction points, 스케일링, child area 계산, 기본값 설정, 방문자/검증, 디버그 경로/문자열
- `CoreOptions` 확장: algorithm/resolved, alignment/aspect ratio, bend points, position, priority, random seed, separate components, padding, label 배치 옵션
- graph util 확장: all_incoming/outgoing/incidents (node/shape)
- validation 모듈 추가 (`GraphIssue`, `GraphValidationException`)
- 테스트 추가 (`kvector`, `kvector_chain`, `elk_util` 등), `cargo clippy --workspace --all-targets` 통과

## 다음 작업
- Java CoreOptions 나머지 옵션/enum/metadata 포팅 (HierarchyHandling, Topdown*, EdgeCoords/ShapeCoords 등)
- `LayoutAlgorithmResolver`, `GraphValidator`, `DeprecatedLayoutOptionReplacer` 등 데이터/검증 로직 포팅 및 연결
- `ElkGraphAdapters`/`GraphAdapters` 포팅
- `ElkSpacings`, `IndividualSpacings`, `FixedLayoutProvider` 등 유틸 포팅
- `RecursiveGraphLayoutEngine` 등 방문자/검증 사용처 포팅
- 테스트 확장 및 성능 측정 자동화
