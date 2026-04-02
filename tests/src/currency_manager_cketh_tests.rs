use currency::{types::currency::CKTokenSymbol, Currency};

use crate::currency_manager_test_support::{
    assert_approve_allowance, assert_deposit_get_balance_and_withdraw, assert_get_fee,
    assert_validate_allowance, CurrencyCase,
};

const CKETH_CASE: CurrencyCase = CurrencyCase {
    currency: Currency::CKETHToken(CKTokenSymbol::ETH),
    user_initial_balance: 100_000_000_000_000,
    deposit_amount: 10_000_000_000_000,
    approval_amount: 20_000_000_000_000,
    canister_approval_amount: 5_000_000_000_000,
};

#[test]
fn currency_manager_cketh_get_fee_returns_default_fee() {
    assert_get_fee(CKETH_CASE, "currency_manager_cketh_get_fee_returns_default_fee");
}

#[test]
fn currency_manager_cketh_validate_allowance_requires_user_approval() {
    assert_validate_allowance(
        CKETH_CASE,
        "currency_manager_cketh_validate_allowance_requires_user_approval",
    );
}

#[test]
fn currency_manager_cketh_deposit_get_balance_and_withdraw_happy_path() {
    assert_deposit_get_balance_and_withdraw(
        CKETH_CASE,
        "currency_manager_cketh_deposit_get_balance_and_withdraw_happy_path",
    );
}

#[test]
fn currency_manager_cketh_approve_allowance_sets_canister_owned_allowance() {
    assert_approve_allowance(
        CKETH_CASE,
        "currency_manager_cketh_approve_allowance_sets_canister_owned_allowance",
    );
}
