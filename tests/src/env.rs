use lazy_static::lazy_static;

use crate::TestEnv;

lazy_static! {
    static ref SHARED_TEST_ENV: TestEnv = TestEnv::default();
}

pub fn new_test_env() -> &'static TestEnv {
    &SHARED_TEST_ENV
}
