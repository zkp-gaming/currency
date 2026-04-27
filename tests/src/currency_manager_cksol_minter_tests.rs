use currency::{
    cksol_minter_canister_interface::{CKSOLWithdrawalStatus, ProcessDepositSuccess},
    currency_error::CurrencyError,
    types::currency::CKSOLTokenSymbol,
    Currency,
};

use crate::{
    env::new_test_env,
    utils::{
        manager_check_sol_withdrawal_status, manager_get_cksol_deposit_address,
        manager_get_cksol_minter_info, manager_process_cksol_deposit,
        manager_withdraw_to_sol_address, non_default_subaccount, test_principal,
    },
    PROCESS_DEPOSIT_REQUIRED_CYCLES,
};

#[test]
fn cksol_minter_info_returns_required_cycles_for_both_networks() {
    let env = new_test_env();

    for currency in [
        Currency::CKSOLToken(CKSOLTokenSymbol::DevnetSOL),
        Currency::CKSOLToken(CKSOLTokenSymbol::SOL),
    ] {
        let info = manager_get_cksol_minter_info(env, currency).unwrap();
        assert_eq!(
            info.process_deposit_required_cycles,
            PROCESS_DEPOSIT_REQUIRED_CYCLES
        );
    }
}

#[test]
fn cksol_deposit_address_uses_explicit_owner_and_subaccount() {
    let env = new_test_env();
    let owner = test_principal("cksol-deposit-owner");
    let subaccount = non_default_subaccount(42);

    let address = manager_get_cksol_deposit_address(
        env,
        Currency::CKSOLToken(CKSOLTokenSymbol::DevnetSOL),
        owner,
        Some(subaccount.clone()),
    )
    .unwrap();

    let subaccount_hex = subaccount
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(address, format!("sol:{}:{}", owner.to_text(), subaccount_hex));
}

#[test]
fn cksol_process_deposit_requires_cycles_and_returns_minted_result() {
    let env = new_test_env();
    let owner = test_principal("cksol-process-owner");

    let insufficient = manager_process_cksol_deposit(
        env,
        Currency::CKSOLToken(CKSOLTokenSymbol::SOL),
        owner,
        None,
        "sig-too-few-cycles".to_string(),
        PROCESS_DEPOSIT_REQUIRED_CYCLES - 1,
    );
    assert!(matches!(insufficient, Err(CurrencyError::CanisterCallFailed(_))));

    let minted = manager_process_cksol_deposit(
        env,
        Currency::CKSOLToken(CKSOLTokenSymbol::SOL),
        owner,
        None,
        "sig-happy-path".to_string(),
        PROCESS_DEPOSIT_REQUIRED_CYCLES,
    )
    .unwrap();
    assert_eq!(
        minted,
        ProcessDepositSuccess::Minted {
            block_index: 1001,
            minted_amount: 990_000_000,
            deposit_id: 1,
        }
    );
}

#[test]
fn cksol_withdraw_status_round_trip_decodes_expected_variants() {
    let env = new_test_env();
    let currency = Currency::CKSOLToken(CKSOLTokenSymbol::DevnetSOL);

    let sent = manager_withdraw_to_sol_address(
        env,
        currency,
        "sent-address".to_string(),
        1_000_000_000,
        None,
    )
    .unwrap();
    let sent_status = manager_check_sol_withdrawal_status(env, currency, sent.block_index).unwrap();
    assert_eq!(
        sent_status,
        CKSOLWithdrawalStatus::TxSent {
            signature: format!("sent-signature-{}", sent.block_index),
        }
    );

    let finalized = manager_withdraw_to_sol_address(
        env,
        currency,
        "finalized-address".to_string(),
        1_000_000_000,
        None,
    )
    .unwrap();
    let finalized_status =
        manager_check_sol_withdrawal_status(env, currency, finalized.block_index).unwrap();
    assert_eq!(
        finalized_status,
        CKSOLWithdrawalStatus::TxFinalized {
            signature: format!("finalized-signature-{}", finalized.block_index),
        }
    );

    let failed = manager_withdraw_to_sol_address(
        env,
        currency,
        "failed-address".to_string(),
        1_000_000_000,
        None,
    )
    .unwrap();
    let failed_status =
        manager_check_sol_withdrawal_status(env, currency, failed.block_index).unwrap();
    assert_eq!(
        failed_status,
        CKSOLWithdrawalStatus::Failed(format!("failed-signature-{}", failed.block_index))
    );
}
