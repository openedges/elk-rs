use super::test_controller::TestController;

pub trait IWhiteBoxTestable {
    fn set_test_controller(&mut self, controller: Option<*mut TestController>);
}
