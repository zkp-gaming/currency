use currency::{currency_error::CurrencyError, types::currency::CKTokenSymbol, Currency};

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

fn assert_duplicate_transaction(result: Result<(), CurrencyError>) {
    assert!(matches!(
        result,
        Err(CurrencyError::DuplicateTransaction { .. })
    ));
}

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

#[test]
fn currency_manager_icp_deposit_rejects_duplicate_request_and_missing_created_at_time() {
    let env = new_test_env();
    let user = test_principal("currency-manager-icp-duplicate-user");
    let deposit_amount = 150_000u64;
    let created_at_time = POCKET_IC_LEDGER_TIME_NANOS + 10;
    let starting_manager_balance = manager_get_balance(env, Currency::ICP).unwrap();

    fund_account(env, Currency::ICP, user, None, 500_000);
    approve_spender_with_args(
        env,
        Currency::ICP,
        user,
        None,
        env.canister_ids.currency_manager_host,
        200_000,
        Some(vec![1]),
        Some(POCKET_IC_LEDGER_TIME_NANOS + 9),
    );

    let missing_time_result = manager_deposit_with_args(
        env,
        Currency::ICP,
        user,
        None,
        deposit_amount,
        Some(vec![2]),
        None,
    );
    assert_eq!(missing_time_result, Err(CurrencyError::MissingCreatedAtTime));

    let first = manager_deposit_with_args(
        env,
        Currency::ICP,
        user,
        None,
        deposit_amount,
        Some(vec![3]),
        Some(created_at_time),
    );
    assert_eq!(first, Ok(()));

    let second = manager_deposit_with_args(
        env,
        Currency::ICP,
        user,
        None,
        deposit_amount,
        Some(vec![3]),
        Some(created_at_time),
    );
    assert_duplicate_transaction(second);

    let ending_manager_balance = manager_get_balance(env, Currency::ICP).unwrap();
    assert_eq!(
        ending_manager_balance - starting_manager_balance,
        deposit_amount as u128
    );
}

#[test]
fn currency_manager_ckusdc_withdraw_rejects_duplicate_request_and_missing_created_at_time() {
    let env = new_test_env();
    let currency = Currency::CKETHToken(CKTokenSymbol::USDC);
    let recipient = test_principal("currency-manager-ckusdc-duplicate-recipient");
    let withdraw_amount = 100_000u64;
    let created_at_time = POCKET_IC_LEDGER_TIME_NANOS + 20;

    fund_account(
        env,
        currency,
        env.canister_ids.currency_manager_host,
        Some(default_subaccount()),
        200_000,
    );

    let recipient_before = balance_of(env, currency, recipient, Some(default_subaccount()));

    let missing_time_result = manager_withdraw_with_args(
        env,
        currency,
        recipient,
        None,
        withdraw_amount,
        Some(vec![4]),
        None,
    );
    assert_eq!(missing_time_result, Err(CurrencyError::MissingCreatedAtTime));

    let first = manager_withdraw_with_args(
        env,
        currency,
        recipient,
        None,
        withdraw_amount,
        Some(vec![5]),
        Some(created_at_time),
    );
    assert_eq!(first, Ok(()));

    let second = manager_withdraw_with_args(
        env,
        currency,
        recipient,
        None,
        withdraw_amount,
        Some(vec![5]),
        Some(created_at_time),
    );
    assert_duplicate_transaction(second);

    let recipient_after = balance_of(env, currency, recipient, Some(default_subaccount()));
    assert_eq!(
        recipient_after - recipient_before,
        withdraw_amount as u128 - fee_for_currency(currency)
    );
}

#[test]
fn currency_manager_icp_deposit_allows_retry_after_failed_request() {
    let env = new_test_env();
    let user = test_principal("currency-manager-icp-retry-user");
    let created_at_time = POCKET_IC_LEDGER_TIME_NANOS + 30;
    let deposit_amount = 150_000u64;

    fund_account(env, Currency::ICP, user, None, 500_000);

    let first = manager_deposit_with_args(
        env,
        Currency::ICP,
        user,
        None,
        deposit_amount,
        Some(vec![6]),
        Some(created_at_time),
    );
    assert_eq!(first, Err(CurrencyError::InsufficientAllowance));

    approve_spender_with_args(
        env,
        Currency::ICP,
        user,
        None,
        env.canister_ids.currency_manager_host,
        200_000,
        Some(vec![7]),
        Some(POCKET_IC_LEDGER_TIME_NANOS + 31),
    );

    let retry = manager_deposit_with_args(
        env,
        Currency::ICP,
        user,
        None,
        deposit_amount,
        Some(vec![6]),
        Some(created_at_time),
    );
    assert_eq!(retry, Ok(()));
}

#[test]
fn currency_manager_ckusdc_withdraw_allows_retry_after_failed_request() {
    let env = new_test_env();
    let currency = Currency::CKETHToken(CKTokenSymbol::USDC);
    let recipient = test_principal("currency-manager-ckusdc-retry-recipient");
    let created_at_time = POCKET_IC_LEDGER_TIME_NANOS + 40;
    let withdraw_amount = 100_000u64;

    fund_account(
        env,
        currency,
        env.canister_ids.currency_manager_host,
        Some(default_subaccount()),
        50_000,
    );

    let first = manager_withdraw_with_args(
        env,
        currency,
        recipient,
        None,
        withdraw_amount,
        Some(vec![8]),
        Some(created_at_time),
    );
    assert!(first.is_err());

    fund_account(
        env,
        currency,
        env.canister_ids.currency_manager_host,
        Some(default_subaccount()),
        200_000,
    );

    let retry = manager_withdraw_with_args(
        env,
        currency,
        recipient,
        None,
        withdraw_amount,
        Some(vec![8]),
        Some(created_at_time),
    );
    assert_eq!(retry, Ok(()));
}
