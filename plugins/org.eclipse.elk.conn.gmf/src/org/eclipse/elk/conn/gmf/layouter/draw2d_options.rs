use std::sync::LazyLock;

use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{
    CoreOptions, Direction, SizeConstraint,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

pub static SPACING_NODE_NODE: LazyLock<Property<f64>> =
    LazyLock::new(|| Property::with_default(CoreOptions::SPACING_NODE_NODE.id(), 16.0_f64));

pub static PADDING: LazyLock<Property<ElkPadding>> =
    LazyLock::new(|| Property::with_default(CoreOptions::PADDING.id(), ElkPadding::with_any(16.0)));

pub static DIRECTION: LazyLock<Property<Direction>> =
    LazyLock::new(|| Property::with_default(CoreOptions::DIRECTION.id(), Direction::Right));

pub static NODE_SIZE_CONSTRAINTS: &LazyLock<Property<EnumSet<SizeConstraint>>> =
    CoreOptions::NODE_SIZE_CONSTRAINTS;

pub struct Draw2DOptions;

impl Draw2DOptions {
    pub const ALGORITHM_ID: &'static str = "org.eclipse.elk.conn.gmf.layouter.Draw2D";
}
