/**
 * Synthetic benchmark scenarios as ELK JSON graph objects.
 *
 * These match the 5 scenarios from perf_layered_issue_scenarios.rs and
 * LayeredIssueParityBenchTest.java. Each function returns a plain JS object
 * suitable for elkjs layout() or JSON.stringify() for NAPI/WASM layout_json().
 */

export function buildIssue405() {
  return {
    id: "root",
    layoutOptions: {
      "org.eclipse.elk.algorithm": "org.eclipse.elk.layered",
      "org.eclipse.elk.direction": "RIGHT",
      "org.eclipse.elk.edgeRouting": "ORTHOGONAL"
    },
    children: [
      {
        id: "reference",
        width: 80,
        height: 60,
        layoutOptions: {
          "org.eclipse.elk.portConstraints": "FIXED_SIDE",
          "org.eclipse.elk.portLabels.placement": "OUTSIDE NEXT_TO_PORT_IF_POSSIBLE"
        },
        ports: [
          {
            id: "west", width: 10, height: 10,
            layoutOptions: { "org.eclipse.elk.port.side": "WEST" },
            labels: [{ text: "west", width: 20, height: 10 }]
          },
          {
            id: "east", width: 10, height: 10,
            layoutOptions: { "org.eclipse.elk.port.side": "EAST" },
            labels: [{ text: "east", width: 20, height: 10 }]
          },
          {
            id: "north", width: 10, height: 10,
            layoutOptions: { "org.eclipse.elk.port.side": "NORTH" },
            labels: [{ text: "north", width: 20, height: 10 }]
          },
          {
            id: "south", width: 10, height: 10,
            layoutOptions: { "org.eclipse.elk.port.side": "SOUTH" },
            labels: [{ text: "south", width: 20, height: 10 }]
          }
        ]
      },
      { id: "westPartner", width: 30, height: 20 },
      { id: "eastPartner", width: 30, height: 20 },
      { id: "northPartner", width: 30, height: 20 },
      { id: "southPartner", width: 30, height: 20 }
    ],
    edges: [
      { id: "e_west", sources: ["westPartner"], targets: ["west"] },
      { id: "e_east", sources: ["east"], targets: ["eastPartner"] },
      { id: "e_north", sources: ["north"], targets: ["northPartner"] },
      { id: "e_south", sources: ["southPartner"], targets: ["south"] }
    ]
  };
}

export function buildIssue603() {
  return {
    id: "root",
    layoutOptions: {
      "org.eclipse.elk.algorithm": "org.eclipse.elk.layered",
      "org.eclipse.elk.nodeLabels.padding": "[top=24.0,left=0.0,bottom=0.0,right=0.0]"
    },
    children: [
      {
        id: "compound",
        width: 120,
        height: 80,
        layoutOptions: {
          "org.eclipse.elk.nodeLabels.placement": "INSIDE V_TOP H_CENTER",
          "org.eclipse.elk.nodeLabels.padding": "[top=24.0,left=0.0,bottom=0.0,right=0.0]"
        },
        labels: [{ text: "compound", width: 40, height: 16 }],
        children: [
          { id: "childA", width: 30, height: 30 },
          { id: "childB", width: 30, height: 30 }
        ],
        edges: [
          { id: "e1", sources: ["childA"], targets: ["childB"] }
        ]
      }
    ]
  };
}

export function buildIssue680() {
  return {
    id: "root",
    layoutOptions: {
      "org.eclipse.elk.algorithm": "org.eclipse.elk.layered",
      "org.eclipse.elk.direction": "DOWN",
      "org.eclipse.elk.edgeRouting": "ORTHOGONAL"
    },
    children: [
      {
        id: "parent",
        width: 180,
        height: 110,
        ports: [
          {
            id: "p1", width: 10, height: 10,
            layoutOptions: {
              "org.eclipse.elk.port.side": "WEST",
              "org.eclipse.elk.port.borderOffset": -20
            }
          },
          {
            id: "p2", width: 10, height: 10,
            layoutOptions: {
              "org.eclipse.elk.port.side": "EAST",
              "org.eclipse.elk.port.borderOffset": -22
            }
          }
        ],
        children: [
          {
            id: "child",
            width: 100,
            height: 60,
            ports: [
              {
                id: "c1", width: 10, height: 10,
                layoutOptions: {
                  "org.eclipse.elk.port.side": "WEST",
                  "org.eclipse.elk.port.borderOffset": -8
                }
              },
              {
                id: "c2", width: 10, height: 10,
                layoutOptions: {
                  "org.eclipse.elk.port.side": "EAST",
                  "org.eclipse.elk.port.borderOffset": -8
                }
              }
            ]
          }
        ],
        edges: [
          { id: "e1", sources: ["p1"], targets: ["c1"] },
          { id: "e2", sources: ["c2"], targets: ["p2"] }
        ]
      }
    ]
  };
}

export function buildIssue871() {
  return {
    id: "root",
    layoutOptions: {
      "org.eclipse.elk.algorithm": "org.eclipse.elk.layered",
      "org.eclipse.elk.direction": "RIGHT",
      "org.eclipse.elk.layered.cycleBreaking.strategy": "MODEL_ORDER",
      "org.eclipse.elk.layered.considerModelOrder.strategy": "PREFER_EDGES",
      "org.eclipse.elk.layered.crossingMinimization.strategy": "NONE",
      "org.eclipse.elk.layered.crossingMinimization.greedySwitch.type": "OFF",
      "org.eclipse.elk.layered.feedbackEdges": true
    },
    children: [
      { id: "n1", width: 30, height: 30 },
      { id: "n2", width: 30, height: 30 },
      { id: "n3", width: 30, height: 30 },
      { id: "n4", width: 30, height: 30 }
    ],
    edges: [
      { id: "e1", sources: ["n1"], targets: ["n2"] },
      { id: "e2", sources: ["n1"], targets: ["n3"] },
      { id: "e3", sources: ["n2"], targets: ["n4"] },
      { id: "e4", sources: ["n4"], targets: ["n3"] }
    ]
  };
}

export function buildIssue905() {
  return {
    id: "root",
    layoutOptions: {
      "org.eclipse.elk.algorithm": "org.eclipse.elk.layered",
      "org.eclipse.elk.direction": "RIGHT"
    },
    children: [
      { id: "source", width: 30, height: 30 },
      { id: "target", width: 30, height: 30 }
    ],
    edges: [
      {
        id: "e1",
        sources: ["source"],
        targets: ["target"],
        labels: [
          {
            text: "tail", width: 16, height: 10, x: 5, y: 10,
            layoutOptions: { "org.eclipse.elk.edgeLabels.placement": "TAIL" }
          },
          {
            text: "center", width: 20, height: 10, x: 20, y: 80,
            layoutOptions: { "org.eclipse.elk.edgeLabels.placement": "CENTER" }
          },
          {
            text: "head", width: 16, height: 10, x: 35, y: 150,
            layoutOptions: { "org.eclipse.elk.edgeLabels.placement": "HEAD" }
          }
        ]
      }
    ]
  };
}

// ---------------------------------------------------------------------------
// Deterministic LCG-based pseudo-random number generator
// Same constants as glibc/Java's LCG for cross-implementation compatibility.
// ---------------------------------------------------------------------------

function lcg(state) { return ((state * 1103515245 + 12345) & 0x7fffffff); }

// ---------------------------------------------------------------------------
// Graph generators
// ---------------------------------------------------------------------------

/**
 * Generate a deterministic DAG with `nodes` nodes and up to `edges` edges.
 * Nodes are partitioned into layers (layer = floor(i * 5 / nodes)) to
 * guarantee all edges point strictly forward (layerIdx[src] < layerIdx[tgt]).
 */
function generateDag(nodes, edges, seed) {
  const children = [];
  const layerIdx = [];
  const maxLayer = 4; // floor((nodes-1) * 5 / nodes) <= 4

  for (let i = 0; i < nodes; i++) {
    layerIdx.push(Math.floor(i * 5 / nodes));
    children.push({ id: `n${i}`, width: 40, height: 30 });
  }

  // Build per-layer node lists for fast lookup
  const layers = [];
  for (let l = 0; l <= maxLayer; l++) layers.push([]);
  for (let i = 0; i < nodes; i++) layers[layerIdx[i]].push(i);

  const edgeList = [];
  let state = seed;
  let attempts = 0;
  const maxAttempts = edges * 8;

  while (edgeList.length < edges && attempts < maxAttempts) {
    state = lcg(state);
    const src = state % nodes;
    attempts++;

    // Nodes in the last layer have no forward targets — skip them
    if (layerIdx[src] >= maxLayer) continue;

    state = lcg(state);
    const span = maxLayer - layerIdx[src]; // >= 1
    const tgtLayer = layerIdx[src] + 1 + (state % span);

    const tgtCandidates = layers[tgtLayer];
    if (!tgtCandidates || tgtCandidates.length === 0) continue;

    state = lcg(state);
    const tgt = tgtCandidates[state % tgtCandidates.length];

    // Deduplicate edges (simple check — good enough for bench graphs)
    const eid = `e${src}_${tgt}`;
    if (!edgeList.find(e => e.id === eid)) {
      edgeList.push({ id: eid, sources: [`n${src}`], targets: [`n${tgt}`] });
    }
  }

  return {
    id: "root",
    layoutOptions: {
      "org.eclipse.elk.algorithm": "org.eclipse.elk.layered",
      "org.eclipse.elk.direction": "RIGHT",
      "org.eclipse.elk.edgeRouting": "ORTHOGONAL"
    },
    children,
    edges: edgeList
  };
}

/**
 * Generate a deterministic tree with `nodes` nodes.
 * Each node i > 0 has exactly one parent: parent = lcgState % i (< i).
 */
function generateTree(nodes, seed) {
  const children = [];
  const edgeList = [];
  let state = seed;

  for (let i = 0; i < nodes; i++) {
    children.push({ id: `n${i}`, width: 40, height: 30 });
    if (i > 0) {
      state = lcg(state);
      const parent = state % i;
      edgeList.push({ id: `e${parent}_${i}`, sources: [`n${parent}`], targets: [`n${i}`] });
    }
  }

  return {
    id: "root",
    layoutOptions: {
      "org.eclipse.elk.algorithm": "org.eclipse.elk.layered",
      "org.eclipse.elk.direction": "RIGHT",
      "org.eclipse.elk.edgeRouting": "ORTHOGONAL"
    },
    children,
    edges: edgeList
  };
}

/**
 * Generate a general graph (cycles allowed) with `nodes` nodes and `edges` edges.
 * src and tgt are chosen freely from [0, nodes).
 */
function generateGeneralGraph(nodes, edges, seed) {
  const children = [];
  for (let i = 0; i < nodes; i++) {
    children.push({ id: `n${i}`, width: 40, height: 30 });
  }

  const edgeList = [];
  let state = seed;

  for (let e = 0; e < edges; e++) {
    state = lcg(state);
    const src = state % nodes;
    state = lcg(state);
    const tgt = state % nodes;
    edgeList.push({ id: `e${e}`, sources: [`n${src}`], targets: [`n${tgt}`] });
  }

  return {
    id: "root",
    layoutOptions: {
      "org.eclipse.elk.algorithm": "org.eclipse.elk.layered",
      "org.eclipse.elk.direction": "RIGHT",
      "org.eclipse.elk.edgeRouting": "ORTHOGONAL"
    },
    children,
    edges: edgeList
  };
}

// ---------------------------------------------------------------------------
// Axis 1 — Size Scaling (Layered)
// ---------------------------------------------------------------------------

export function buildLayeredSmall() {
  return generateDag(10, 15, 42);
}

export function buildLayeredMedium() {
  return generateDag(50, 100, 42);
}

export function buildLayeredLarge() {
  return generateDag(200, 500, 42);
}

export function buildLayeredXlarge() {
  return generateDag(1000, 3000, 42);
}

// ---------------------------------------------------------------------------
// Axis 2 — Algorithm Diversity (50 nodes each)
// ---------------------------------------------------------------------------

export function buildForceMedium() {
  const g = generateGeneralGraph(50, 80, 100);
  g.layoutOptions["org.eclipse.elk.algorithm"] = "org.eclipse.elk.force";
  delete g.layoutOptions["org.eclipse.elk.direction"];
  delete g.layoutOptions["org.eclipse.elk.edgeRouting"];
  return g;
}

export function buildStressMedium() {
  const g = generateGeneralGraph(50, 80, 100);
  g.layoutOptions["org.eclipse.elk.algorithm"] = "org.eclipse.elk.stress";
  delete g.layoutOptions["org.eclipse.elk.direction"];
  delete g.layoutOptions["org.eclipse.elk.edgeRouting"];
  return g;
}

export function buildMrtreeMedium() {
  const g = generateTree(50, 200);
  g.layoutOptions["org.eclipse.elk.algorithm"] = "org.eclipse.elk.mrtree";
  delete g.layoutOptions["org.eclipse.elk.direction"];
  delete g.layoutOptions["org.eclipse.elk.edgeRouting"];
  return g;
}

export function buildRadialMedium() {
  const g = generateTree(50, 200);
  g.layoutOptions["org.eclipse.elk.algorithm"] = "org.eclipse.elk.radial";
  delete g.layoutOptions["org.eclipse.elk.direction"];
  delete g.layoutOptions["org.eclipse.elk.edgeRouting"];
  return g;
}

export function buildRectpackingMedium() {
  const children = [];
  let state = 999;
  for (let i = 0; i < 50; i++) {
    state = lcg(state);
    const w = 20 + (state % 61); // range [20, 80]
    state = lcg(state);
    const h = 20 + (state % 61); // range [20, 80]
    children.push({ id: `n${i}`, width: w, height: h });
  }
  return {
    id: "root",
    layoutOptions: {
      "org.eclipse.elk.algorithm": "org.eclipse.elk.rectpacking"
    },
    children,
    edges: []
  };
}

// ---------------------------------------------------------------------------
// Axis 2b — Algorithm Scaling (large / xlarge)
// ---------------------------------------------------------------------------

export function buildForceLarge() {
  const g = generateGeneralGraph(200, 400, 100);
  g.layoutOptions["org.eclipse.elk.algorithm"] = "org.eclipse.elk.force";
  delete g.layoutOptions["org.eclipse.elk.direction"];
  delete g.layoutOptions["org.eclipse.elk.edgeRouting"];
  return g;
}

export function buildForceXlarge() {
  const g = generateGeneralGraph(500, 1200, 100);
  g.layoutOptions["org.eclipse.elk.algorithm"] = "org.eclipse.elk.force";
  delete g.layoutOptions["org.eclipse.elk.direction"];
  delete g.layoutOptions["org.eclipse.elk.edgeRouting"];
  return g;
}

export function buildStressLarge() {
  const g = generateGeneralGraph(200, 400, 100);
  g.layoutOptions["org.eclipse.elk.algorithm"] = "org.eclipse.elk.stress";
  delete g.layoutOptions["org.eclipse.elk.direction"];
  delete g.layoutOptions["org.eclipse.elk.edgeRouting"];
  return g;
}

export function buildStressXlarge() {
  const g = generateGeneralGraph(500, 1200, 100);
  g.layoutOptions["org.eclipse.elk.algorithm"] = "org.eclipse.elk.stress";
  delete g.layoutOptions["org.eclipse.elk.direction"];
  delete g.layoutOptions["org.eclipse.elk.edgeRouting"];
  return g;
}

export function buildMrtreeLarge() {
  const g = generateTree(200, 200);
  g.layoutOptions["org.eclipse.elk.algorithm"] = "org.eclipse.elk.mrtree";
  delete g.layoutOptions["org.eclipse.elk.direction"];
  delete g.layoutOptions["org.eclipse.elk.edgeRouting"];
  return g;
}

export function buildMrtreeXlarge() {
  const g = generateTree(1000, 200);
  g.layoutOptions["org.eclipse.elk.algorithm"] = "org.eclipse.elk.mrtree";
  delete g.layoutOptions["org.eclipse.elk.direction"];
  delete g.layoutOptions["org.eclipse.elk.edgeRouting"];
  return g;
}

export function buildRadialLarge() {
  const g = generateTree(200, 200);
  g.layoutOptions["org.eclipse.elk.algorithm"] = "org.eclipse.elk.radial";
  delete g.layoutOptions["org.eclipse.elk.direction"];
  delete g.layoutOptions["org.eclipse.elk.edgeRouting"];
  return g;
}

export function buildRadialXlarge() {
  const g = generateTree(1000, 200);
  g.layoutOptions["org.eclipse.elk.algorithm"] = "org.eclipse.elk.radial";
  delete g.layoutOptions["org.eclipse.elk.direction"];
  delete g.layoutOptions["org.eclipse.elk.edgeRouting"];
  return g;
}

export function buildRectpackingLarge() {
  const children = [];
  let state = 100;
  for (let i = 0; i < 200; i++) {
    state = lcg(state);
    const w = 20 + (state % 61);
    state = lcg(state);
    const h = 20 + (state % 61);
    children.push({ id: `n${i}`, width: w, height: h });
  }
  return {
    id: "root",
    layoutOptions: {
      "org.eclipse.elk.algorithm": "org.eclipse.elk.rectpacking"
    },
    children,
    edges: []
  };
}

export function buildRectpackingXlarge() {
  const children = [];
  let state = 100;
  for (let i = 0; i < 1000; i++) {
    state = lcg(state);
    const w = 20 + (state % 61);
    state = lcg(state);
    const h = 20 + (state % 61);
    children.push({ id: `n${i}`, width: w, height: h });
  }
  return {
    id: "root",
    layoutOptions: {
      "org.eclipse.elk.algorithm": "org.eclipse.elk.rectpacking"
    },
    children,
    edges: []
  };
}

// ---------------------------------------------------------------------------
// Axis 3 — Edge Routing (50-node DAG, vary edgeRouting)
// ---------------------------------------------------------------------------

export function buildRoutingPolyline() {
  const g = generateDag(50, 100, 42);
  g.layoutOptions["org.eclipse.elk.edgeRouting"] = "POLYLINE";
  return g;
}

export function buildRoutingOrthogonal() {
  const g = generateDag(50, 100, 42);
  g.layoutOptions["org.eclipse.elk.edgeRouting"] = "ORTHOGONAL";
  return g;
}

export function buildRoutingSplines() {
  const g = generateDag(50, 100, 42);
  g.layoutOptions["org.eclipse.elk.edgeRouting"] = "SPLINES";
  return g;
}

// ---------------------------------------------------------------------------
// Axis 4 — Crossing Minimization
// ---------------------------------------------------------------------------

export function buildCrossminLayerSweep() {
  const g = generateDag(50, 100, 42);
  g.layoutOptions["org.eclipse.elk.layered.crossingMinimization.strategy"] = "LAYER_SWEEP";
  g.layoutOptions["org.eclipse.elk.layered.crossingMinimization.greedySwitch.type"] = "TWO_SIDED";
  return g;
}

export function buildCrossminNone() {
  const g = generateDag(50, 100, 42);
  g.layoutOptions["org.eclipse.elk.layered.crossingMinimization.strategy"] = "NONE";
  g.layoutOptions["org.eclipse.elk.layered.crossingMinimization.greedySwitch.type"] = "OFF";
  return g;
}

// ---------------------------------------------------------------------------
// Axis 5 — Hierarchy
// ---------------------------------------------------------------------------

export function buildHierarchyFlat() {
  return generateDag(30, 50, 300);
}

export function buildHierarchyNested() {
  // 3-level nested graph: root -> 3 compounds -> ~9 leaves each (~30 total nodes)
  // Uses LCG seed 300 for determinism.
  let state = 300;

  const compounds = [];
  const rootEdges = [];

  for (let m = 0; m < 3; m++) {
    const leaves = [];
    const compoundEdges = [];

    // 9 leaf children per compound
    for (let l = 0; l < 9; l++) {
      leaves.push({ id: `mid${m}_leaf${l}`, width: 40, height: 30 });
    }

    // ~4 edges within each compound (leaf-to-leaf, forward only)
    for (let e = 0; e < 4; e++) {
      state = lcg(state);
      const src = state % 8; // [0,7]
      state = lcg(state);
      const tgt = src + 1 + (state % (8 - src)); // tgt > src
      compoundEdges.push({
        id: `mid${m}_ie${e}`,
        sources: [`mid${m}_leaf${src}`],
        targets: [`mid${m}_leaf${tgt}`]
      });
    }

    compounds.push({
      id: `mid${m}`,
      layoutOptions: {
        "org.eclipse.elk.algorithm": "org.eclipse.elk.layered",
        "org.eclipse.elk.direction": "RIGHT"
      },
      children: leaves,
      edges: compoundEdges
    });
  }

  // Cross-compound edges: mid0->mid1, mid1->mid2
  rootEdges.push({ id: "re0", sources: ["mid0"], targets: ["mid1"] });
  rootEdges.push({ id: "re1", sources: ["mid1"], targets: ["mid2"] });

  return {
    id: "root",
    layoutOptions: {
      "org.eclipse.elk.algorithm": "org.eclipse.elk.layered",
      "org.eclipse.elk.direction": "RIGHT"
    },
    children: compounds,
    edges: rootEdges
  };
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/** Return all synthetic scenarios as [name, graph] pairs. */
export function allScenarios() {
  return [
    // Size scaling
    ["layered_small", buildLayeredSmall()],
    ["layered_medium", buildLayeredMedium()],
    ["layered_large", buildLayeredLarge()],
    ["layered_xlarge", buildLayeredXlarge()],
    // Algorithm diversity
    ["force_medium", buildForceMedium()],
    ["stress_medium", buildStressMedium()],
    ["mrtree_medium", buildMrtreeMedium()],
    ["radial_medium", buildRadialMedium()],
    ["rectpacking_medium", buildRectpackingMedium()],
    // Algorithm scaling
    ["force_large", buildForceLarge()],
    ["force_xlarge", buildForceXlarge()],
    ["stress_large", buildStressLarge()],
    ["stress_xlarge", buildStressXlarge()],
    ["mrtree_large", buildMrtreeLarge()],
    ["mrtree_xlarge", buildMrtreeXlarge()],
    ["radial_large", buildRadialLarge()],
    ["radial_xlarge", buildRadialXlarge()],
    ["rectpacking_large", buildRectpackingLarge()],
    ["rectpacking_xlarge", buildRectpackingXlarge()],
    // Edge routing
    ["routing_polyline", buildRoutingPolyline()],
    ["routing_orthogonal", buildRoutingOrthogonal()],
    ["routing_splines", buildRoutingSplines()],
    // Crossing minimization
    ["crossmin_layer_sweep", buildCrossminLayerSweep()],
    ["crossmin_none", buildCrossminNone()],
    // Hierarchy
    ["hierarchy_flat", buildHierarchyFlat()],
    ["hierarchy_nested", buildHierarchyNested()],
  ];
}
