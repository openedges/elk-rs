use std::any::{Any, TypeId};
use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::data::{LayoutMetaDataService, LayoutOptionType};
use org_eclipse_elk_core::org::eclipse::elk::core::math::{ElkMargin, ElkPadding, KVector, KVectorChain};
use org_eclipse_elk_core::org::eclipse::elk::core::util::IndividualSpacings;

#[test]
fn parseable_object_options_roundtrip_default_or_sample() {
    let service = LayoutMetaDataService::get_instance();

    for option in service.get_option_data_list() {
        if option.option_type() != LayoutOptionType::Object || !option.can_parse_value() {
            continue;
        }

        let default_value = option.default_value();
        let mut sample_from_default = false;
        let sample = if let Some(serialized_default) = default_value
            .as_ref()
            .and_then(serialize_known_object)
            .filter(|value| !value.trim().is_empty())
        {
            sample_from_default = true;
            serialized_default
        } else if let Some(fallback_sample) = sample_for_object_type(option.value_type_id()) {
            fallback_sample.to_string()
        } else {
            panic!(
                "parseable object option '{}' has no serializable default and no fallback sample",
                option.id()
            );
        };

        let parsed = option.parse_value(&sample).unwrap_or_else(|| {
            panic!(
                "object option '{}' failed to parse sample '{}'",
                option.id(),
                sample
            )
        });

        if let Some(expected_type) = option.value_type_id() {
            assert_eq!(
                parsed.as_ref().type_id(),
                expected_type,
                "object option '{}' parsed into unexpected type",
                option.id()
            );
        }

        if sample_from_default {
            if let Some(default_value) = default_value.as_ref() {
                assert_roundtrip_matches_default(option.id(), default_value, &parsed);
            }
        }
    }
}

fn serialize_known_object(value: &Arc<dyn Any + Send + Sync>) -> Option<String> {
    if let Some(value) = value.as_ref().downcast_ref::<KVector>() {
        return Some(format!("({},{})", value.x, value.y));
    }
    if let Some(value) = value.as_ref().downcast_ref::<KVectorChain>() {
        return Some(value.to_string());
    }
    if let Some(value) = value.as_ref().downcast_ref::<ElkPadding>() {
        return Some(format!(
            "[top={},left={},bottom={},right={}]",
            value.top, value.left, value.bottom, value.right
        ));
    }
    if let Some(value) = value.as_ref().downcast_ref::<ElkMargin>() {
        return Some(format!(
            "[top={},left={},bottom={},right={}]",
            value.top, value.left, value.bottom, value.right
        ));
    }
    if let Some(value) = value.as_ref().downcast_ref::<IndividualSpacings>() {
        return Some(value.to_string());
    }
    None
}

fn sample_for_object_type(value_type_id: Option<TypeId>) -> Option<&'static str> {
    if value_type_id == Some(TypeId::of::<KVector>()) {
        return Some("(1.5,2.5)");
    }
    if value_type_id == Some(TypeId::of::<KVectorChain>()) {
        return Some("(1,2;3,4)");
    }
    if value_type_id == Some(TypeId::of::<ElkPadding>())
        || value_type_id == Some(TypeId::of::<ElkMargin>())
    {
        return Some("[top=1,left=2,bottom=3,right=4]");
    }
    if value_type_id == Some(TypeId::of::<IndividualSpacings>()) {
        return Some("nodeNode:10");
    }
    None
}

fn assert_roundtrip_matches_default(
    option_id: &str,
    default_value: &Arc<dyn Any + Send + Sync>,
    parsed_value: &Arc<dyn Any + Send + Sync>,
) {
    if let (Some(default), Some(parsed)) = (
        default_value.as_ref().downcast_ref::<KVector>(),
        parsed_value.as_ref().downcast_ref::<KVector>(),
    ) {
        assert_eq!(
            default, parsed,
            "object option '{}' roundtrip mismatch for KVector default",
            option_id
        );
        return;
    }
    if let (Some(default), Some(parsed)) = (
        default_value.as_ref().downcast_ref::<KVectorChain>(),
        parsed_value.as_ref().downcast_ref::<KVectorChain>(),
    ) {
        assert_eq!(
            default, parsed,
            "object option '{}' roundtrip mismatch for KVectorChain default",
            option_id
        );
        return;
    }
    if let (Some(default), Some(parsed)) = (
        default_value.as_ref().downcast_ref::<ElkPadding>(),
        parsed_value.as_ref().downcast_ref::<ElkPadding>(),
    ) {
        assert_eq!(
            default, parsed,
            "object option '{}' roundtrip mismatch for ElkPadding default",
            option_id
        );
        return;
    }
    if let (Some(default), Some(parsed)) = (
        default_value.as_ref().downcast_ref::<ElkMargin>(),
        parsed_value.as_ref().downcast_ref::<ElkMargin>(),
    ) {
        assert_eq!(
            default, parsed,
            "object option '{}' roundtrip mismatch for ElkMargin default",
            option_id
        );
        return;
    }
    if let (Some(default), Some(parsed)) = (
        default_value.as_ref().downcast_ref::<IndividualSpacings>(),
        parsed_value.as_ref().downcast_ref::<IndividualSpacings>(),
    ) {
        assert_eq!(
            default.to_string(),
            parsed.to_string(),
            "object option '{}' roundtrip mismatch for IndividualSpacings default",
            option_id
        );
    }
}
