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
- `LayoutAlgorithmResolver`, `GraphValidator`, `DeprecatedLayoutOptionReplacer`, `ElkGraphAdapters` 포팅 및 테스트 추가
- `ElkSpacings`, `IndividualSpacings`, `FixedLayoutProvider`, `RecursiveGraphLayoutEngine` 포팅 및 테스트 추가
- 레이아웃 구성 저장소/매니저 root 처리 보강 및 테스트 추가
- layered 메타데이터 보강(지원 기능/기본값) 및 테스트 추가
- layered considerModelOrder/groupModelOrder 옵션/메타데이터, ordering 전략 enum, 컴포넌트 ordering 전략, 테스트 추가
- layered LGraph 모델(LGraph/Layer/LNode/LPort/LEdge/LLabel/LMargin/LPadding) 및 최소 LGraphUtil/옵션(PortType, InteractiveReferencePoint, InternalProperties) 포팅
- layered Tarjan SCC 유틸 포팅(InternalProperties 확장 포함)
- label manager 옵션 분리(Core vs Labels) 및 메타데이터/테스트 추가
- `PropertyConstantsDelegator` 옵션 타입 확장(label manager, topdown size approximator, layout algorithm data)
- 테스트 추가/확장 (`layout_algorithm_metadata`, `label_management_options`, `deprecated_layout_option_replacer`, `elk_graph_adapters` 등), `cargo clippy --workspace --all-targets` 통과

## 다음 작업
- layered 알고리즘 본체 포팅(옵션/메타데이터, LGraph 모델, 변환기/프로세서)
- 다른 알고리즘 모듈 포팅 및 연동(alg.*)
- 알고리즘 테스트 이식 확대(특히 layered 테스트군)
- 성능 측정 자동화(벤치/프로파일링 스크립트)
