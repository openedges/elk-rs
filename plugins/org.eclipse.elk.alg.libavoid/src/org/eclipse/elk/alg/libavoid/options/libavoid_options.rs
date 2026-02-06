use std::sync::LazyLock;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{Direction, EdgeRouting, PortConstraints, PortSide};

pub struct LibavoidOptions;

pub static SEGMENT_PENALTY_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.alg.libavoid.segmentPenalty", 10.0));

pub static ANGLE_PENALTY_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.alg.libavoid.anglePenalty", 0.0));

pub static CROSSING_PENALTY_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.alg.libavoid.crossingPenalty", 0.0));

pub static CLUSTER_CROSSING_PENALTY_PROPERTY: LazyLock<Property<f64>> = LazyLock::new(|| {
    Property::with_default("org.eclipse.elk.alg.libavoid.clusterCrossingPenalty", 0.0)
});

pub static FIXED_SHARED_PATH_PENALTY_PROPERTY: LazyLock<Property<f64>> = LazyLock::new(|| {
    Property::with_default("org.eclipse.elk.alg.libavoid.fixedSharedPathPenalty", 0.0)
});

pub static PORT_DIRECTION_PENALTY_PROPERTY: LazyLock<Property<f64>> = LazyLock::new(|| {
    Property::with_default("org.eclipse.elk.alg.libavoid.portDirectionPenalty", 0.0)
});

pub static SHAPE_BUFFER_DISTANCE_PROPERTY: LazyLock<Property<f64>> = LazyLock::new(|| {
    Property::with_default("org.eclipse.elk.alg.libavoid.shapeBufferDistance", 4.0)
});

pub static IDEAL_NUDGING_DISTANCE_PROPERTY: LazyLock<Property<f64>> = LazyLock::new(|| {
    Property::with_default("org.eclipse.elk.alg.libavoid.idealNudgingDistance", 4.0)
});

pub static REVERSE_DIRECTION_PENALTY_PROPERTY: LazyLock<Property<f64>> = LazyLock::new(|| {
    Property::with_default("org.eclipse.elk.alg.libavoid.reverseDirectionPenalty", 0.0)
});

pub static NUDGE_ORTHOGONAL_SEGMENTS_CONNECTED_TO_SHAPES_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.libavoid.nudgeOrthogonalSegmentsConnectedToShapes",
            false,
        )
    });

pub static IMPROVE_HYPEREDGE_ROUTES_MOVING_JUNCTIONS_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.libavoid.improveHyperedgeRoutesMovingJunctions",
            true,
        )
    });

pub static PENALISE_ORTHOGONAL_SHARED_PATHS_AT_CONN_ENDS_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.libavoid.penaliseOrthogonalSharedPathsAtConnEnds",
            false,
        )
    });

pub static NUDGE_ORTHOGONAL_TOUCHING_COLINEAR_SEGMENTS_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.libavoid.nudgeOrthogonalTouchingColinearSegments",
            false,
        )
    });

pub static PERFORM_UNIFYING_NUDGING_PREPROCESSING_STEP_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.libavoid.performUnifyingNudgingPreprocessingStep",
            true,
        )
    });

pub static IMPROVE_HYPEREDGE_ROUTES_MOVING_ADDING_AND_DELETING_JUNCTIONS_PROPERTY: LazyLock<
    Property<bool>,
> = LazyLock::new(|| {
    Property::with_default(
        "org.eclipse.elk.alg.libavoid.improveHyperedgeRoutesMovingAddingAndDeletingJunctions",
        false,
    )
});

pub static NUDGE_SHARED_PATHS_WITH_COMMON_END_POINT_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.libavoid.nudgeSharedPathsWithCommonEndPoint",
            true,
        )
    });

pub static ENABLE_HYPEREDGES_FROM_COMMON_SOURCE_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| {
        Property::with_default(
            "org.eclipse.elk.alg.libavoid.enableHyperedgesFromCommonSource",
            false,
        )
    });

pub static IS_CLUSTER_PROPERTY: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("org.eclipse.elk.alg.libavoid.isCluster", false));

pub static PROCESS_TIMEOUT_PROPERTY: LazyLock<Property<i32>> = LazyLock::new(|| {
    Property::with_default("org.eclipse.elk.alg.libavoid.processTimeout", 10000)
});

impl LibavoidOptions {
    pub const ALGORITHM_ID: &'static str = "org.eclipse.elk.alg.libavoid";

    pub const SEGMENT_PENALTY: &'static LazyLock<Property<f64>> = &SEGMENT_PENALTY_PROPERTY;
    pub const ANGLE_PENALTY: &'static LazyLock<Property<f64>> = &ANGLE_PENALTY_PROPERTY;
    pub const CROSSING_PENALTY: &'static LazyLock<Property<f64>> = &CROSSING_PENALTY_PROPERTY;
    pub const CLUSTER_CROSSING_PENALTY: &'static LazyLock<Property<f64>> =
        &CLUSTER_CROSSING_PENALTY_PROPERTY;
    pub const FIXED_SHARED_PATH_PENALTY: &'static LazyLock<Property<f64>> =
        &FIXED_SHARED_PATH_PENALTY_PROPERTY;
    pub const PORT_DIRECTION_PENALTY: &'static LazyLock<Property<f64>> =
        &PORT_DIRECTION_PENALTY_PROPERTY;
    pub const SHAPE_BUFFER_DISTANCE: &'static LazyLock<Property<f64>> =
        &SHAPE_BUFFER_DISTANCE_PROPERTY;
    pub const IDEAL_NUDGING_DISTANCE: &'static LazyLock<Property<f64>> =
        &IDEAL_NUDGING_DISTANCE_PROPERTY;
    pub const REVERSE_DIRECTION_PENALTY: &'static LazyLock<Property<f64>> =
        &REVERSE_DIRECTION_PENALTY_PROPERTY;

    pub const NUDGE_ORTHOGONAL_SEGMENTS_CONNECTED_TO_SHAPES: &'static LazyLock<Property<bool>> =
        &NUDGE_ORTHOGONAL_SEGMENTS_CONNECTED_TO_SHAPES_PROPERTY;
    pub const IMPROVE_HYPEREDGE_ROUTES_MOVING_JUNCTIONS: &'static LazyLock<Property<bool>> =
        &IMPROVE_HYPEREDGE_ROUTES_MOVING_JUNCTIONS_PROPERTY;
    pub const PENALISE_ORTHOGONAL_SHARED_PATHS_AT_CONN_ENDS: &'static LazyLock<Property<bool>> =
        &PENALISE_ORTHOGONAL_SHARED_PATHS_AT_CONN_ENDS_PROPERTY;
    pub const NUDGE_ORTHOGONAL_TOUCHING_COLINEAR_SEGMENTS: &'static LazyLock<Property<bool>> =
        &NUDGE_ORTHOGONAL_TOUCHING_COLINEAR_SEGMENTS_PROPERTY;
    pub const PERFORM_UNIFYING_NUDGING_PREPROCESSING_STEP: &'static LazyLock<Property<bool>> =
        &PERFORM_UNIFYING_NUDGING_PREPROCESSING_STEP_PROPERTY;
    pub const IMPROVE_HYPEREDGE_ROUTES_MOVING_ADDING_AND_DELETING_JUNCTIONS:
        &'static LazyLock<Property<bool>> =
        &IMPROVE_HYPEREDGE_ROUTES_MOVING_ADDING_AND_DELETING_JUNCTIONS_PROPERTY;
    pub const NUDGE_SHARED_PATHS_WITH_COMMON_END_POINT: &'static LazyLock<Property<bool>> =
        &NUDGE_SHARED_PATHS_WITH_COMMON_END_POINT_PROPERTY;
    pub const ENABLE_HYPEREDGES_FROM_COMMON_SOURCE: &'static LazyLock<Property<bool>> =
        &ENABLE_HYPEREDGES_FROM_COMMON_SOURCE_PROPERTY;
    pub const IS_CLUSTER: &'static LazyLock<Property<bool>> = &IS_CLUSTER_PROPERTY;
    pub const PROCESS_TIMEOUT: &'static LazyLock<Property<i32>> = &PROCESS_TIMEOUT_PROPERTY;

    pub const DEBUG_MODE: &'static LazyLock<Property<bool>> = CoreOptions::DEBUG_MODE;
    pub const PORT_SIDE: &'static LazyLock<Property<PortSide>> = CoreOptions::PORT_SIDE;
    pub const DIRECTION: &'static LazyLock<Property<Direction>> = CoreOptions::DIRECTION;
    pub const EDGE_ROUTING: &'static LazyLock<Property<EdgeRouting>> = CoreOptions::EDGE_ROUTING;
    pub const PORT_CONSTRAINTS: &'static LazyLock<Property<PortConstraints>> =
        CoreOptions::PORT_CONSTRAINTS;
    pub const OMIT_NODE_MICRO_LAYOUT: &'static LazyLock<Property<bool>> =
        CoreOptions::OMIT_NODE_MICRO_LAYOUT;
}
