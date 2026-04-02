use candid::Principal;
use ic_ledger_types::{AccountIdentifier, MAINNET_LEDGER_CANISTER_ID, Memo, Subaccount, Timestamp};

use crate::{
    currency_error::CurrencyError,
    icrc1_types::{Account, TransferArg, TransferErrorIcrc1}, types::constants::TEST_ICP_LEDGER_CANISTER_ID,
};

pub async fn transfer_icp(
    amount: u64,
    from_subaccount: Subaccount,
    to: Principal,
    memo: Option<u64>,
    created_at_time: Option<Timestamp>,
) -> Result<(), CurrencyError> {
    let transfer_result = ic_ledger_types::transfer(
        MAINNET_LEDGER_CANISTER_ID,
    &ic_ledger_types::TransferArgs {
            memo: Memo(memo.unwrap_or_default()), // Use an appropriate memo
            amount: ic_ledger_types::Tokens::from_e8s(amount - ic_ledger_types::DEFAULT_FEE.e8s()),
            fee: ic_ledger_types::DEFAULT_FEE,
            from_subaccount: Some(from_subaccount),
            to: AccountIdentifier::new(&to, &ic_ledger_types::DEFAULT_SUBACCOUNT),
            created_at_time, // Optionally specify a time
        },
    )
    .await;

    match transfer_result {
        Ok(result) => match result {
            Ok(block_index) => ic_cdk::println!(
                "Transfer successful with block index {}",
                block_index
            ),
            Err(e) => {
                return Err(CurrencyError::LedgerError(format!(
                    "Transfer failed: {:?}",
                    e
                )))
            }
        },
        Err(e) => {
            return Err(CurrencyError::LedgerError(format!(
                "ICDK call error: {:?}",
                e
            )))
        }
    }
    Ok(())
}

pub async fn transfer_test_icp(
    amount: u64,
    from_subaccount: Option<Subaccount>,
    to: Principal,
    memo: Option<u64>,
    created_at_time: Option<Timestamp>,
) -> Result<(), CurrencyError> {
    let transfer_result = ic_ledger_types::transfer(
        Principal::from_text(TEST_ICP_LEDGER_CANISTER_ID).unwrap(),
    &ic_ledger_types::TransferArgs {
            memo: Memo(memo.unwrap_or_default()), // Use an appropriate memo
            amount: ic_ledger_types::Tokens::from_e8s(amount - ic_ledger_types::DEFAULT_FEE.e8s()),
            fee: ic_ledger_types::DEFAULT_FEE,
            from_subaccount,
            to: AccountIdentifier::new(&to, &ic_ledger_types::DEFAULT_SUBACCOUNT),
            created_at_time, // Optionally specify a time
        },
    )
    .await;

    match transfer_result {
        Ok(result) => match result {
            Ok(block_index) => ic_cdk::println!(
                "Transfer successful with block index {}",
                block_index
            ),
            Err(e) => {
                return Err(CurrencyError::LedgerError(format!(
                    "Transfer failed: {:?}",
                    e
                )))
            }
        },
        Err(e) => {
            return Err(CurrencyError::LedgerError(format!(
                "ICDK call error: {:?}",
                e
            )))
        }
    }
    Ok(())
}

// Adjusted transfer_icrc1 function
pub async fn transfer_icrc1(
    ledger_canister_id: Principal,
    amount: u64,
    from_subaccount: Option<Vec<u8>>,
    to_subaccount: Option<Vec<u8>>,
    to_account: Principal,
    fee: Option<u128>,
    memo: Option<Vec<u8>>,
    created_at_time: Option<u64>,
) -> Result<u128, CurrencyError> {
    ic_cdk::println!(
        "Transferring {} tokens to account {:?}",
        amount,
        &to_account,
    );

    let transfer_args = TransferArg {
        to: Account {
            owner: to_account,
            subaccount: to_subaccount,
        },
        fee,
        amount: (amount as u128 - fee.unwrap_or(ic_ledger_types::DEFAULT_FEE.e8s().into())),
        memo,
        from_subaccount,
        created_at_time,
    };

    // Call the icrc1_transfer method
    let transfer_result = ic_cdk::call::Call::unbounded_wait(ledger_canister_id, "icrc1_transfer")
        .with_arg(transfer_args)
        .await;

    ic_cdk::println!("Transfer result: {:?}", transfer_result);

    match transfer_result {
        Ok(response) => {
            let (transfer_result,): (Result<u128, TransferErrorIcrc1>,) = response
                .candid_tuple()
                .map_err(|e| CurrencyError::LedgerError(format!("{:?}", e)))?;
            match transfer_result {
                Ok(block_index) => {
            ic_cdk::println!(
                "Transfer successful with block index {}",
                block_index
            );
            Ok(block_index)
                }
                Err(e) => Err(CurrencyError::LedgerError(format!(
                    "Ledger transfer error: {:?}",
                    e
                ))),
            }
        }
        Err(e) => Err(CurrencyError::LedgerError(format!(
            "Failed to call ledger: {:?}",
            e
        ))),
    }
}
