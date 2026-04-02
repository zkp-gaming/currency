use currency::{types::currency::CKTokenSymbol, Currency};

use crate::{
    env::new_test_env,
    utils::{
        allowance_of_with_subaccount, approve_spender_with_args, balance_of, default_subaccount,
        fee_for_currency, fund_account, manager_approve_allowance_with_args,
        manager_deposit_with_args, manager_get_balance, manager_validate_allowance_with_args,
        manager_withdraw_with_args, non_default_subaccount, test_principal,
    },
};

const POCKET_IC_LEDGER_TIME_NANOS: u64 = 1_620_328_630_000_000_000;

#[test]
fn currency_manager_icp_deposit_and_validate_allowance_support_source_subaccount_and_metadata() {
    let env = new_test_env();
    let user = test_principal("currency-manager-icp-subaccount-user");
    let user_subaccount = non_default_subaccount(7);
    let deposit_amount = 150_000u64;

    let starting_manager_balance = manager_get_balance(env, Currency::ICP).unwrap();

    fund_account(
        env,
        Currency::ICP,
        user,
        Some(user_subaccount.clone()),
        500_000,
    );
    approve_spender_with_args(
        env,
        Currency::ICP,
        user,
        Some(user_subaccount.clone()),
        env.canister_ids.currency_manager_host,
        200_000,
        Some(vec![1, 2, 3]),
        Some(POCKET_IC_LEDGER_TIME_NANOS),
    );

    let allowance_result = manager_validate_allowance_with_args(
        env,
        Currency::ICP,
        user,
        Some(user_subaccount.clone()),
        deposit_amount,
        Some(vec![9, 9]),
        Some(POCKET_IC_LEDGER_TIME_NANOS + 1),
    );
    assert_eq!(allowance_result, Ok(()));

    let deposit_result = manager_deposit_with_args(
        env,
        Currency::ICP,
        user,
        Some(user_subaccount),
        deposit_amount,
        Some(vec![4, 5, 6]),
        Some(POCKET_IC_LEDGER_TIME_NANOS + 2),
    );
    assert_eq!(deposit_result, Ok(()));

    let ending_manager_balance = manager_get_balance(env, Currency::ICP).unwrap();
    assert_eq!(
        ending_manager_balance,
        starting_manager_balance + deposit_amount as u128
    );
}

#[test]
fn currency_manager_ckusdc_withdraw_and_approve_allowance_support_source_subaccount_and_metadata()
{
    let env = new_test_env();
    let currency = Currency::CKETHToken(CKTokenSymbol::USDC);
    let spender = test_principal("currency-manager-ckusdc-subaccount-spender");
    let recipient = test_principal("currency-manager-ckusdc-subaccount-recipient");
    let canister_source_subaccount = non_default_subaccount(11);
    let withdraw_amount = 100_000u64;
    let approve_amount = 50_000u128;

    fund_account(
        env,
        currency,
        env.canister_ids.currency_manager_host,
        Some(canister_source_subaccount.clone()),
        200_000,
    );

    let recipient_before = balance_of(
        env,
        currency,
        recipient,
        Some(default_subaccount()),
    );

    let withdraw_result = manager_withdraw_with_args(
        env,
        currency,
        recipient,
        Some(canister_source_subaccount.clone()),
        withdraw_amount,
        Some(vec![7, 8, 9]),
        Some(POCKET_IC_LEDGER_TIME_NANOS + 3),
    );
    assert_eq!(withdraw_result, Ok(()));

    let recipient_after = balance_of(
        env,
        currency,
        recipient,
        Some(default_subaccount()),
    );
    assert_eq!(
        recipient_after - recipient_before,
        withdraw_amount as u128 - fee_for_currency(currency)
    );

    let approve_result = manager_approve_allowance_with_args(
        env,
        currency,
        spender,
        Some(canister_source_subaccount.clone()),
        approve_amount,
        Some(vec![10, 11]),
        Some(POCKET_IC_LEDGER_TIME_NANOS + 4),
    );
    assert_eq!(approve_result, Ok(()));

    let allowance = allowance_of_with_subaccount(
        env,
        currency,
        env.canister_ids.currency_manager_host,
        Some(canister_source_subaccount),
        spender,
    );
    assert_eq!(allowance.allowance, approve_amount);
}
