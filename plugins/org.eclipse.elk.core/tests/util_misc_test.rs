use org_eclipse_elk_core::org::eclipse::elk::core::util::{
    DefaultFactory, IFactory, Maybe, Quadruple, Triple, WrappedException,
};

#[test]
fn default_factory_creates_defaults() {
    let factory = DefaultFactory::<i32>::new();
    assert_eq!(factory.create(), 0);
}

#[test]
fn maybe_basic_usage() {
    let mut maybe = Maybe::<i32>::new();
    assert!(maybe.is_empty());
    assert_eq!(maybe.get(), None);

    maybe.set(7);
    assert_eq!(maybe.get(), Some(&7));

    let collected: Vec<_> = maybe.iter().copied().collect();
    assert_eq!(collected, vec![7]);

    maybe.clear();
    assert!(maybe.is_empty());
}

#[test]
fn maybe_into_iter() {
    let maybe = Maybe::with(String::from("hi"));
    let collected: Vec<_> = maybe.into_iter().collect();
    assert_eq!(collected, vec!["hi".to_string()]);
}

#[test]
fn triple_and_quadruple_accessors() {
    let triple = Triple::new(1, "two", 3.0_f64);
    assert_eq!(*triple.first(), 1);
    assert_eq!(*triple.second(), "two");
    assert!((*triple.third() - 3.0_f64).abs() < f64::EPSILON);

    let quadruple = Quadruple::new(1, 2, 3, 4);
    assert_eq!(*quadruple.first(), 1);
    assert_eq!(*quadruple.second(), 2);
    assert_eq!(*quadruple.third(), 3);
    assert_eq!(*quadruple.fourth(), 4);
}

#[test]
fn wrapped_exception_formats_message() {
    let io_error = std::io::Error::other("cause");
    let wrapped = WrappedException::with_message("boom", io_error);
    let message = format!("{wrapped}");
    assert!(message.contains("boom"));
}
