use currency::Currency;

use crate::currency_manager_test_support::{
    assert_approve_allowance, assert_deposit_get_balance_and_withdraw, assert_get_fee,
    assert_validate_allowance, CurrencyCase,
};

const TEST_ICP_CASE: CurrencyCase = CurrencyCase {
    currency: Currency::TestICP,
    user_initial_balance: 500_000,
    deposit_amount: 150_000,
    approval_amount: 200_000,
    canister_approval_amount: 50_000,
};

#[test]
fn currency_manager_test_icp_get_fee_returns_default_fee() {
    assert_get_fee(
        TEST_ICP_CASE,
        "currency_manager_test_icp_get_fee_returns_default_fee",
    );
}

#[test]
fn currency_manager_test_icp_validate_allowance_requires_user_approval() {
    assert_validate_allowance(
        TEST_ICP_CASE,
        "currency_manager_test_icp_validate_allowance_requires_user_approval",
    );
}

#[test]
fn currency_manager_test_icp_deposit_get_balance_and_withdraw_happy_path() {
    assert_deposit_get_balance_and_withdraw(
        TEST_ICP_CASE,
        "currency_manager_test_icp_deposit_get_balance_and_withdraw_happy_path",
    );
}

#[test]
fn currency_manager_test_icp_approve_allowance_sets_canister_owned_allowance() {
    assert_approve_allowance(
        TEST_ICP_CASE,
        "currency_manager_test_icp_approve_allowance_sets_canister_owned_allowance",
    );
}
