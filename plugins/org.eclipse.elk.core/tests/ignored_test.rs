#[test]
fn ignored_test_success() {
    assert!(true);
}

#[test]
#[ignore]
fn ignored_test_fail() {
    assert!(false, "ignored test should not execute");
}
