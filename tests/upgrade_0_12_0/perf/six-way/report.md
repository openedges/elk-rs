# Performance Comparison Report


## Summary

| Engine | Scenarios | Avg ms (mean) | Avg ops/s | vs java |
|--------|-----------|---------------|-----------|---------|
| java | 26 | 93.7806 | 450 | 1.00x |
| rust_native | 26 | 22.1288 | 1150 | 4.24x |
| rust_api | 26 | 23.4824 | 759 | 3.99x |
| napi | 26 | 24.5033 | 714 | 3.83x |
| wasm | 26 | 33.1527 | 403 | 2.83x |
| elkjs | 26 | 957.4159 | 56 | 0.10x |

## Category Summary

| Category | java_ms | rust_native_ms | rust_api_ms | napi_ms | wasm_ms | elkjs_ms |
|----------| -------- | -------------- | ----------- | -------- | -------- | -------- |
| Size Scaling | 97.8684 | 56.9636 | 59.8547 | 64.0868 | 86.3375 | 686.6333 |
| Algorithms | 7.7684 | 1.3471 | 1.4849 | 1.5755 | 2.2347 | 76.0114 |
| Algorithm Scaling | 199.2854 | 32.9474 | 35.0266 | 35.7396 | 48.2143 | 2163.9901 |
| Edge Routing | 2.9318 | 2.1659 | 2.5241 | 2.8584 | 4.3564 | 23.3849 |
| Crossing Min | 1.9163 | 1.6286 | 2.0056 | 2.3275 | 3.4500 | 18.9086 |
| Hierarchy | 1.2489 | 0.7648 | 0.9247 | 1.1172 | 1.6678 | 9.1746 |

## Per-Scenario Detail

| Scenario | java_ms | rust_native_ms | rust_api_ms | napi_ms | wasm_ms | elkjs_ms |
|----------| -------- | -------------- | ----------- | -------- | -------- | -------- |
| **Size Scaling** |  |  |  |  |  |
| layered_large | 31.8236 | 11.8830 | 12.9512 | 14.8576 | 21.8393 | 114.7343 |
| layered_medium | 10.5172 | 2.9975 | 3.4091 | 2.9328 | 9.2492 | 25.1441 |
| layered_small | 2.9149 | 0.7468 | 0.9539 | 0.4391 | 1.0588 | 6.7814 |
| layered_xlarge | 346.2176 | 212.2269 | 222.1047 | 238.1174 | 313.2027 | 2599.8732 |
| **Algorithms** |  |  |  |  |  |
| force_medium | 17.5794 | 2.6274 | 2.8580 | 2.9349 | 4.0690 | 179.3701 |
| mrtree_medium | 1.0648 | 0.1484 | 0.3096 | 0.3614 | 0.6339 | 6.6375 |
| radial_medium | 0.4105 | 0.7759 | 0.9382 | 0.9817 | 1.3570 | 4.9277 |
| rectpacking_medium | 0.5225 | 0.1156 | 0.1796 | 0.2083 | 0.4321 | 4.9760 |
| stress_medium | 19.2647 | 3.0682 | 3.1389 | 3.3912 | 4.6815 | 184.1457 |
| **Algorithm Scaling** |  |  |  |  |  |
| force_large | 72.7252 | 11.4982 | 12.3424 | 12.7395 | 18.2485 | 774.1487 |
| force_xlarge | 868.1665 | 132.2057 | 133.1837 | 135.3009 | 179.7301 | 9563.1902 |
| mrtree_large | 3.8033 | 0.5968 | 1.2297 | 1.4442 | 2.2199 | 19.4782 |
| mrtree_xlarge | 25.1631 | 3.2466 | 6.5414 | 7.6935 | 10.7871 | 139.9926 |
| radial_large | 1.2735 | 2.2418 | 2.8847 | 3.0577 | 5.0666 | 18.2568 |
| radial_xlarge | 12.7628 | 9.9458 | 13.2859 | 14.1722 | 24.5686 | 243.3942 |
| rectpacking_large | 1.7070 | 0.4731 | 0.7209 | 0.8274 | 1.4668 | 12.1265 |
| rectpacking_xlarge | 6.1021 | 2.9862 | 4.3175 | 5.1045 | 8.4226 | 65.6556 |
| stress_large | 74.3454 | 12.9861 | 13.4597 | 14.5351 | 19.3079 | 775.9472 |
| stress_xlarge | 926.8051 | 153.2938 | 162.3002 | 162.5210 | 212.3251 | 10027.7113 |
| **Edge Routing** |  |  |  |  |  |
| routing_orthogonal | 2.5850 | 2.1089 | 2.4920 | 2.8129 | 4.2971 | 23.8455 |
| routing_polyline | 2.4649 | 1.9473 | 2.2568 | 2.5503 | 3.9195 | 21.5475 |
| routing_splines | 3.7456 | 2.4415 | 2.8236 | 3.2120 | 4.8525 | 24.7616 |
| **Crossing Min** |  |  |  |  |  |
| crossmin_layer_sweep | 2.6234 | 2.1315 | 2.5046 | 2.8322 | 4.3180 | 23.2170 |
| crossmin_none | 1.2091 | 1.1257 | 1.5067 | 1.8227 | 2.5821 | 14.6002 |
| **Hierarchy** |  |  |  |  |  |
| hierarchy_flat | 1.3205 | 0.9865 | 1.1842 | 1.3832 | 2.0946 | 11.2018 |
| hierarchy_nested | 1.1774 | 0.5431 | 0.6652 | 0.8511 | 1.2410 | 7.1474 |

## Per-Scenario Throughput (ops/s)

| Scenario | java | rust_native | rust_api | napi | wasm | elkjs |
|----------| -------- | ----------- | -------- | -------- | -------- | -------- |
| **Size Scaling** |  |  |  |  |  |
| layered_large | 31 | 84 | 77 | 67 | 46 | 9 |
| layered_medium | 95 | 334 | 293 | 341 | 108 | 40 |
| layered_small | 343 | 1339 | 1048 | 2277 | 944 | 147 |
| layered_xlarge | 3 | 5 | 4 | 4 | 3 | 0 |
| **Algorithms** |  |  |  |  |  |
| force_medium | 57 | 381 | 350 | 341 | 246 | 6 |
| mrtree_medium | 939 | 6740 | 3230 | 2767 | 1577 | 151 |
| radial_medium | 2436 | 1289 | 1066 | 1019 | 737 | 203 |
| rectpacking_medium | 1914 | 8648 | 5569 | 4800 | 2314 | 201 |
| stress_medium | 52 | 326 | 319 | 295 | 214 | 5 |
| **Algorithm Scaling** |  |  |  |  |  |
| force_large | 14 | 87 | 81 | 78 | 55 | 1 |
| force_xlarge | 1 | 8 | 8 | 7 | 6 | 0 |
| mrtree_large | 263 | 1676 | 813 | 692 | 450 | 51 |
| mrtree_xlarge | 40 | 308 | 153 | 130 | 93 | 7 |
| radial_large | 785 | 446 | 347 | 327 | 197 | 55 |
| radial_xlarge | 78 | 101 | 75 | 71 | 41 | 4 |
| rectpacking_large | 586 | 2114 | 1387 | 1209 | 682 | 82 |
| rectpacking_xlarge | 164 | 335 | 232 | 196 | 119 | 15 |
| stress_large | 13 | 77 | 74 | 69 | 52 | 1 |
| stress_xlarge | 1 | 7 | 6 | 6 | 5 | 0 |
| **Edge Routing** |  |  |  |  |  |
| routing_orthogonal | 387 | 474 | 401 | 356 | 233 | 42 |
| routing_polyline | 406 | 514 | 443 | 392 | 255 | 46 |
| routing_splines | 267 | 410 | 354 | 311 | 206 | 40 |
| **Crossing Min** |  |  |  |  |  |
| crossmin_layer_sweep | 381 | 469 | 399 | 353 | 232 | 43 |
| crossmin_none | 827 | 888 | 664 | 549 | 387 | 68 |
| **Hierarchy** |  |  |  |  |  |
| hierarchy_flat | 757 | 1014 | 844 | 723 | 477 | 89 |
| hierarchy_nested | 849 | 1841 | 1503 | 1175 | 806 | 140 |

## Notes

- **elkjs** is async (Promise-based); results include Promise overhead.
- **NAPI/WASM** are synchronous; WASM has one-time module init cost (absorbed in warmup).
- **rust_api** measures the same code path as NAPI/WASM (JSON parse + layout + JSON serialize).
- **rust_native** uses direct `ElkNode` construction (no JSON overhead).
- Speedup ratios (vs java) show how many times faster each engine is.
