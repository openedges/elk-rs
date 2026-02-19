use std::cell::RefCell;
use std::rc::Rc;

use serde_json::Value;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::plain_java_initialization::initialize_plain_java_layout;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{BasicProgressMonitor, Maybe};
use org_eclipse_elk_core::org::eclipse::elk::core::{
    IGraphLayoutEngine, RecursiveGraphLayoutEngine,
};
use org_eclipse_elk_graph_json::org::eclipse::elk::graph::json::ElkGraphJson;

const GRAPH: &str = r#"    {
        "id": "root",
        "properties": {
            "algorithm": "layered",
            "org.eclipse.elk.hierarchyHandling": "INCLUDE_CHILDREN"
        },
        "children": [
            { "id": "A",
                "children": [
                    { "id": "x", "width": 50, "height": 90 },
                    { "id": "B",
                        "labels": [ { "text": "B", "width": 10, "height": 12 } ],
                        "ports": [
                            { "id": "p", "width": 10, "height": 10,
                                "labels": [ { "text": "p", "width": 10, "height": 12 } ]
                            }
                        ],
                        "children": [
                            { "id": "y", "width": 50, "height": 90 },
                            { "id": "z", "width": 50, "height": 90 }
                        ],
                        "edges": [
                            { "id": "e1", "sources": [ "y" ], "targets": [ "z" ] },
                            { "id": "e2", "sources": [ "x" ], "targets": [ "z" ],
                                "labels": [ { "text": "e2", "width": 20, "height": 12 } ]
                            },
                            { "id": "e3", "sources": [ "x" ], "targets": [ "p" ] },
                            { "id": "e4", "sources": [ "p" ], "targets": [ "y" ] }
                        ]
                    }
                ]
            }
        ]
    }"#;

const OUTPUT_PC: &str = r#"{
  "children": [
    {
      "children": [
        {
          "height": 90,
          "id": "x",
          "width": 50,
          "x": 12,
          "y": 24
        },
        {
          "children": [
            {
              "height": 90,
              "id": "y",
              "width": 50,
              "x": 12,
              "y": 27
            },
            {
              "height": 90,
              "id": "z",
              "width": 50,
              "x": 82,
              "y": 12
            }
          ],
          "edges": [
            {
              "container": "B",
              "id": "e1",
              "sections": [
                {
                  "endPoint": {
                    "x": 82,
                    "y": 72
                  },
                  "id": "e1_s0",
                  "incomingShape": "y",
                  "outgoingShape": "z",
                  "startPoint": {
                    "x": 62,
                    "y": 72
                  }
                }
              ],
              "sources": [
                "y"
              ],
              "targets": [
                "z"
              ]
            },
            {
              "container": "A",
              "id": "e2",
              "labels": [
                {
                  "height": 12,
                  "text": "e2",
                  "width": 20,
                  "x": 82,
                  "y": 39
                }
              ],
              "sections": [
                {
                  "bendPoints": [
                    {
                      "x": 112,
                      "y": 54
                    },
                    {
                      "x": 112,
                      "y": 28
                    },
                    {
                      "x": 215,
                      "y": 28
                    },
                    {
                      "x": 215,
                      "y": 54
                    }
                  ],
                  "endPoint": {
                    "x": 225,
                    "y": 54
                  },
                  "id": "e2_s0",
                  "incomingShape": "x",
                  "outgoingShape": "z",
                  "startPoint": {
                    "x": 62,
                    "y": 54
                  }
                }
              ],
              "sources": [
                "x"
              ],
              "targets": [
                "z"
              ]
            },
            {
              "container": "A",
              "id": "e3",
              "sections": [
                {
                  "endPoint": {
                    "x": 133,
                    "y": 84
                  },
                  "id": "e3_s0",
                  "incomingShape": "x",
                  "outgoingShape": "p",
                  "startPoint": {
                    "x": 62,
                    "y": 84
                  }
                }
              ],
              "sources": [
                "x"
              ],
              "targets": [
                "p"
              ]
            },
            {
              "container": "B",
              "id": "e4",
              "sections": [
                {
                  "endPoint": {
                    "x": 12,
                    "y": 72
                  },
                  "id": "e4_s0",
                  "incomingShape": "p",
                  "outgoingShape": "y",
                  "startPoint": {
                    "x": -10,
                    "y": 72
                  }
                }
              ],
              "sources": [
                "p"
              ],
              "targets": [
                "y"
              ]
            }
          ],
          "height": 129,
          "id": "B",
          "labels": [
            {
              "height": 12,
              "text": "B",
              "width": 10,
              "x": 0,
              "y": 0
            }
          ],
          "ports": [
            {
              "height": 10,
              "id": "p",
              "labels": [
                {
                  "height": 12,
                  "text": "p",
                  "width": 10,
                  "x": -11,
                  "y": 11
                }
              ],
              "width": 10,
              "x": -10,
              "y": 67
            }
          ],
          "width": 144,
          "x": 143,
          "y": 12
        }
      ],
      "height": 153,
      "id": "A",
      "width": 299,
      "x": 12,
      "y": 12
    }
  ],
  "height": 177,
  "id": "root",
  "properties": {
    "algorithm": "layered",
    "org.eclipse.elk.hierarchyHandling": "INCLUDE_CHILDREN",
    "org.eclipse.elk.json.edgeCoords": "CONTAINER",
    "org.eclipse.elk.json.shapeCoords": "PARENT"
  },
  "width": 323,
  "x": 0,
  "y": 0
}
"#;
const OUTPUT_PP: &str = r#"{
  "children": [
    {
      "children": [
        {
          "height": 90,
          "id": "x",
          "width": 50,
          "x": 12,
          "y": 24
        },
        {
          "children": [
            {
              "height": 90,
              "id": "y",
              "width": 50,
              "x": 12,
              "y": 27
            },
            {
              "height": 90,
              "id": "z",
              "width": 50,
              "x": 82,
              "y": 12
            }
          ],
          "edges": [
            {
              "id": "e1",
              "sections": [
                {
                  "endPoint": {
                    "x": 82,
                    "y": 72
                  },
                  "id": "e1_s0",
                  "incomingShape": "y",
                  "outgoingShape": "z",
                  "startPoint": {
                    "x": 62,
                    "y": 72
                  }
                }
              ],
              "sources": [
                "y"
              ],
              "targets": [
                "z"
              ]
            },
            {
              "id": "e2",
              "labels": [
                {
                  "height": 12,
                  "text": "e2",
                  "width": 20,
                  "x": -61,
                  "y": 27
                }
              ],
              "sections": [
                {
                  "bendPoints": [
                    {
                      "x": -31,
                      "y": 42
                    },
                    {
                      "x": -31,
                      "y": 16
                    },
                    {
                      "x": 72,
                      "y": 16
                    },
                    {
                      "x": 72,
                      "y": 42
                    }
                  ],
                  "endPoint": {
                    "x": 82,
                    "y": 42
                  },
                  "id": "e2_s0",
                  "incomingShape": "x",
                  "outgoingShape": "z",
                  "startPoint": {
                    "x": -81,
                    "y": 42
                  }
                }
              ],
              "sources": [
                "x"
              ],
              "targets": [
                "z"
              ]
            },
            {
              "id": "e3",
              "sections": [
                {
                  "endPoint": {
                    "x": -10,
                    "y": 72
                  },
                  "id": "e3_s0",
                  "incomingShape": "x",
                  "outgoingShape": "p",
                  "startPoint": {
                    "x": -81,
                    "y": 72
                  }
                }
              ],
              "sources": [
                "x"
              ],
              "targets": [
                "p"
              ]
            },
            {
              "id": "e4",
              "sections": [
                {
                  "endPoint": {
                    "x": 12,
                    "y": 72
                  },
                  "id": "e4_s0",
                  "incomingShape": "p",
                  "outgoingShape": "y",
                  "startPoint": {
                    "x": -10,
                    "y": 72
                  }
                }
              ],
              "sources": [
                "p"
              ],
              "targets": [
                "y"
              ]
            }
          ],
          "height": 129,
          "id": "B",
          "labels": [
            {
              "height": 12,
              "text": "B",
              "width": 10,
              "x": 0,
              "y": 0
            }
          ],
          "ports": [
            {
              "height": 10,
              "id": "p",
              "labels": [
                {
                  "height": 12,
                  "text": "p",
                  "width": 10,
                  "x": -11,
                  "y": 11
                }
              ],
              "width": 10,
              "x": -10,
              "y": 67
            }
          ],
          "width": 144,
          "x": 143,
          "y": 12
        }
      ],
      "height": 153,
      "id": "A",
      "width": 299,
      "x": 12,
      "y": 12
    }
  ],
  "height": 177,
  "id": "root",
  "properties": {
    "algorithm": "layered",
    "org.eclipse.elk.hierarchyHandling": "INCLUDE_CHILDREN",
    "org.eclipse.elk.json.edgeCoords": "PARENT",
    "org.eclipse.elk.json.shapeCoords": "PARENT"
  },
  "width": 323,
  "x": 0,
  "y": 0
}
"#;
const OUTPUT_PR: &str = r#"{
  "children": [
    {
      "children": [
        {
          "height": 90,
          "id": "x",
          "width": 50,
          "x": 12,
          "y": 24
        },
        {
          "children": [
            {
              "height": 90,
              "id": "y",
              "width": 50,
              "x": 12,
              "y": 27
            },
            {
              "height": 90,
              "id": "z",
              "width": 50,
              "x": 82,
              "y": 12
            }
          ],
          "edges": [
            {
              "id": "e1",
              "sections": [
                {
                  "endPoint": {
                    "x": 237,
                    "y": 96
                  },
                  "id": "e1_s0",
                  "incomingShape": "y",
                  "outgoingShape": "z",
                  "startPoint": {
                    "x": 217,
                    "y": 96
                  }
                }
              ],
              "sources": [
                "y"
              ],
              "targets": [
                "z"
              ]
            },
            {
              "id": "e2",
              "labels": [
                {
                  "height": 12,
                  "text": "e2",
                  "width": 20,
                  "x": 94,
                  "y": 51
                }
              ],
              "sections": [
                {
                  "bendPoints": [
                    {
                      "x": 124,
                      "y": 66
                    },
                    {
                      "x": 124,
                      "y": 40
                    },
                    {
                      "x": 227,
                      "y": 40
                    },
                    {
                      "x": 227,
                      "y": 66
                    }
                  ],
                  "endPoint": {
                    "x": 237,
                    "y": 66
                  },
                  "id": "e2_s0",
                  "incomingShape": "x",
                  "outgoingShape": "z",
                  "startPoint": {
                    "x": 74,
                    "y": 66
                  }
                }
              ],
              "sources": [
                "x"
              ],
              "targets": [
                "z"
              ]
            },
            {
              "id": "e3",
              "sections": [
                {
                  "endPoint": {
                    "x": 145,
                    "y": 96
                  },
                  "id": "e3_s0",
                  "incomingShape": "x",
                  "outgoingShape": "p",
                  "startPoint": {
                    "x": 74,
                    "y": 96
                  }
                }
              ],
              "sources": [
                "x"
              ],
              "targets": [
                "p"
              ]
            },
            {
              "id": "e4",
              "sections": [
                {
                  "endPoint": {
                    "x": 167,
                    "y": 96
                  },
                  "id": "e4_s0",
                  "incomingShape": "p",
                  "outgoingShape": "y",
                  "startPoint": {
                    "x": 145,
                    "y": 96
                  }
                }
              ],
              "sources": [
                "p"
              ],
              "targets": [
                "y"
              ]
            }
          ],
          "height": 129,
          "id": "B",
          "labels": [
            {
              "height": 12,
              "text": "B",
              "width": 10,
              "x": 0,
              "y": 0
            }
          ],
          "ports": [
            {
              "height": 10,
              "id": "p",
              "labels": [
                {
                  "height": 12,
                  "text": "p",
                  "width": 10,
                  "x": -11,
                  "y": 11
                }
              ],
              "width": 10,
              "x": -10,
              "y": 67
            }
          ],
          "width": 144,
          "x": 143,
          "y": 12
        }
      ],
      "height": 153,
      "id": "A",
      "width": 299,
      "x": 12,
      "y": 12
    }
  ],
  "height": 177,
  "id": "root",
  "properties": {
    "algorithm": "layered",
    "org.eclipse.elk.hierarchyHandling": "INCLUDE_CHILDREN",
    "org.eclipse.elk.json.edgeCoords": "ROOT",
    "org.eclipse.elk.json.shapeCoords": "PARENT"
  },
  "width": 323,
  "x": 0,
  "y": 0
}
"#;
const OUTPUT_RC: &str = r#"{
  "children": [
    {
      "children": [
        {
          "height": 90,
          "id": "x",
          "width": 50,
          "x": 24,
          "y": 36
        },
        {
          "children": [
            {
              "height": 90,
              "id": "y",
              "width": 50,
              "x": 167,
              "y": 51
            },
            {
              "height": 90,
              "id": "z",
              "width": 50,
              "x": 237,
              "y": 36
            }
          ],
          "edges": [
            {
              "container": "B",
              "id": "e1",
              "sections": [
                {
                  "endPoint": {
                    "x": 82,
                    "y": 72
                  },
                  "id": "e1_s0",
                  "incomingShape": "y",
                  "outgoingShape": "z",
                  "startPoint": {
                    "x": 62,
                    "y": 72
                  }
                }
              ],
              "sources": [
                "y"
              ],
              "targets": [
                "z"
              ]
            },
            {
              "container": "A",
              "id": "e2",
              "labels": [
                {
                  "height": 12,
                  "text": "e2",
                  "width": 20,
                  "x": 82,
                  "y": 39
                }
              ],
              "sections": [
                {
                  "bendPoints": [
                    {
                      "x": 112,
                      "y": 54
                    },
                    {
                      "x": 112,
                      "y": 28
                    },
                    {
                      "x": 215,
                      "y": 28
                    },
                    {
                      "x": 215,
                      "y": 54
                    }
                  ],
                  "endPoint": {
                    "x": 225,
                    "y": 54
                  },
                  "id": "e2_s0",
                  "incomingShape": "x",
                  "outgoingShape": "z",
                  "startPoint": {
                    "x": 62,
                    "y": 54
                  }
                }
              ],
              "sources": [
                "x"
              ],
              "targets": [
                "z"
              ]
            },
            {
              "container": "A",
              "id": "e3",
              "sections": [
                {
                  "endPoint": {
                    "x": 133,
                    "y": 84
                  },
                  "id": "e3_s0",
                  "incomingShape": "x",
                  "outgoingShape": "p",
                  "startPoint": {
                    "x": 62,
                    "y": 84
                  }
                }
              ],
              "sources": [
                "x"
              ],
              "targets": [
                "p"
              ]
            },
            {
              "container": "B",
              "id": "e4",
              "sections": [
                {
                  "endPoint": {
                    "x": 12,
                    "y": 72
                  },
                  "id": "e4_s0",
                  "incomingShape": "p",
                  "outgoingShape": "y",
                  "startPoint": {
                    "x": -10,
                    "y": 72
                  }
                }
              ],
              "sources": [
                "p"
              ],
              "targets": [
                "y"
              ]
            }
          ],
          "height": 129,
          "id": "B",
          "labels": [
            {
              "height": 12,
              "text": "B",
              "width": 10,
              "x": 155,
              "y": 24
            }
          ],
          "ports": [
            {
              "height": 10,
              "id": "p",
              "labels": [
                {
                  "height": 12,
                  "text": "p",
                  "width": 10,
                  "x": 134,
                  "y": 102
                }
              ],
              "width": 10,
              "x": 145,
              "y": 91
            }
          ],
          "width": 144,
          "x": 155,
          "y": 24
        }
      ],
      "height": 153,
      "id": "A",
      "width": 299,
      "x": 12,
      "y": 12
    }
  ],
  "height": 177,
  "id": "root",
  "properties": {
    "algorithm": "layered",
    "org.eclipse.elk.hierarchyHandling": "INCLUDE_CHILDREN",
    "org.eclipse.elk.json.edgeCoords": "CONTAINER",
    "org.eclipse.elk.json.shapeCoords": "ROOT"
  },
  "width": 323,
  "x": 0,
  "y": 0
}
"#;
const OUTPUT_RP: &str = r#"{
  "children": [
    {
      "children": [
        {
          "height": 90,
          "id": "x",
          "width": 50,
          "x": 24,
          "y": 36
        },
        {
          "children": [
            {
              "height": 90,
              "id": "y",
              "width": 50,
              "x": 167,
              "y": 51
            },
            {
              "height": 90,
              "id": "z",
              "width": 50,
              "x": 237,
              "y": 36
            }
          ],
          "edges": [
            {
              "id": "e1",
              "sections": [
                {
                  "endPoint": {
                    "x": 82,
                    "y": 72
                  },
                  "id": "e1_s0",
                  "incomingShape": "y",
                  "outgoingShape": "z",
                  "startPoint": {
                    "x": 62,
                    "y": 72
                  }
                }
              ],
              "sources": [
                "y"
              ],
              "targets": [
                "z"
              ]
            },
            {
              "id": "e2",
              "labels": [
                {
                  "height": 12,
                  "text": "e2",
                  "width": 20,
                  "x": -61,
                  "y": 27
                }
              ],
              "sections": [
                {
                  "bendPoints": [
                    {
                      "x": -31,
                      "y": 42
                    },
                    {
                      "x": -31,
                      "y": 16
                    },
                    {
                      "x": 72,
                      "y": 16
                    },
                    {
                      "x": 72,
                      "y": 42
                    }
                  ],
                  "endPoint": {
                    "x": 82,
                    "y": 42
                  },
                  "id": "e2_s0",
                  "incomingShape": "x",
                  "outgoingShape": "z",
                  "startPoint": {
                    "x": -81,
                    "y": 42
                  }
                }
              ],
              "sources": [
                "x"
              ],
              "targets": [
                "z"
              ]
            },
            {
              "id": "e3",
              "sections": [
                {
                  "endPoint": {
                    "x": -10,
                    "y": 72
                  },
                  "id": "e3_s0",
                  "incomingShape": "x",
                  "outgoingShape": "p",
                  "startPoint": {
                    "x": -81,
                    "y": 72
                  }
                }
              ],
              "sources": [
                "x"
              ],
              "targets": [
                "p"
              ]
            },
            {
              "id": "e4",
              "sections": [
                {
                  "endPoint": {
                    "x": 12,
                    "y": 72
                  },
                  "id": "e4_s0",
                  "incomingShape": "p",
                  "outgoingShape": "y",
                  "startPoint": {
                    "x": -10,
                    "y": 72
                  }
                }
              ],
              "sources": [
                "p"
              ],
              "targets": [
                "y"
              ]
            }
          ],
          "height": 129,
          "id": "B",
          "labels": [
            {
              "height": 12,
              "text": "B",
              "width": 10,
              "x": 155,
              "y": 24
            }
          ],
          "ports": [
            {
              "height": 10,
              "id": "p",
              "labels": [
                {
                  "height": 12,
                  "text": "p",
                  "width": 10,
                  "x": 134,
                  "y": 102
                }
              ],
              "width": 10,
              "x": 145,
              "y": 91
            }
          ],
          "width": 144,
          "x": 155,
          "y": 24
        }
      ],
      "height": 153,
      "id": "A",
      "width": 299,
      "x": 12,
      "y": 12
    }
  ],
  "height": 177,
  "id": "root",
  "properties": {
    "algorithm": "layered",
    "org.eclipse.elk.hierarchyHandling": "INCLUDE_CHILDREN",
    "org.eclipse.elk.json.edgeCoords": "PARENT",
    "org.eclipse.elk.json.shapeCoords": "ROOT"
  },
  "width": 323,
  "x": 0,
  "y": 0
}
"#;
const OUTPUT_RR: &str = r#"{
  "children": [
    {
      "children": [
        {
          "height": 90,
          "id": "x",
          "width": 50,
          "x": 24,
          "y": 36
        },
        {
          "children": [
            {
              "height": 90,
              "id": "y",
              "width": 50,
              "x": 167,
              "y": 51
            },
            {
              "height": 90,
              "id": "z",
              "width": 50,
              "x": 237,
              "y": 36
            }
          ],
          "edges": [
            {
              "id": "e1",
              "sections": [
                {
                  "endPoint": {
                    "x": 237,
                    "y": 96
                  },
                  "id": "e1_s0",
                  "incomingShape": "y",
                  "outgoingShape": "z",
                  "startPoint": {
                    "x": 217,
                    "y": 96
                  }
                }
              ],
              "sources": [
                "y"
              ],
              "targets": [
                "z"
              ]
            },
            {
              "id": "e2",
              "labels": [
                {
                  "height": 12,
                  "text": "e2",
                  "width": 20,
                  "x": 94,
                  "y": 51
                }
              ],
              "sections": [
                {
                  "bendPoints": [
                    {
                      "x": 124,
                      "y": 66
                    },
                    {
                      "x": 124,
                      "y": 40
                    },
                    {
                      "x": 227,
                      "y": 40
                    },
                    {
                      "x": 227,
                      "y": 66
                    }
                  ],
                  "endPoint": {
                    "x": 237,
                    "y": 66
                  },
                  "id": "e2_s0",
                  "incomingShape": "x",
                  "outgoingShape": "z",
                  "startPoint": {
                    "x": 74,
                    "y": 66
                  }
                }
              ],
              "sources": [
                "x"
              ],
              "targets": [
                "z"
              ]
            },
            {
              "id": "e3",
              "sections": [
                {
                  "endPoint": {
                    "x": 145,
                    "y": 96
                  },
                  "id": "e3_s0",
                  "incomingShape": "x",
                  "outgoingShape": "p",
                  "startPoint": {
                    "x": 74,
                    "y": 96
                  }
                }
              ],
              "sources": [
                "x"
              ],
              "targets": [
                "p"
              ]
            },
            {
              "id": "e4",
              "sections": [
                {
                  "endPoint": {
                    "x": 167,
                    "y": 96
                  },
                  "id": "e4_s0",
                  "incomingShape": "p",
                  "outgoingShape": "y",
                  "startPoint": {
                    "x": 145,
                    "y": 96
                  }
                }
              ],
              "sources": [
                "p"
              ],
              "targets": [
                "y"
              ]
            }
          ],
          "height": 129,
          "id": "B",
          "labels": [
            {
              "height": 12,
              "text": "B",
              "width": 10,
              "x": 155,
              "y": 24
            }
          ],
          "ports": [
            {
              "height": 10,
              "id": "p",
              "labels": [
                {
                  "height": 12,
                  "text": "p",
                  "width": 10,
                  "x": 134,
                  "y": 102
                }
              ],
              "width": 10,
              "x": 145,
              "y": 91
            }
          ],
          "width": 144,
          "x": 155,
          "y": 24
        }
      ],
      "height": 153,
      "id": "A",
      "width": 299,
      "x": 12,
      "y": 12
    }
  ],
  "height": 177,
  "id": "root",
  "properties": {
    "algorithm": "layered",
    "org.eclipse.elk.hierarchyHandling": "INCLUDE_CHILDREN",
    "org.eclipse.elk.json.edgeCoords": "ROOT",
    "org.eclipse.elk.json.shapeCoords": "ROOT"
  },
  "width": 323,
  "x": 0,
  "y": 0
}
"#;

#[test]
fn edge_coords_round_trip() {
    initialize_plain_java_layout();

    let cases = [
        ("PARENT", "CONTAINER", OUTPUT_PC),
        ("PARENT", "PARENT", OUTPUT_PP),
        ("PARENT", "ROOT", OUTPUT_PR),
        ("ROOT", "CONTAINER", OUTPUT_RC),
        ("ROOT", "PARENT", OUTPUT_RP),
        ("ROOT", "ROOT", OUTPUT_RR),
    ];

    for (shape_coords, edge_coords, expected_output) in cases {
        let mut json_graph = parse_lenient_json(GRAPH);
        let properties = json_graph
            .get_mut("properties")
            .and_then(|value| value.as_object_mut())
            .expect("properties");
        properties.insert(
            "org.eclipse.elk.json.shapeCoords".to_string(),
            Value::String(shape_coords.to_string()),
        );
        properties.insert(
            "org.eclipse.elk.json.edgeCoords".to_string(),
            Value::String(edge_coords.to_string()),
        );

        let shared = Rc::new(RefCell::new(json_graph));
        let mut importer = Maybe::default();
        let root = ElkGraphJson::for_graph_shared(shared.clone())
            .remember_importer(&mut importer)
            .to_elk()
            .unwrap();

        let mut engine = RecursiveGraphLayoutEngine::new();
        engine.layout(&root, &mut BasicProgressMonitor::new());

        importer
            .get_mut()
            .expect("importer")
            .transfer_layout(&root)
            .unwrap();

        let computed = shared.borrow().clone();
        let expected = parse_lenient_json(expected_output);
        assert_eq!(
            expected, computed,
            "shapeCoords={shape_coords} edgeCoords={edge_coords}"
        );
    }
}

fn parse_lenient_json(input: &str) -> Value {
    json5::from_str(input).expect("lenient json")
}
