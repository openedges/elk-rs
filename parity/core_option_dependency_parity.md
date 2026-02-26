# Core Option Dependency Parity

- status: drift
- java dependencies: 10
- rust dependencies: 8
- java-only dependencies: 2
- rust-only dependencies: 0
- value mismatches: 0

## Java-Only Dependencies
- `org.eclipse.elk.topdown.sizeApproximator` -> `org.eclipse.elk.topdown.nodeType` (value=TopdownNodeTypes::HierarchicalNode)
- `org.eclipse.elk.topdown.sizeCategories` -> `org.eclipse.elk.topdown.sizeApproximator` (value=TopdownSizeApproximator::FixedIntegerRatioBoxes)

## Rust-Only Dependencies
- none

## Value Mismatches
- none
