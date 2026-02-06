use std::any::Any;
use std::sync::{Arc, LazyLock};

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::{GraphFeature, Property};

use org_eclipse_elk_core::org::eclipse::elk::core::data::{
    ILayoutMetaDataProvider, LayoutAlgorithmData, LayoutMetaDataRegistry,
};
use org_eclipse_elk_core::org::eclipse::elk::core::math::ElkPadding;
use org_eclipse_elk_core::org::eclipse::elk::core::options::{CoreOptions, Direction};
use org_eclipse_elk_core::org::eclipse::elk::core::util::{AlgorithmFactory, InstancePool};

use crate::org::eclipse::elk::conn::gmf::layouter::draw2d_layout_provider::Draw2DLayoutProvider;
use crate::org::eclipse::elk::conn::gmf::layouter::draw2d_options::{
    Draw2DOptions, NODE_SIZE_CONSTRAINTS,
};

pub struct Draw2DMetaDataProvider;

impl ILayoutMetaDataProvider for Draw2DMetaDataProvider {
    fn apply(&self, registry: &mut dyn LayoutMetaDataRegistry) {
        register_algorithm(registry);
        register_supports(registry);
    }
}

fn register_algorithm(registry: &mut dyn LayoutMetaDataRegistry) {
    let factory = AlgorithmFactory::new(|| Box::new(Draw2DLayoutProvider::new()));
    let pool = InstancePool::new(Box::new(factory));

    let mut data = LayoutAlgorithmData::new(Draw2DOptions::ALGORITHM_ID)
        .with_provider_pool(Arc::new(pool));
    data.set_name("Draw2D Layout")
        .set_description(
            "'Directed Graph Layout' provided by the Draw2D framework. This is the same algorithm that is used by the standard layout button of GMF diagrams.",
        )
        .set_category_id(Some("org.eclipse.elk.layered"))
        .set_bundle_name(Some("GMF"))
        .set_defining_bundle_id(Some("org.eclipse.elk.conn.gmf"))
        .set_preview_image_path(Some("images/draw2d.png"))
        .add_supported_feature(GraphFeature::MultiEdges);
    registry.register_algorithm(data);
}

fn register_supports(registry: &mut dyn LayoutMetaDataRegistry) {
    let algo = Draw2DOptions::ALGORITHM_ID;
    registry.add_option_support(algo, CoreOptions::SPACING_NODE_NODE.id(), arc_any(16.0_f64));
    registry.add_option_support(
        algo,
        CoreOptions::PADDING.id(),
        arc_any(ElkPadding::with_any(16.0_f64)),
    );
    registry.add_option_support(algo, CoreOptions::DIRECTION.id(), arc_any(Direction::Right));
    registry.add_option_support(
        algo,
        CoreOptions::NODE_SIZE_CONSTRAINTS.id(),
        property_default_any(NODE_SIZE_CONSTRAINTS),
    );
}

fn arc_any<T: Any + Send + Sync>(value: T) -> Option<Arc<dyn Any + Send + Sync>> {
    Some(Arc::new(value))
}

fn property_default_any<T: Clone + Send + Sync + 'static>(
    property: &'static LazyLock<Property<T>>,
) -> Option<Arc<dyn Any + Send + Sync>> {
    if !property.is_cloneable() {
        return None;
    }
    property
        .get_default()
        .map(|value| Arc::new(value) as Arc<dyn Any + Send + Sync>)
}
