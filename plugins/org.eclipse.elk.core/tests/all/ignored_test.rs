#[test]
fn ignored_test_success() {
    assert!(std::env::var_os("ELK_IGNORE_TEST_MARKER").is_none());
}

#[test]
#[ignore]
fn ignored_test_fail() {
    panic!("ignored test should not execute");
}
