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

const OUTPUT_PC: &str = r#"    {
     "id": "root",
     "properties": {
      "algorithm": "layered",
      "org.eclipse.elk.hierarchyHandling": "INCLUDE_CHILDREN",
      "org.eclipse.elk.json.shapeCoords": "PARENT",
      "org.eclipse.elk.json.edgeCoords": "CONTAINER"
     },
     "children": [
      {
       "id": "A",
       "children": [
        {
         "id": "x",
         "width": 50,
         "height": 90,
         "x": 12,
         "y": 39
        },
        {
         "id": "B",
         "labels": [
          {
           "text": "B",
           "width": 10,
           "height": 12,
           "x": 0,
           "y": 0
          }
         ],
         "ports": [
          {
           "id": "p",
           "width": 10,
           "height": 10,
           "labels": [
            {
             "text": "p",
             "width": 10,
             "height": 12,
             "x": -11,
             "y": -13
            }
           ],
           "x": -10,
           "y": 52
          }
         ],
         "children": [
          {
           "id": "y",
           "width": 50,
           "height": 90,
           "x": 12,
           "y": 12
          },
          {
           "id": "z",
           "width": 50,
           "height": 90,
           "x": 82,
           "y": 27
          }
         ],
         "edges": [
          {
           "id": "e1",
           "sources": [
            "y"
           ],
           "targets": [
            "z"
           ],
           "sections": [
            {
             "id": "e1_s0",
             "startPoint": {
              "x": 62,
              "y": 57
             },
             "endPoint": {
              "x": 82,
              "y": 57
             },
             "incomingShape": "y",
             "outgoingShape": "z"
            }
           ],
           "container": "B"
          },
          {
           "id": "e2",
           "sources": [
            "x"
           ],
           "targets": [
            "z"
           ],
           "labels": [
            {
             "text": "e2",
             "width": 20,
             "height": 12,
             "x": 82,
             "y": 102
            }
           ],
           "sections": [
            {
             "id": "e2_s0",
             "startPoint": {
              "x": 62,
              "y": 99
             },
             "endPoint": {
              "x": 225,
              "y": 99
             },
             "bendPoints": [
              {
               "x": 112,
               "y": 99
              },
              {
               "x": 112,
               "y": 124
              },
              {
               "x": 215,
               "y": 124
              },
              {
               "x": 215,
               "y": 99
              }
             ],
             "incomingShape": "x",
             "outgoingShape": "z"
            }
           ],
           "container": "A"
          },
          {
           "id": "e3",
           "sources": [
            "x"
           ],
           "targets": [
            "p"
           ],
           "sections": [
            {
             "id": "e3_s0",
             "startPoint": {
              "x": 62,
              "y": 69
             },
             "endPoint": {
              "x": 133,
              "y": 69
             },
             "incomingShape": "x",
             "outgoingShape": "p"
            }
           ],
           "container": "A"
          },
          {
           "id": "e4",
           "sources": [
            "p"
           ],
           "targets": [
            "y"
           ],
           "sections": [
            {
             "id": "e4_s0",
             "startPoint": {
              "x": -10,
              "y": 57
             },
             "endPoint": {
              "x": 12,
              "y": 57
             },
             "incomingShape": "p",
             "outgoingShape": "y"
            }
           ],
           "container": "B"
          }
         ],
         "x": 143,
         "y": 12,
         "width": 144,
         "height": 129
        }
       ],
       "x": 12,
       "y": 12,
       "width": 299,
       "height": 153
      }
     ],
     "x": 0,
     "y": 0,
     "width": 323,
     "height": 177
    }"#;
const OUTPUT_PP: &str = r#"    {
     "id": "root",
     "properties": {
      "algorithm": "layered",
      "org.eclipse.elk.hierarchyHandling": "INCLUDE_CHILDREN",
      "org.eclipse.elk.json.shapeCoords": "PARENT",
      "org.eclipse.elk.json.edgeCoords": "PARENT"
     },
     "children": [
      {
       "id": "A",
       "children": [
        {
         "id": "x",
         "width": 50,
         "height": 90,
         "x": 12,
         "y": 39
        },
        {
         "id": "B",
         "labels": [
          {
           "text": "B",
           "width": 10,
           "height": 12,
           "x": 0,
           "y": 0
          }
         ],
         "ports": [
          {
           "id": "p",
           "width": 10,
           "height": 10,
           "labels": [
            {
             "text": "p",
             "width": 10,
             "height": 12,
             "x": -11,
             "y": -13
            }
           ],
           "x": -10,
           "y": 52
          }
         ],
         "children": [
          {
           "id": "y",
           "width": 50,
           "height": 90,
           "x": 12,
           "y": 12
          },
          {
           "id": "z",
           "width": 50,
           "height": 90,
           "x": 82,
           "y": 27
          }
         ],
         "edges": [
          {
           "id": "e1",
           "sources": [
            "y"
           ],
           "targets": [
            "z"
           ],
           "sections": [
            {
             "id": "e1_s0",
             "startPoint": {
              "x": 62,
              "y": 57
             },
             "endPoint": {
              "x": 82,
              "y": 57
             },
             "incomingShape": "y",
             "outgoingShape": "z"
            }
           ]
          },
          {
           "id": "e2",
           "sources": [
            "x"
           ],
           "targets": [
            "z"
           ],
           "labels": [
            {
             "text": "e2",
             "width": 20,
             "height": 12,
             "x": -61,
             "y": 90
            }
           ],
           "sections": [
            {
             "id": "e2_s0",
             "startPoint": {
              "x": -81,
              "y": 87
             },
             "endPoint": {
              "x": 82,
              "y": 87
             },
             "bendPoints": [
              {
               "x": -31,
               "y": 87
              },
              {
               "x": -31,
               "y": 112
              },
              {
               "x": 72,
               "y": 112
              },
              {
               "x": 72,
               "y": 87
              }
             ],
             "incomingShape": "x",
             "outgoingShape": "z"
            }
           ]
          },
          {
           "id": "e3",
           "sources": [
            "x"
           ],
           "targets": [
            "p"
           ],
           "sections": [
            {
             "id": "e3_s0",
             "startPoint": {
              "x": -81,
              "y": 57
             },
             "endPoint": {
              "x": -10,
              "y": 57
             },
             "incomingShape": "x",
             "outgoingShape": "p"
            }
           ]
          },
          {
           "id": "e4",
           "sources": [
            "p"
           ],
           "targets": [
            "y"
           ],
           "sections": [
            {
             "id": "e4_s0",
             "startPoint": {
              "x": -10,
              "y": 57
             },
             "endPoint": {
              "x": 12,
              "y": 57
             },
             "incomingShape": "p",
             "outgoingShape": "y"
            }
           ]
          }
         ],
         "x": 143,
         "y": 12,
         "width": 144,
         "height": 129
        }
       ],
       "x": 12,
       "y": 12,
       "width": 299,
       "height": 153
      }
     ],
     "x": 0,
     "y": 0,
     "width": 323,
     "height": 177
    }"#;
const OUTPUT_PR: &str = r#"    {
     "id": "root",
     "properties": {
      "algorithm": "layered",
      "org.eclipse.elk.hierarchyHandling": "INCLUDE_CHILDREN",
      "org.eclipse.elk.json.shapeCoords": "PARENT",
      "org.eclipse.elk.json.edgeCoords": "ROOT"
     },
     "children": [
      {
       "id": "A",
       "children": [
        {
         "id": "x",
         "width": 50,
         "height": 90,
         "x": 12,
         "y": 39
        },
        {
         "id": "B",
         "labels": [
          {
           "text": "B",
           "width": 10,
           "height": 12,
           "x": 0,
           "y": 0
          }
         ],
         "ports": [
          {
           "id": "p",
           "width": 10,
           "height": 10,
           "labels": [
            {
             "text": "p",
             "width": 10,
             "height": 12,
             "x": -11,
             "y": -13
            }
           ],
           "x": -10,
           "y": 52
          }
         ],
         "children": [
          {
           "id": "y",
           "width": 50,
           "height": 90,
           "x": 12,
           "y": 12
          },
          {
           "id": "z",
           "width": 50,
           "height": 90,
           "x": 82,
           "y": 27
          }
         ],
         "edges": [
          {
           "id": "e1",
           "sources": [
            "y"
           ],
           "targets": [
            "z"
           ],
           "sections": [
            {
             "id": "e1_s0",
             "startPoint": {
              "x": 217,
              "y": 81
             },
             "endPoint": {
              "x": 237,
              "y": 81
             },
             "incomingShape": "y",
             "outgoingShape": "z"
            }
           ]
          },
          {
           "id": "e2",
           "sources": [
            "x"
           ],
           "targets": [
            "z"
           ],
           "labels": [
            {
             "text": "e2",
             "width": 20,
             "height": 12,
             "x": 94,
             "y": 114
            }
           ],
           "sections": [
            {
             "id": "e2_s0",
             "startPoint": {
              "x": 74,
              "y": 111
             },
             "endPoint": {
              "x": 237,
              "y": 111
             },
             "bendPoints": [
              {
               "x": 124,
               "y": 111
              },
              {
               "x": 124,
               "y": 136
              },
              {
               "x": 227,
               "y": 136
              },
              {
               "x": 227,
               "y": 111
              }
             ],
             "incomingShape": "x",
             "outgoingShape": "z"
            }
           ]
          },
          {
           "id": "e3",
           "sources": [
            "x"
           ],
           "targets": [
            "p"
           ],
           "sections": [
            {
             "id": "e3_s0",
             "startPoint": {
              "x": 74,
              "y": 81
             },
             "endPoint": {
              "x": 145,
              "y": 81
             },
             "incomingShape": "x",
             "outgoingShape": "p"
            }
           ]
          },
          {
           "id": "e4",
           "sources": [
            "p"
           ],
           "targets": [
            "y"
           ],
           "sections": [
            {
             "id": "e4_s0",
             "startPoint": {
              "x": 145,
              "y": 81
             },
             "endPoint": {
              "x": 167,
              "y": 81
             },
             "incomingShape": "p",
             "outgoingShape": "y"
            }
           ]
          }
         ],
         "x": 143,
         "y": 12,
         "width": 144,
         "height": 129
        }
       ],
       "x": 12,
       "y": 12,
       "width": 299,
       "height": 153
      }
     ],
     "x": 0,
     "y": 0,
     "width": 323,
     "height": 177
    }"#;
const OUTPUT_RC: &str = r#"    {
     "id": "root",
     "properties": {
      "algorithm": "layered",
      "org.eclipse.elk.hierarchyHandling": "INCLUDE_CHILDREN",
      "org.eclipse.elk.json.shapeCoords": "ROOT",
      "org.eclipse.elk.json.edgeCoords": "CONTAINER"
     },
     "children": [
      {
       "id": "A",
       "children": [
        {
         "id": "x",
         "width": 50,
         "height": 90,
         "x": 24,
         "y": 51
        },
        {
         "id": "B",
         "labels": [
          {
           "text": "B",
           "width": 10,
           "height": 12,
           "x": 155,
           "y": 24
          }
         ],
         "ports": [
          {
           "id": "p",
           "width": 10,
           "height": 10,
           "labels": [
            {
             "text": "p",
             "width": 10,
             "height": 12,
             "x": 134,
             "y": 63
            }
           ],
           "x": 145,
           "y": 76
          }
         ],
         "children": [
          {
           "id": "y",
           "width": 50,
           "height": 90,
           "x": 167,
           "y": 36
          },
          {
           "id": "z",
           "width": 50,
           "height": 90,
           "x": 237,
           "y": 51
          }
         ],
         "edges": [
          {
           "id": "e1",
           "sources": [
            "y"
           ],
           "targets": [
            "z"
           ],
           "sections": [
            {
             "id": "e1_s0",
             "startPoint": {
              "x": 62,
              "y": 57
             },
             "endPoint": {
              "x": 82,
              "y": 57
             },
             "incomingShape": "y",
             "outgoingShape": "z"
            }
           ],
           "container": "B"
          },
          {
           "id": "e2",
           "sources": [
            "x"
           ],
           "targets": [
            "z"
           ],
           "labels": [
            {
             "text": "e2",
             "width": 20,
             "height": 12,
             "x": 82,
             "y": 102
            }
           ],
           "sections": [
            {
             "id": "e2_s0",
             "startPoint": {
              "x": 62,
              "y": 99
             },
             "endPoint": {
              "x": 225,
              "y": 99
             },
             "bendPoints": [
              {
               "x": 112,
               "y": 99
              },
              {
               "x": 112,
               "y": 124
              },
              {
               "x": 215,
               "y": 124
              },
              {
               "x": 215,
               "y": 99
              }
             ],
             "incomingShape": "x",
             "outgoingShape": "z"
            }
           ],
           "container": "A"
          },
          {
           "id": "e3",
           "sources": [
            "x"
           ],
           "targets": [
            "p"
           ],
           "sections": [
            {
             "id": "e3_s0",
             "startPoint": {
              "x": 62,
              "y": 69
             },
             "endPoint": {
              "x": 133,
              "y": 69
             },
             "incomingShape": "x",
             "outgoingShape": "p"
            }
           ],
           "container": "A"
          },
          {
           "id": "e4",
           "sources": [
            "p"
           ],
           "targets": [
            "y"
           ],
           "sections": [
            {
             "id": "e4_s0",
             "startPoint": {
              "x": -10,
              "y": 57
             },
             "endPoint": {
              "x": 12,
              "y": 57
             },
             "incomingShape": "p",
             "outgoingShape": "y"
            }
           ],
           "container": "B"
          }
         ],
         "x": 155,
         "y": 24,
         "width": 144,
         "height": 129
        }
       ],
       "x": 12,
       "y": 12,
       "width": 299,
       "height": 153
      }
     ],
     "x": 0,
     "y": 0,
     "width": 323,
     "height": 177
    }"#;
const OUTPUT_RP: &str = r#"    {
     "id": "root",
     "properties": {
      "algorithm": "layered",
      "org.eclipse.elk.hierarchyHandling": "INCLUDE_CHILDREN",
      "org.eclipse.elk.json.shapeCoords": "ROOT",
      "org.eclipse.elk.json.edgeCoords": "PARENT"
     },
     "children": [
      {
       "id": "A",
       "children": [
        {
         "id": "x",
         "width": 50,
         "height": 90,
         "x": 24,
         "y": 51
        },
        {
         "id": "B",
         "labels": [
          {
           "text": "B",
           "width": 10,
           "height": 12,
           "x": 155,
           "y": 24
          }
         ],
         "ports": [
          {
           "id": "p",
           "width": 10,
           "height": 10,
           "labels": [
            {
             "text": "p",
             "width": 10,
             "height": 12,
             "x": 134,
             "y": 63
            }
           ],
           "x": 145,
           "y": 76
          }
         ],
         "children": [
          {
           "id": "y",
           "width": 50,
           "height": 90,
           "x": 167,
           "y": 36
          },
          {
           "id": "z",
           "width": 50,
           "height": 90,
           "x": 237,
           "y": 51
          }
         ],
         "edges": [
          {
           "id": "e1",
           "sources": [
            "y"
           ],
           "targets": [
            "z"
           ],
           "sections": [
            {
             "id": "e1_s0",
             "startPoint": {
              "x": 62,
              "y": 57
             },
             "endPoint": {
              "x": 82,
              "y": 57
             },
             "incomingShape": "y",
             "outgoingShape": "z"
            }
           ]
          },
          {
           "id": "e2",
           "sources": [
            "x"
           ],
           "targets": [
            "z"
           ],
           "labels": [
            {
             "text": "e2",
             "width": 20,
             "height": 12,
             "x": -61,
             "y": 90
            }
           ],
           "sections": [
            {
             "id": "e2_s0",
             "startPoint": {
              "x": -81,
              "y": 87
             },
             "endPoint": {
              "x": 82,
              "y": 87
             },
             "bendPoints": [
              {
               "x": -31,
               "y": 87
              },
              {
               "x": -31,
               "y": 112
              },
              {
               "x": 72,
               "y": 112
              },
              {
               "x": 72,
               "y": 87
              }
             ],
             "incomingShape": "x",
             "outgoingShape": "z"
            }
           ]
          },
          {
           "id": "e3",
           "sources": [
            "x"
           ],
           "targets": [
            "p"
           ],
           "sections": [
            {
             "id": "e3_s0",
             "startPoint": {
              "x": -81,
              "y": 57
             },
             "endPoint": {
              "x": -10,
              "y": 57
             },
             "incomingShape": "x",
             "outgoingShape": "p"
            }
           ]
          },
          {
           "id": "e4",
           "sources": [
            "p"
           ],
           "targets": [
            "y"
           ],
           "sections": [
            {
             "id": "e4_s0",
             "startPoint": {
              "x": -10,
              "y": 57
             },
             "endPoint": {
              "x": 12,
              "y": 57
             },
             "incomingShape": "p",
             "outgoingShape": "y"
            }
           ]
          }
         ],
         "x": 155,
         "y": 24,
         "width": 144,
         "height": 129
        }
       ],
       "x": 12,
       "y": 12,
       "width": 299,
       "height": 153
      }
     ],
     "x": 0,
     "y": 0,
     "width": 323,
     "height": 177
    }"#;
const OUTPUT_RR: &str = r#"    {
     "id": "root",
     "properties": {
      "algorithm": "layered",
      "org.eclipse.elk.hierarchyHandling": "INCLUDE_CHILDREN",
      "org.eclipse.elk.json.shapeCoords": "ROOT",
      "org.eclipse.elk.json.edgeCoords": "ROOT"
     },
     "children": [
      {
       "id": "A",
       "children": [
        {
         "id": "x",
         "width": 50,
         "height": 90,
         "x": 24,
         "y": 51
        },
        {
         "id": "B",
         "labels": [
          {
           "text": "B",
           "width": 10,
           "height": 12,
           "x": 155,
           "y": 24
          }
         ],
         "ports": [
          {
           "id": "p",
           "width": 10,
           "height": 10,
           "labels": [
            {
             "text": "p",
             "width": 10,
             "height": 12,
             "x": 134,
             "y": 63
            }
           ],
           "x": 145,
           "y": 76
          }
         ],
         "children": [
          {
           "id": "y",
           "width": 50,
           "height": 90,
           "x": 167,
           "y": 36
          },
          {
           "id": "z",
           "width": 50,
           "height": 90,
           "x": 237,
           "y": 51
          }
         ],
         "edges": [
          {
           "id": "e1",
           "sources": [
            "y"
           ],
           "targets": [
            "z"
           ],
           "sections": [
            {
             "id": "e1_s0",
             "startPoint": {
              "x": 217,
              "y": 81
             },
             "endPoint": {
              "x": 237,
              "y": 81
             },
             "incomingShape": "y",
             "outgoingShape": "z"
            }
           ]
          },
          {
           "id": "e2",
           "sources": [
            "x"
           ],
           "targets": [
            "z"
           ],
           "labels": [
            {
             "text": "e2",
             "width": 20,
             "height": 12,
             "x": 94,
             "y": 114
            }
           ],
           "sections": [
            {
             "id": "e2_s0",
             "startPoint": {
              "x": 74,
              "y": 111
             },
             "endPoint": {
              "x": 237,
              "y": 111
             },
             "bendPoints": [
              {
               "x": 124,
               "y": 111
              },
              {
               "x": 124,
               "y": 136
              },
              {
               "x": 227,
               "y": 136
              },
              {
               "x": 227,
               "y": 111
              }
             ],
             "incomingShape": "x",
             "outgoingShape": "z"
            }
           ]
          },
          {
           "id": "e3",
           "sources": [
            "x"
           ],
           "targets": [
            "p"
           ],
           "sections": [
            {
             "id": "e3_s0",
             "startPoint": {
              "x": 74,
              "y": 81
             },
             "endPoint": {
              "x": 145,
              "y": 81
             },
             "incomingShape": "x",
             "outgoingShape": "p"
            }
           ]
          },
          {
           "id": "e4",
           "sources": [
            "p"
           ],
           "targets": [
            "y"
           ],
           "sections": [
            {
             "id": "e4_s0",
             "startPoint": {
              "x": 145,
              "y": 81
             },
             "endPoint": {
              "x": 167,
              "y": 81
             },
             "incomingShape": "p",
             "outgoingShape": "y"
            }
           ]
          }
         ],
         "x": 155,
         "y": 24,
         "width": 144,
         "height": 129
        }
       ],
       "x": 12,
       "y": 12,
       "width": 299,
       "height": 153
      }
     ],
     "x": 0,
     "y": 0,
     "width": 323,
     "height": 177
    }"#;

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
