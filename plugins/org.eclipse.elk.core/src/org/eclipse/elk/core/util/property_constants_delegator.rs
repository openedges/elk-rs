use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::{MapPropertyHolder, Property};
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::core::data::{
    LayoutAlgorithmData, LayoutMetaDataService, LayoutOptionData, LayoutOptionType,
};
use crate::org::eclipse::elk::core::labels::ILabelManager;
use crate::org::eclipse::elk::core::math::{ElkMargin, ElkPadding, KVector, KVectorChain};
use crate::org::eclipse::elk::core::options::{
    Alignment, ContentAlignment, Direction, EdgeCoords, EdgeLabelPlacement, EdgeRouting, EdgeType,
    HierarchyHandling, ITopdownSizeApproximator, LabelSide, NodeLabelPlacement, PackingMode,
    PortAlignment, PortConstraints, PortLabelPlacement, PortSide, ShapeCoords, SizeConstraint,
    SizeOptions, TopdownNodeTypes,
};
use crate::org::eclipse::elk::core::util::adapters::{GraphElementAdapter, NodeAdapter};
use crate::org::eclipse::elk::core::util::{EnumSet, IndividualSpacings};

pub struct PropertyConstantsDelegator {
    property_delegates: HashMap<String, Box<dyn Any + Send + Sync>>,
}

impl PropertyConstantsDelegator {
    pub fn create_empty() -> Self {
        PropertyConstantsDelegator {
            property_delegates: HashMap::new(),
        }
    }

    pub fn create_for_layout_algorithm_data(algorithm_data: &LayoutAlgorithmData) -> Self {
        let mut delegator = PropertyConstantsDelegator::create_empty();
        let meta_service = LayoutMetaDataService::get_instance();

        for option_id in algorithm_data.known_option_ids() {
            let Some(option_data) = meta_service.get_option_data(option_id) else {
                continue;
            };
            let default_value = algorithm_data.default_value_any(option_id);
            if let Some(delegate) = build_property_delegate(&option_data, default_value) {
                delegator
                    .property_delegates
                    .insert(option_id.to_string(), delegate);
            }
        }

        delegator
    }

    pub fn create_for_layout_algorithm_with_id(algorithm_id: &str) -> Self {
        let algorithm_data = LayoutMetaDataService::get_instance().get_algorithm_data(algorithm_id);
        if let Some(data) = algorithm_data {
            Self::create_for_layout_algorithm_data(&data)
        } else {
            Self::create_empty()
        }
    }

    pub fn add_delegate<T: Clone + Send + Sync + 'static>(
        &mut self,
        delegate: Property<T>,
    ) -> &mut Self {
        self.property_delegates
            .insert(delegate.id().to_string(), Box::new(delegate));
        self
    }

    pub fn get_property_or_delegate<T: Clone + Send + Sync + 'static>(
        &self,
        property: &'static Property<T>,
    ) -> &Property<T> {
        if let Some(delegate) = self.property_delegates.get(property.id()) {
            if let Some(property) = delegate.downcast_ref::<Property<T>>() {
                return property;
            }
        }
        property
    }

    pub fn get_property<T: Clone + Send + Sync + 'static>(
        &self,
        property_holder: &mut MapPropertyHolder,
        property: &'static Property<T>,
    ) -> Option<T> {
        property_holder.get_property(self.get_property_or_delegate(property))
    }

    pub fn get_property_from_adapter<T, A, E>(
        &self,
        adapter: &A,
        property: &'static Property<T>,
    ) -> Option<T>
    where
        T: Clone + Send + Sync + 'static,
        A: GraphElementAdapter<E>,
    {
        adapter.get_property(self.get_property_or_delegate(property))
    }

    pub fn get_individual_or_inherited_property<T: Clone + Send + Sync + 'static>(
        &self,
        node: &ElkNodeRef,
        property: &'static Property<T>,
    ) -> Option<T> {
        IndividualSpacings::get_individual_or_inherited(
            node,
            self.get_property_or_delegate(property),
        )
    }

    pub fn get_individual_or_inherited_property_adapter<T, N, U>(
        &self,
        node: &N,
        property: &'static Property<T>,
    ) -> Option<T>
    where
        T: Clone + Send + Sync + 'static,
        U: 'static,
        N: NodeAdapter<U>,
        N::Graph: GraphElementAdapter<U>,
    {
        IndividualSpacings::get_individual_or_inherited_adapter(
            node,
            self.get_property_or_delegate(property),
        )
    }
}

fn build_property_delegate(
    option_data: &LayoutOptionData,
    default_value: Option<std::sync::Arc<dyn Any + Send + Sync>>,
) -> Option<Box<dyn Any + Send + Sync>> {
    let id = option_data.id();
    match option_data.option_type() {
        LayoutOptionType::Boolean => {
            Some(Box::new(property_with_default::<bool>(id, default_value)))
        }
        LayoutOptionType::Int => Some(Box::new(property_with_default::<i32>(id, default_value))),
        LayoutOptionType::Double => Some(Box::new(property_with_default::<f64>(id, default_value))),
        LayoutOptionType::String => {
            Some(Box::new(property_with_default::<String>(id, default_value)))
        }
        LayoutOptionType::Enum | LayoutOptionType::EnumSet | LayoutOptionType::Object => {
            let type_id = option_data.value_type_id()?;
            Some(match_type_id(id, type_id, default_value)?)
        }
        LayoutOptionType::Undefined => None,
    }
}

fn match_type_id(
    id: &str,
    type_id: TypeId,
    default_value: Option<std::sync::Arc<dyn Any + Send + Sync>>,
) -> Option<Box<dyn Any + Send + Sync>> {
    if type_id == TypeId::of::<Alignment>() {
        return Some(Box::new(property_with_default::<Alignment>(
            id,
            default_value,
        )));
    }
    if type_id == TypeId::of::<ContentAlignment>() {
        return Some(Box::new(property_with_default::<ContentAlignment>(
            id,
            default_value,
        )));
    }
    if type_id == TypeId::of::<Direction>() {
        return Some(Box::new(property_with_default::<Direction>(
            id,
            default_value,
        )));
    }
    if type_id == TypeId::of::<LayoutAlgorithmData>() {
        return Some(Box::new(property_with_default::<LayoutAlgorithmData>(
            id,
            default_value,
        )));
    }
    if type_id == TypeId::of::<Arc<dyn ILabelManager>>() {
        return Some(Box::new(property_with_default::<Arc<dyn ILabelManager>>(
            id,
            default_value,
        )));
    }
    if type_id == TypeId::of::<Arc<dyn ITopdownSizeApproximator>>() {
        return Some(Box::new(property_with_default::<
            Arc<dyn ITopdownSizeApproximator>,
        >(id, default_value)));
    }
    if type_id == TypeId::of::<EdgeCoords>() {
        return Some(Box::new(property_with_default::<EdgeCoords>(
            id,
            default_value,
        )));
    }
    if type_id == TypeId::of::<EdgeLabelPlacement>() {
        return Some(Box::new(property_with_default::<EdgeLabelPlacement>(
            id,
            default_value,
        )));
    }
    if type_id == TypeId::of::<EdgeRouting>() {
        return Some(Box::new(property_with_default::<EdgeRouting>(
            id,
            default_value,
        )));
    }
    if type_id == TypeId::of::<EdgeType>() {
        return Some(Box::new(property_with_default::<EdgeType>(
            id,
            default_value,
        )));
    }
    if type_id == TypeId::of::<HierarchyHandling>() {
        return Some(Box::new(property_with_default::<HierarchyHandling>(
            id,
            default_value,
        )));
    }
    if type_id == TypeId::of::<LabelSide>() {
        return Some(Box::new(property_with_default::<LabelSide>(
            id,
            default_value,
        )));
    }
    if type_id == TypeId::of::<NodeLabelPlacement>() {
        return Some(Box::new(property_with_default::<NodeLabelPlacement>(
            id,
            default_value,
        )));
    }
    if type_id == TypeId::of::<PackingMode>() {
        return Some(Box::new(property_with_default::<PackingMode>(
            id,
            default_value,
        )));
    }
    if type_id == TypeId::of::<PortAlignment>() {
        return Some(Box::new(property_with_default::<PortAlignment>(
            id,
            default_value,
        )));
    }
    if type_id == TypeId::of::<PortConstraints>() {
        return Some(Box::new(property_with_default::<PortConstraints>(
            id,
            default_value,
        )));
    }
    if type_id == TypeId::of::<PortLabelPlacement>() {
        return Some(Box::new(property_with_default::<PortLabelPlacement>(
            id,
            default_value,
        )));
    }
    if type_id == TypeId::of::<PortSide>() {
        return Some(Box::new(property_with_default::<PortSide>(
            id,
            default_value,
        )));
    }
    if type_id == TypeId::of::<ShapeCoords>() {
        return Some(Box::new(property_with_default::<ShapeCoords>(
            id,
            default_value,
        )));
    }
    if type_id == TypeId::of::<SizeConstraint>() {
        return Some(Box::new(property_with_default::<SizeConstraint>(
            id,
            default_value,
        )));
    }
    if type_id == TypeId::of::<SizeOptions>() {
        return Some(Box::new(property_with_default::<SizeOptions>(
            id,
            default_value,
        )));
    }
    if type_id == TypeId::of::<TopdownNodeTypes>() {
        return Some(Box::new(property_with_default::<TopdownNodeTypes>(
            id,
            default_value,
        )));
    }
    if type_id == TypeId::of::<ElkMargin>() {
        return Some(Box::new(property_with_default::<ElkMargin>(
            id,
            default_value,
        )));
    }
    if type_id == TypeId::of::<ElkPadding>() {
        return Some(Box::new(property_with_default::<ElkPadding>(
            id,
            default_value,
        )));
    }
    if type_id == TypeId::of::<KVector>() {
        return Some(Box::new(property_with_default::<KVector>(
            id,
            default_value,
        )));
    }
    if type_id == TypeId::of::<KVectorChain>() {
        return Some(Box::new(property_with_default::<KVectorChain>(
            id,
            default_value,
        )));
    }
    if type_id == TypeId::of::<IndividualSpacings>() {
        return Some(Box::new(property_with_default::<IndividualSpacings>(
            id,
            default_value,
        )));
    }
    if type_id == TypeId::of::<EnumSet<SizeConstraint>>() {
        return Some(Box::new(property_with_default::<EnumSet<SizeConstraint>>(
            id,
            default_value,
        )));
    }
    if type_id == TypeId::of::<EnumSet<ContentAlignment>>() {
        return Some(Box::new(
            property_with_default::<EnumSet<ContentAlignment>>(id, default_value),
        ));
    }
    if type_id == TypeId::of::<EnumSet<SizeOptions>>() {
        return Some(Box::new(property_with_default::<EnumSet<SizeOptions>>(
            id,
            default_value,
        )));
    }
    if type_id == TypeId::of::<EnumSet<NodeLabelPlacement>>() {
        return Some(Box::new(
            property_with_default::<EnumSet<NodeLabelPlacement>>(id, default_value),
        ));
    }
    if type_id == TypeId::of::<EnumSet<PortLabelPlacement>>() {
        return Some(Box::new(
            property_with_default::<EnumSet<PortLabelPlacement>>(id, default_value),
        ));
    }
    None
}

fn property_with_default<T: Clone + Send + Sync + 'static>(
    id: &str,
    default_value: Option<std::sync::Arc<dyn Any + Send + Sync>>,
) -> Property<T> {
    if let Some(default_value) = default_value.as_ref() {
        if let Some(typed) = default_value.downcast_ref::<T>() {
            return Property::with_default(id, typed.clone());
        }
    }
    Property::new(id)
}
