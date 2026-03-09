# 08. Layout Option 인덱스

## 8.1 source of truth

layout option/algorithm/category metadata의 canonical source는 다음이다.

- `org.eclipse.elk.core::LayoutMetaDataService`

즉:

- 별도 `data/layout_options.json`를 runtime source of truth로 두지 않는다.
- build.rs에서 metadata 코드를 다시 생성하지 않는다.

## 8.2 왜 별도 DB를 만들지 않는가

기존 workspace에는 이미 다음이 있다.

- metadata registry
- option/algorithm lookup
- content assist helper
- JSON metadata export API

중복 DB를 만들면 다음 drift가 생긴다.

1. 새 option 추가 시 두 군데 갱신 필요
2. algorithm-specific known option 목록 불일치
3. default value / target / enum choice drift

## 8.3 `LayoutOptionIndex`

LSP가 빠르게 lookup할 수 있도록 startup 시 얇은 인덱스를 만든다.

```text
LayoutOptionIndex
  ├── options_by_id
  ├── options_by_suffix
  ├── algorithms_by_id
  ├── algorithms_by_suffix
  ├── normalized search index
  └── precomputed value choices
```

이 인덱스는 `LayoutMetaDataService`에서 파생된 캐시일 뿐이다.

## 8.4 제공 기능

### option key completion

- full id: `org.eclipse.elk.algorithm`
- suffix: `algorithm`, `elk.algorithm`, `layered.spacing.nodeNodeBetweenLayers`

### algorithm completion

- `layered`
- `mrtree`
- full id fallback

### value completion

- boolean: `true`, `false`
- enum / enum-set choice
- 숫자/문자열 타입의 default value

## 8.5 algorithm-aware filtering

가능하면 현재 문맥의 algorithm을 읽어 후보를 줄인다.

기준:

- 현재 graph/node의 `CoreOptions::ALGORITHM`
- 없으면 전역 후보 사용

재사용 우선순위:

1. `LayoutDataContentAssist`
2. 부족한 부분만 `LayoutOptionIndex` 보완

## 8.6 target-aware filtering

option target은 completion과 validation 모두에서 사용한다.

- root/parent node
- node
- edge
- port
- label

hierarchical node의 경우 `Parents` target 처리까지 고려한다.

## 8.7 snapshot 테스트

runtime source of truth는 service지만, 회귀 테스트를 위해 snapshot은 남길 수 있다.

권장 방식:

```text
layout_api::known_layout_options()
layout_api::known_layout_algorithms()
layout_api::known_layout_categories()
```

이 결과를 golden fixture와 비교해 metadata drift를 검출한다.

이 fixture는 테스트용이며 런타임 의존성이 아니다.

## 8.8 validation 연계

layout option validation은 가능한 한 기존 validator를 사용한다.

- unknown option
- invalid target
- invalid value
- unsupported-by-algorithm

LSP는 validator 결과를 diagnostic으로 투영한다.

## 8.9 테스트 기준

1. suffix lookup
2. full id lookup
3. algorithm completion
4. option value completion
5. target filtering
6. snapshot drift detection
