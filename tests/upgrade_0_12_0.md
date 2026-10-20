# ELK 0.12.0 포팅 검증

검증일: 2026-10-20. 작업 기준 커밋은 `d2f4c53ec25b35c2dd7e1d1fd62c2f8664114aca`이며, `port/0.12.0`에서 검증했다. 기존 0.11.0 기준선도 새 Java export와 보관한 Rust 바이너리로 다시 측정했다.

## 고정한 입력과 환경

| 입력 | 버전/커밋 |
|---|---|
| ELK Java | `v0.12.0`, `323312916048544e4e594eab3303cdb0d01e2929` |
| elkjs | `0.12.0`, `ff5771d7165445c42c408bb8a090c8035272218c` |
| elk-models | `ac5af2ba511c0037e4f347e8bd91f013950951c7` (기존과 동일, 원격 HEAD 확인) |
| Rust | 1.97.1, workspace/default features (성능 측정의 allocator는 아래에서 구분) |
| Java/Maven | OpenJDK 21.0.12.1, Maven 3.9.16, `-Ddash.skip=true` |
| JavaScript | Node 24.19.0, elkjs devDependency 정확히 `0.12.0` |
| WASM | wasm-pack 0.13.1, wasm-bindgen 0.2.113, web 및 nodejs target |
| 측정 머신 | MacBook Pro (Mac15,6), Apple M3 Pro, 11코어(5P+6E), RAM 36GB, macOS 27.0.1 (26A434) |

Java 빌드는 버전별 격리 디렉터리를 사용했다. 결정적 self-loop 라우팅 패치는 양쪽 버전에 동일하게 적용했고, 서브모듈 소스는 수정하지 않았다. 원시 로그와 산출물은 로컬 `/tmp/elk-012-validation/` 아래 버전별 디렉터리에 있다. 임시 경로는 영구 저장소가 아니므로 재현 시 아래 절차로 다시 생성한다.

## 반영한 upstream 변경

- [PR #1194](https://github.com/eclipse-elk/elk/pull/1194): `NodePlacementRepeater`, nullable `recomputeNodePlacement` 옵션, P4 뒤 프로세서 연결, 노드 크기를 유지한 포트·라벨·margin 재계산 및 두 번째 배치 전략 실행.
- [PR #1170](https://github.com/eclipse-elk/elk/pull/1170): `AlternatingLayerUnzipper`의 split 크기 경계 조건 `>` → `>=`.
- upstream 조건부 secondary-license notice를 반영했다. SPDX는 `EPL-2.0 OR GPL-3.0-or-later`; 실제 GPL 선택 가능 조건은 동봉한 LICENSE/NOTICE에 따른다. crates.io 18개와 npm 7개 패키지에 필요한 라이선스 문서를 포함한다.
- elkjs 0.12의 VertiFlex 제거 때문에 Java/Rust의 해당 알고리즘을 제거하지 않았다. VertiFlex·DisCo는 실제 Java 비교가 성공한 경우에만 JS 비교기에서 추가 지원으로 분류한다.

## 검증 결과

| 검사 | 0.11.0 재측정 | 0.12.0 |
|---|---:|---:|
| Workspace build / Clippy | warning 0 / error 0 | warning 0 / error 0 |
| Workspace tests | 1,027 passed, 0 failed, 2 ignored | 1,032 passed, 0 failed, 2 ignored |
| Model parity: 전체 입력 | 1,998 | 1,998 |
| Model parity: 비교 / 일치 | 1,989 / 1,988 | 1,989 / 1,988 |
| Model parity: drift / skipped / errors | 1 / 9 / 0 | 1 / 9 / 0 |
| Phase-step trace | 1,997/1,997 match, 0 drift | 1,997/1,997 match, 0 drift |
| Phase-step trace: Rust only | 1 | 1 |
| Java 옵션·알고리즘·연결 정적 검사 | — | 12/12 `ok`, strict 검사 통과 |
| JS corpus: NAPI vs Java | — | 550/550 match |
| JS corpus: web WASM vs Java | — | 550/550 match |
| JS unit/integration tests | — | 49 passed, 0 failed |
| Rust allocation regression | — | 2,007개 입력 및 기존 구조·엔진 재사용 검사 통과, 잔존 할당 증가 0 |
| crates.io publish dry-run | — | 18개 package 및 verification build 통과, upload 없음 |

Model parity의 raw 일치율은 1,988/1,989(99.95%)다. 유일한 drift는 기존 `213_componentsCompaction.elkt`의 Java NaN/compaction 오류이며 새 drift는 없다. 정상 비교 대상으로 제한하면 1,988/1,988이다. Phase trace의 Rust-only 1개는 기존 `588_hierarchicalLayoutNPE.elkt`의 Java NPE다. 제외·skipped를 성공한 비교에 합산하지 않았다.

JS 두 backend는 각각 `PASS=530`, `ELKJS_DRIFT=20`, `javaCompared=550`, `javaMatches=550`, `missingJava=0`, failures=0이다. 실제 0.12.0 Java JSON baseline을 사용했고 `--require-java`를 활성화했다. Rust와 elkjs가 서로 같아도 Java와 다르면 실패하도록 비교기를 수정했으며, 실패한 정확한 `.json` baseline을 이름이 같은 `.elkg` 결과로 대체할 수 없다.

### 신규 기능 입력

`plugins/org.eclipse.elk.alg.layered/tests/resources/node_flexibility/`의 9개 입력을 사용했다.

- 5개 재배치 전략(`BRANDES_KOEPF`, `NETWORK_SIMPLEX`, `SIMPLE`, `INTERACTIVE`, `LINEAR_SEGMENTS`), inherited flexibility, 옵션 생략, 동일 split 크기의 8개 입력: model parity **8/8**, phase trace **8/8**, NAPI/Java **8/8**, web WASM/Java **8/8** 일치.
- `null_recompute.json`: Java Gson importer가 `UnsupportedOperationException: JsonNull`을 내므로 Java layout/trace가 없다. 전체 9개를 대상으로 한 JS `--require-java` 실행은 이 1개 때문에 의도대로 실패한다. 별도 유효 8개 실행을 성공 결과로 기록했다.
- Rust/JS에서 명시적 null이 옵션 생략과 같은 결과를 내는 기존 동작은 회귀 테스트로 확인했다. Java에서 실행되지 않은 입력을 parity 성공으로 집계하지 않는다.
- 단위 테스트는 크기 고정, 포트·라벨·margin 갱신, 옵션 메타데이터, 프로세서 활성화와 split 경계를 확인한다.

### 배포 경로 검증

- 실제 .12 NAPI와 web WASM을 다시 빌드한 뒤 위 비교를 실행했다.
- Node fallback용 CommonJS WASM을 추가했다. 배포에는 `dist/wasm/`과 `dist/wasm-node/`를 모두 포함한다.
- NAPI와 optional dependency가 없는 격리 패키지에서 direct/fakeWorker/worker_threads 세 경로를 확인했다.
- Chrome에서 실제 browser bundle의 direct/worker 레이아웃과 동시 최초 metadata/layout 요청을 확인했다. browser direct의 누락된 WASM 초기화는 공유 Promise로 수정했다.
- `actionlint` 및 ShellCheck를 사용한 4개 workflow 검사 통과. npm dry-run은 모든 패키지의 manifest/라이선스 및 두 WASM target을 확인했다. 비호스트 NAPI 5개는 원격 release rehearsal에서 검증한다.
- 코드 리뷰에서 발견한 잘못된 Java `.elkg` alias fallback을 수정하고 회귀 테스트로 확인했다. 추가 확정 결함은 없다.
- `cargo fmt --all -- --check`는 기존 저장소 전반의 포맷 차이로 실패한다(예: 변경하지 않은 `n_edge.rs`, facade `lib.rs`). 이번 신규 Rust 파일은 rustfmt를 적용했다. 무관한 파일의 일괄 포맷 변경은 포함하지 않았다.

## 성능 및 원격 검증

동일 머신에서 0.11.0/0.12.0 native 바이너리의 3회 교대 측정(각 20회, warmup 3), 0.12.0의 6-engine 측정(각 10회, warmup 3)을 진행한다. 원격 CI와 6개 NAPI 플랫폼의 release rehearsal 결과도 완료 후 기록한다. 과거 다른 OS·툴체인의 3.88x 기준선을 이번 측정과 직접 비교하지 않는다.

## 재현 절차

1. `TESTING.md`의 Java 도구를 준비하고 버전별 output/isolation 디렉터리를 비운 새 경로로 지정한다. `scripts/java_model_parity_trace.sh external/elk-models <output>/model_parity_full/java`로 Java baseline을 새로 생성한다. 기존 Java export를 재사용하지 않는다.
2. 원본 `java_manifest.tsv`를 보존하고 `generate_full_trace_manifest.py`로 전체 1,998개 입력 manifest를 별도 생성한다. 정상 model runner와 trace runner는 별도로 실행한다(trace 모드는 layered를 강제한다).
3. `compare_model_parity_layouts.py`로 최종 출력을, `java_model_phase_step_trace.sh`와 `compare_phase_traces.py --batch --report-missing`으로 중간 단계를 비교한다.
4. 신규 fixture root로 같은 절차를 반복하고 null의 Java 예외를 보존한다. JS에서는 명시적 local NAPI/web WASM `--backend`, fresh Java `--java-dir`, `--require-java`를 사용한다(`TESTING.md` § 3.4).
5. 생성된 Java `src-gen`을 포함하는 isolation의 `plugins`를 `JAVA_SOURCES_ROOT`로 지정하고 정적 검사 12개를 실행한다.
6. full manifest와 신규 9개 입력을 합친 `ELK_MEMORY_MANIFEST`로 `cargo test --release -p org-eclipse-elk-graph-json --test memory_allocations -- --nocapture`를 실행한다.
7. 성능은 다른 빌드·테스트가 멈춘 뒤 실행한다. `sh scripts/run_perf_benchmark.sh synthetic 10 3 <output>` 결과에 6개 엔진 × 26개 시나리오가 있는지 확인한다.
