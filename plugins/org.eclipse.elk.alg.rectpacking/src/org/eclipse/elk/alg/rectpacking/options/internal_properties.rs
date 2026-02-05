use std::sync::LazyLock;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

pub struct InternalProperties;

pub static ADDITIONAL_HEIGHT_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::new("additionalHeight"));
pub static DRAWING_HEIGHT_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::new("drawingHeight"));
pub static DRAWING_WIDTH_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::new("drawingWidth"));

pub static MIN_HEIGHT_PROPERTY: LazyLock<Property<f64>> = LazyLock::new(|| Property::new("minHeight"));
pub static MIN_WIDTH_PROPERTY: LazyLock<Property<f64>> = LazyLock::new(|| Property::new("minWidth"));

pub static ROWS_PROPERTY: LazyLock<Property<usize>> =
    LazyLock::new(|| Property::new("rows"));

pub static TARGET_WIDTH_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::new("targetWidth"));

pub static MIN_ROW_INCREASE_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("minRowIncrease", 0.0));
pub static MAX_ROW_INCREASE_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("maxRowIncrease", 0.0));

pub static MIN_ROW_DECREASE_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("minRowDecrease", 0.0));
pub static MAX_ROW_DECREASE_PROPERTY: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default("maxRowDecrease", 0.0));

impl InternalProperties {
    pub const ADDITIONAL_HEIGHT: &'static LazyLock<Property<f64>> = &ADDITIONAL_HEIGHT_PROPERTY;
    pub const DRAWING_HEIGHT: &'static LazyLock<Property<f64>> = &DRAWING_HEIGHT_PROPERTY;
    pub const DRAWING_WIDTH: &'static LazyLock<Property<f64>> = &DRAWING_WIDTH_PROPERTY;

    pub const MIN_HEIGHT: &'static LazyLock<Property<f64>> = &MIN_HEIGHT_PROPERTY;
    pub const MIN_WIDTH: &'static LazyLock<Property<f64>> = &MIN_WIDTH_PROPERTY;

    pub const ROWS: &'static LazyLock<Property<usize>> = &ROWS_PROPERTY;

    pub const TARGET_WIDTH: &'static LazyLock<Property<f64>> = &TARGET_WIDTH_PROPERTY;

    pub const MIN_ROW_INCREASE: &'static LazyLock<Property<f64>> = &MIN_ROW_INCREASE_PROPERTY;
    pub const MAX_ROW_INCREASE: &'static LazyLock<Property<f64>> = &MAX_ROW_INCREASE_PROPERTY;
    pub const MIN_ROW_DECREASE: &'static LazyLock<Property<f64>> = &MIN_ROW_DECREASE_PROPERTY;
    pub const MAX_ROW_DECREASE: &'static LazyLock<Property<f64>> = &MAX_ROW_DECREASE_PROPERTY;
}
