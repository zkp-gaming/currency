use currency::{currency_error::CurrencyError, Currency};

use crate::{
    env::new_test_env,
    utils::{
        allowance_of, balance_of, default_subaccount, fee_for_currency, fund_principal,
        manager_approve_allowance, manager_canister_account, manager_deposit, manager_get_balance,
        manager_get_fee, manager_validate_allowance, manager_withdraw, approve_spender,
        test_principal,
    },
};

#[derive(Clone, Copy)]
pub struct CurrencyCase {
    pub currency: Currency,
    pub user_initial_balance: u64,
    pub deposit_amount: u64,
    pub approval_amount: u128,
    pub canister_approval_amount: u128,
}

fn recipient_subaccount(currency: Currency) -> Option<Vec<u8>> {
    match currency {
        Currency::ICP | Currency::TestICP => None,
        Currency::BTC | Currency::CKETHToken(_) => Some(default_subaccount()),
        Currency::GenericICRC1(_) => unreachable!("generic currencies are not covered here"),
    }
}

pub fn assert_get_fee(case: CurrencyCase, label: &str) {
    let env = new_test_env();
    let result = manager_get_fee(env, case.currency);

    assert_eq!(result.unwrap(), fee_for_currency(case.currency), "{label}");
}

pub fn assert_validate_allowance(case: CurrencyCase, label: &str) {
    let env = new_test_env();
    let user = test_principal(&format!("{label}-user"));

    fund_principal(env, case.currency, user, case.user_initial_balance);

    let before_approval = manager_validate_allowance(env, case.currency, user, case.deposit_amount);
    assert_eq!(
        before_approval,
        Err(CurrencyError::InsufficientAllowance),
        "{label}"
    );

    approve_spender(
        env,
        case.currency,
        user,
        env.canister_ids.currency_manager_host,
        case.approval_amount,
    );

    let after_approval = manager_validate_allowance(env, case.currency, user, case.deposit_amount);
    assert_eq!(after_approval, Ok(()), "{label}");
}

pub fn assert_deposit_get_balance_and_withdraw(case: CurrencyCase, label: &str) {
    let env = new_test_env();
    let user = test_principal(&format!("{label}-user"));
    let recipient = test_principal(&format!("{label}-recipient"));

    let manager_account = manager_canister_account(env, case.currency);
    let starting_manager_balance = balance_of(
        env,
        case.currency,
        manager_account.owner,
        manager_account.subaccount.clone(),
    );

    fund_principal(env, case.currency, user, case.user_initial_balance);
    approve_spender(
        env,
        case.currency,
        user,
        env.canister_ids.currency_manager_host,
        case.approval_amount,
    );

    let deposit_result = manager_deposit(env, case.currency, user, case.deposit_amount);
    assert_eq!(deposit_result, Ok(()), "{label}");

    let ledger_balance = balance_of(
        env,
        case.currency,
        manager_account.owner,
        manager_account.subaccount.clone(),
    );
    assert_eq!(
        ledger_balance,
        starting_manager_balance + case.deposit_amount as u128,
        "{label}"
    );

    let manager_balance = manager_get_balance(env, case.currency);
    assert_eq!(
        manager_balance.unwrap(),
        starting_manager_balance + case.deposit_amount as u128,
        "{label}"
    );

    let recipient_before = balance_of(
        env,
        case.currency,
        recipient,
        recipient_subaccount(case.currency),
    );

    let withdraw_result = manager_withdraw(env, case.currency, recipient, case.deposit_amount);
    assert_eq!(withdraw_result, Ok(()), "{label}");

    let manager_balance_after = manager_get_balance(env, case.currency);
    assert_eq!(manager_balance_after.unwrap(), starting_manager_balance, "{label}");

    let recipient_after = balance_of(
        env,
        case.currency,
        recipient,
        recipient_subaccount(case.currency),
    );
    assert_eq!(
        recipient_after - recipient_before,
        case.deposit_amount as u128 - fee_for_currency(case.currency),
        "{label}"
    );
}

pub fn assert_approve_allowance(case: CurrencyCase, label: &str) {
    let env = new_test_env();
    let user = test_principal(&format!("{label}-user"));
    let spender = test_principal(&format!("{label}-spender"));

    let starting_manager_balance = manager_get_balance(env, case.currency).unwrap();

    fund_principal(env, case.currency, user, case.user_initial_balance);
    approve_spender(
        env,
        case.currency,
        user,
        env.canister_ids.currency_manager_host,
        case.approval_amount,
    );

    let deposit_result = manager_deposit(env, case.currency, user, case.deposit_amount);
    assert_eq!(deposit_result, Ok(()), "{label}");

    let pre_approval_balance = manager_get_balance(env, case.currency).unwrap();
    assert_eq!(
        pre_approval_balance,
        starting_manager_balance + case.deposit_amount as u128,
        "{label}"
    );

    let approve_result = manager_approve_allowance(
        env,
        case.currency,
        spender,
        None,
        case.canister_approval_amount,
    );
    assert_eq!(approve_result, Ok(()), "{label}");

    let allowance = allowance_of(env, case.currency, env.canister_ids.currency_manager_host, spender);
    assert_eq!(allowance.allowance, case.canister_approval_amount, "{label}");

    let post_approval_balance = manager_get_balance(env, case.currency).unwrap();
    assert_eq!(
        post_approval_balance,
        starting_manager_balance + case.deposit_amount as u128 - fee_for_currency(case.currency),
        "{label}"
    );
}
