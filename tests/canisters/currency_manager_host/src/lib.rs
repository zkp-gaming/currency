use std::cell::RefCell;

use candid::{CandidType, Principal};
use currency::{
    cksol_minter_canister_interface::{
        CKSOLMinterInfo, CKSOLWithdrawalStatus, ProcessDepositSuccess, WithdrawToSolSuccess,
    },
    currency_error::CurrencyError,
    state::TransactionState,
    types::{
        currency::Token,
        currency_manager::CurrencyManager,
    },
    utils::get_canister_state,
    Currency,
};
use ic_cdk_macros::{query, update};

thread_local! {
    static TRANSACTION_STATE: RefCell<TransactionState> = RefCell::new(TransactionState::new());
}

#[derive(CandidType)]
pub struct AccountView {
    pub owner: Principal,
    pub subaccount: Option<Vec<u8>>,
}

async fn manager_for_currency(currency: &Currency) -> Result<CurrencyManager, CurrencyError> {
    let mut manager = CurrencyManager::new();

    if matches!(
        currency,
        Currency::CKETHToken(_) | Currency::CKSOLToken(_) | Currency::GenericICRC1(_)
    ) {
        manager.add_currency(*currency).await?;
    }

    Ok(manager)
}

#[update]
async fn deposit(
    currency: Currency,
    from_principal: Principal,
    subaccount: Option<Vec<u8>>,
    amount: u64,
    memo: Option<Vec<u8>>,
    created_at_time: Option<u64>,
) -> Result<(), CurrencyError> {
    let mut transaction_state = TRANSACTION_STATE.with(|state| state.borrow().clone());
    manager_for_currency(&currency)
        .await?
        .deposit(
            &mut transaction_state,
            &currency,
            from_principal,
            subaccount,
            amount,
            memo,
            created_at_time,
        )
        .await?;
    TRANSACTION_STATE.with(|state| *state.borrow_mut() = transaction_state);
    Ok(())
}

#[update]
async fn validate_allowance(
    currency: Currency,
    from_principal: Principal,
    subaccount: Option<Vec<u8>>,
    amount: u64,
    memo: Option<Vec<u8>>,
    created_at_time: Option<u64>,
) -> Result<(), CurrencyError> {
    manager_for_currency(&currency)
        .await?
        .validate_allowance(
            &currency,
            from_principal,
            subaccount,
            amount,
            memo,
            created_at_time,
        )
        .await
}

#[update]
async fn withdraw(
    currency: Currency,
    to_principal: Principal,
    subaccount: Option<Vec<u8>>,
    amount: u64,
    memo: Option<Vec<u8>>,
    created_at_time: Option<u64>,
) -> Result<(), CurrencyError> {
    let mut transaction_state = TRANSACTION_STATE.with(|state| state.borrow().clone());
    manager_for_currency(&currency)
        .await?
        .withdraw(
            &mut transaction_state,
            &currency,
            to_principal,
            subaccount,
            amount,
            memo,
            created_at_time,
        )
        .await?;
    TRANSACTION_STATE.with(|state| *state.borrow_mut() = transaction_state);
    Ok(())
}

#[update]
async fn get_balance(currency: Currency) -> Result<u128, CurrencyError> {
    manager_for_currency(&currency)
        .await?
        .get_balance(&currency, ic_cdk::api::canister_self())
        .await
}

#[update]
async fn get_fee(currency: Currency) -> Result<u128, CurrencyError> {
    manager_for_currency(&currency).await?.get_fee(&currency).await
}

#[update]
async fn approve_allowance(
    currency: Currency,
    spender_principal: Principal,
    subaccount: Option<Vec<u8>>,
    amount: u128,
    memo: Option<Vec<u8>>,
    created_at_time: Option<u64>,
) -> Result<(), CurrencyError> {
    manager_for_currency(&currency)
        .await?
        .approve_allowance(
            &currency,
            spender_principal,
            subaccount,
            amount,
            memo,
            created_at_time,
        )
        .await
}

#[update]
async fn get_cksol_minter_info(currency: Currency) -> Result<CKSOLMinterInfo, CurrencyError> {
    manager_for_currency(&currency)
        .await?
        .get_cksol_minter_info(&currency)
        .await
}

#[update]
async fn get_cksol_deposit_address(
    currency: Currency,
    owner: Principal,
    subaccount: Option<Vec<u8>>,
) -> Result<String, CurrencyError> {
    manager_for_currency(&currency)
        .await?
        .get_cksol_deposit_address(&currency, owner, subaccount)
        .await
}

#[update]
async fn process_cksol_deposit(
    currency: Currency,
    owner: Principal,
    subaccount: Option<Vec<u8>>,
    signature: String,
    cycles: u128,
) -> Result<ProcessDepositSuccess, CurrencyError> {
    manager_for_currency(&currency)
        .await?
        .process_cksol_deposit(&currency, owner, subaccount, signature, cycles)
        .await
}

#[update]
async fn withdraw_to_sol_address(
    currency: Currency,
    address: String,
    amount: u64,
    from_subaccount: Option<Vec<u8>>,
) -> Result<WithdrawToSolSuccess, CurrencyError> {
    manager_for_currency(&currency)
        .await?
        .withdraw_to_sol_address(&currency, address, amount, from_subaccount)
        .await
}

#[update]
async fn check_sol_withdrawal_status(
    currency: Currency,
    block_index: u64,
) -> Result<CKSOLWithdrawalStatus, CurrencyError> {
    manager_for_currency(&currency)
        .await?
        .check_sol_withdrawal_status(&currency, block_index)
        .await
}

#[query]
fn get_canister_principal() -> Principal {
    ic_cdk::api::canister_self()
}

#[query]
fn get_account_for_currency(currency: Currency) -> AccountView {
    let state = get_canister_state();
    match currency {
        Currency::BTC => AccountView {
            owner: state.owner,
            subaccount: None,
        },
        Currency::ICP
        | Currency::TestICP
        | Currency::CKETHToken(_)
        | Currency::CKSOLToken(_)
        | Currency::GenericICRC1(Token { .. }) => AccountView {
            owner: state.owner,
            subaccount: Some(state.default_subaccount.0.to_vec()),
        },
    }
}
