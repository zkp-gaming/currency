use currency::{types::currency::CKSOLTokenSymbol, Currency};

use crate::currency_manager_test_support::{
    assert_approve_allowance, assert_deposit_get_balance_and_withdraw, assert_get_fee,
    assert_validate_allowance, CurrencyCase,
};

const CKDEVNETSOL_CASE: CurrencyCase = CurrencyCase {
    currency: Currency::CKSOLToken(CKSOLTokenSymbol::DevnetSOL),
    user_initial_balance: 5_000_000_000,
    deposit_amount: 1_500_000_000,
    approval_amount: 2_000_000_000,
    canister_approval_amount: 500_000_000,
};

const CKSOL_CASE: CurrencyCase = CurrencyCase {
    currency: Currency::CKSOLToken(CKSOLTokenSymbol::SOL),
    user_initial_balance: 5_000_000_000,
    deposit_amount: 1_500_000_000,
    approval_amount: 2_000_000_000,
    canister_approval_amount: 500_000_000,
};

#[test]
fn currency_manager_ckdevnetsol_get_fee_returns_default_fee() {
    assert_get_fee(
        CKDEVNETSOL_CASE,
        "currency_manager_ckdevnetsol_get_fee_returns_default_fee",
    );
}

#[test]
fn currency_manager_ckdevnetsol_validate_allowance_requires_user_approval() {
    assert_validate_allowance(
        CKDEVNETSOL_CASE,
        "currency_manager_ckdevnetsol_validate_allowance_requires_user_approval",
    );
}

#[test]
fn currency_manager_ckdevnetsol_deposit_get_balance_and_withdraw_happy_path() {
    assert_deposit_get_balance_and_withdraw(
        CKDEVNETSOL_CASE,
        "currency_manager_ckdevnetsol_deposit_get_balance_and_withdraw_happy_path",
    );
}

#[test]
fn currency_manager_ckdevnetsol_approve_allowance_sets_canister_owned_allowance() {
    assert_approve_allowance(
        CKDEVNETSOL_CASE,
        "currency_manager_ckdevnetsol_approve_allowance_sets_canister_owned_allowance",
    );
}

#[test]
fn currency_manager_cksol_get_fee_returns_default_fee() {
    assert_get_fee(CKSOL_CASE, "currency_manager_cksol_get_fee_returns_default_fee");
}

#[test]
fn currency_manager_cksol_validate_allowance_requires_user_approval() {
    assert_validate_allowance(
        CKSOL_CASE,
        "currency_manager_cksol_validate_allowance_requires_user_approval",
    );
}

#[test]
fn currency_manager_cksol_deposit_get_balance_and_withdraw_happy_path() {
    assert_deposit_get_balance_and_withdraw(
        CKSOL_CASE,
        "currency_manager_cksol_deposit_get_balance_and_withdraw_happy_path",
    );
}

#[test]
fn currency_manager_cksol_approve_allowance_sets_canister_owned_allowance() {
    assert_approve_allowance(
        CKSOL_CASE,
        "currency_manager_cksol_approve_allowance_sets_canister_owned_allowance",
    );
}
