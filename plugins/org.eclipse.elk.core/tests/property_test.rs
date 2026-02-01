use std::collections::{BTreeSet, HashSet, LinkedList};

use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::math::{ElkMargin, ElkPadding, KVector, KVectorChain};
use org_eclipse_elk_core::org::eclipse::elk::core::options::{PortSide, SizeConstraint};
use org_eclipse_elk_core::org::eclipse::elk::core::util::LinkedHashSet;

#[test]
fn test_property_default_primitives() {
    LayoutMetaDataService::get_instance();

    let i = 43;
    test_property_primitive(i);

    let f = 23.3_f32;
    test_property_primitive(f);

    let d = 32.3_f64;
    test_property_primitive(d);

    let s = String::from("foo");
    test_property_primitive(s);
}

#[test]
fn test_property_default_i_data_object() {
    LayoutMetaDataService::get_instance();

    let v = KVector::with_values(2.0, 3.0);
    test_property_object(v);

    let vc = KVectorChain::from_vectors(&[v, v]);
    test_property_object(vc);

    let ep = ElkPadding::with_sides(2.0, 3.0);
    test_property_object(ep);

    let em = ElkMargin::with_sides(3.0, 2.0);
    test_property_object(em);
}

#[test]
fn test_property_default_object() {
    LayoutMetaDataService::get_instance();

    let v = KVector::with_values(3.0, 2.0);
    let mut al = Vec::new();
    let mut normalized = v;
    normalized.normalize();
    let mut negated = v;
    negated.negate();
    al.push(v);
    al.push(normalized);
    al.push(negated);
    test_property_object(al.clone());

    let mut ll = LinkedList::new();
    for item in &al {
        ll.push_back(*item);
    }
    test_property_object(ll);

    let mut hs = HashSet::new();
    for item in &al {
        hs.insert(*item);
    }
    test_property_object(hs);

    let lhs = LinkedHashSet::from_iter(al.clone());
    test_property_object(lhs);

    let mut ts = BTreeSet::new();
    ts.extend([1, 2, 3, 4]);
    test_property_object(ts);
}

#[test]
fn test_property_default_enum() {
    LayoutMetaDataService::get_instance();

    let ps = PortSide::East;
    test_property_primitive(ps);
}

#[test]
fn test_property_default_enum_set() {
    LayoutMetaDataService::get_instance();

    let sc = SizeConstraint::free();
    test_property_object(sc);
}

fn test_property_object<T>(default_value: T)
where
    T: Clone + PartialEq + std::fmt::Debug + Send + Sync + 'static,
{
    let property = Property::with_default("dummy", default_value.clone());
    let copy = property.get_default().expect("default value");
    assert_eq!(default_value, copy);
}

fn test_property_primitive<T>(default_value: T)
where
    T: Clone + PartialEq + std::fmt::Debug + Send + Sync + 'static,
{
    let property = Property::with_default("dummy", default_value.clone());
    let copy = property.get_default().expect("default value");
    assert_eq!(default_value, copy);
}

#[test]
#[should_panic(expected = "Couldn't clone property 'o'")]
fn test_unknown_property_get_default() {
    LayoutMetaDataService::get_instance();

    #[derive(Clone, Debug, PartialEq)]
    struct Dummy;

    let property = Property::with_default("o", Dummy);
    let _ = property.get_default();
}
