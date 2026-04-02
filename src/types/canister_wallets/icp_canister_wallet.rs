use crate::{
    currency_error::CurrencyError,
    icrc1_types::{Account, Allowance, AllowanceArgs, ApproveArgs, ApproveError, TransferFromArg, TransferFromError},
    transfer::transfer_icp,
};
use candid::{CandidType, Principal};
use ic_ledger_types::Timestamp;
use serde::{Deserialize, Serialize};

use crate::{
    state::TransactionState, types::canister_wallet::CanisterWallet, utils::get_canister_state,
};

pub const ICP_LEDGER_CANISTER_ID: &str = "ryjl3-tyaaa-aaaaa-aaaba-cai";

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct ICPCanisterWallet;

impl ICPCanisterWallet {
    /// Check the allowance granted by a user to this canister
    pub async fn check_allowance(
        &self,
        from_principal: Principal,
        subaccount: Option<Vec<u8>>,
    ) -> Result<Allowance, CurrencyError> {
        let args = AllowanceArgs {
            account: Account {
                owner: from_principal,
                subaccount,
            },
            spender: Account {
                owner: ic_cdk::api::canister_self(),
                subaccount: None,
            },
        };

        let response = ic_cdk::call::Call::unbounded_wait(
            Principal::from_text(ICP_LEDGER_CANISTER_ID).unwrap(),
            "icrc2_allowance",
        )
        .with_arg(args)
        .await
        .map_err(|e| CurrencyError::AllowanceCheckFailed(format!("{:?}", e)))?;
        let (allowance,): (Allowance,) = response
            .candid_tuple()
            .map_err(|e| CurrencyError::AllowanceCheckFailed(format!("{:?}", e)))?;

        Ok(allowance)
    }

    /// Transfer tokens from a user's account to this canister using ICRC-2 transfer_from
    pub async fn transfer_from(
        &self,
        from_principal: Principal,
        from_subaccount: Option<Vec<u8>>,
        amount: u64,
        memo: Option<Vec<u8>>,
        created_at_time: Option<u64>,
    ) -> Result<u128, CurrencyError> {
        let canister_account = Account {
            owner: ic_cdk::api::canister_self(),
            subaccount: None,
        };

        let from_account = Account {
            owner: from_principal,
            subaccount: from_subaccount,
        };

        let args = TransferFromArg {
            spender_subaccount: None,
            from: from_account,
            to: canister_account,
            amount: amount.into(),
            fee: Some(ic_ledger_types::DEFAULT_FEE.e8s().into()),
            memo,
            created_at_time,
        };

        let response = ic_cdk::call::Call::unbounded_wait(
            Principal::from_text(ICP_LEDGER_CANISTER_ID).unwrap(),
            "icrc2_transfer_from",
        )
        .with_arg(args)
        .await
        .map_err(|e| CurrencyError::TransferFromFailed(format!("{:?}", e)))?;
        let (result,): (Result<u128, TransferFromError>,) = response
            .candid_tuple()
            .map_err(|e| CurrencyError::TransferFromFailed(format!("{:?}", e)))?;

        match result {
            Ok(block_index) => Ok(block_index),
            Err(e) => match e {
                TransferFromError::InsufficientAllowance { .. } => {
                    Err(CurrencyError::InsufficientAllowance)
                }
                TransferFromError::Duplicate { duplicate_of } => {
                    Err(CurrencyError::DuplicateTransaction { id: duplicate_of })
                }
                _ => Err(CurrencyError::TransferFromFailed(format!("{:?}", e))),
            },
        }
    }

    pub async fn approve(
        &self,
        spender: Principal,
        amount: u128,
        from_subaccount: Option<Vec<u8>>,
        memo: Option<Vec<u8>>,
        created_at_time: Option<u64>
    ) -> Result<(), CurrencyError> {
        let approve_args = ApproveArgs {
            spender: Account {
                owner: spender,
                subaccount: None,
            },
            amount,
            expected_allowance: None,
            expires_at: None,
            fee: Some(ic_ledger_types::DEFAULT_FEE.e8s() as u128),
            memo,
            from_subaccount,
            created_at_time,
        };

        let response = ic_cdk::call::Call::unbounded_wait(
            Principal::from_text(ICP_LEDGER_CANISTER_ID).unwrap(),
            "icrc2_approve",
        )
        .with_arg(approve_args)
        .await
        .map_err(|e| CurrencyError::ApproveFailed(format!("{:?}", e)))?;
        let (result,): (Result<u128, ApproveError>,) = response
            .candid_tuple()
            .map_err(|e| CurrencyError::ApproveFailed(format!("{:?}", e)))?;

        match result {
            Ok(_) => Ok(()),
            Err(e) => match e {
                ApproveError::Duplicate { duplicate_of } => {
                    Err(CurrencyError::DuplicateTransaction { id: duplicate_of })
                }
                _ => Err(CurrencyError::ApproveFailed(format!("{:?}", e))),
            },
        }
    }
}

impl CanisterWallet for ICPCanisterWallet {
    async fn deposit(
        &self,
        transaction_state: &mut TransactionState,
        from_principal: Principal,
        subaccount: Option<Vec<u8>>,
        amount: u64,
        memo: Option<Vec<u8>>,
        created_at_time: Option<u64>,
    ) -> Result<(), CurrencyError> {
        // First check the allowance to make sure it's sufficient
        let allowance = self
            .check_allowance(from_principal, subaccount.clone())
            .await?;

        if allowance.allowance < amount as u128 {
            return Err(CurrencyError::InsufficientAllowance);
        }

        // Check if the allowance is expired
        if let Some(expires_at) = allowance.expires_at {
            if expires_at < ic_cdk::api::time() {
                return Err(CurrencyError::InsufficientAllowance);
            }
        }

        // Transfer the tokens using the allowance
        let block_index = self
            .transfer_from(from_principal, subaccount, amount, memo, created_at_time)
            .await?;

        // Record the transaction
        let tx_id = format!(
            "ICP-DEPOSIT-{}-{}-{}",
            block_index,
            from_principal,
            ic_cdk::api::time()
        );

        transaction_state.add_transaction(tx_id);

        Ok(())
    }

    async fn validate_allowance(
        &self, 
        from_principal: Principal, 
        subaccount: Option<Vec<u8>>,
        amount: u64,
        _memo: Option<Vec<u8>>,
        _created_at_time: Option<u64>,
    ) -> Result<(), CurrencyError> {
        // Check the allowance to make sure it's sufficient
        let allowance = self.check_allowance(from_principal, subaccount).await?;
        
        if allowance.allowance < amount as u128 {
            return Err(CurrencyError::InsufficientAllowance);
        }
        
        // Check if the allowance is expired
        if let Some(expires_at) = allowance.expires_at {
            if expires_at < ic_cdk::api::time() {
                return Err(CurrencyError::InsufficientAllowance);
            }
        }
        
        Ok(())
    }

    async fn withdraw(
        &self,
        wallet_principal_id: Principal,
        subaccount: Option<Vec<u8>>,
        amount: u64,
        memo: Option<Vec<u8>>,
        created_at_time: Option<u64>,
    ) -> Result<(), CurrencyError> {
        let from_subaccount = ic_ledger_types::Subaccount(
            subaccount
                .unwrap_or_else(|| get_canister_state().default_subaccount.0.to_vec())
                .try_into()
                .map_err(|_| {
                    CurrencyError::LedgerError("Invalid ICP subaccount length".to_string())
                })?,
        );

        let memo = memo.map(|m| m.iter().map(|b| *b as u64).sum());

        // map timestamp option to option timestamp
        let created_at_time = created_at_time.map(|t| Timestamp { timestamp_nanos: t });

        transfer_icp(amount, from_subaccount, wallet_principal_id, memo, created_at_time).await?;
        Ok(())
    }

    async fn get_balance(&self, principal_id: Principal) -> Result<u128, CurrencyError> {
        let default_subaccount = {
            let canister_state = get_canister_state();
            canister_state.default_subaccount
        };

        let account = Account {
            owner: principal_id,
            subaccount: Some(default_subaccount.0.to_vec()),
        };
        
        let response = ic_cdk::call::Call::unbounded_wait(
            Principal::from_text(ICP_LEDGER_CANISTER_ID).unwrap(),
            "icrc1_balance_of",
        )
        .with_arg(account)
        .await
        .map_err(|e| CurrencyError::LedgerError(format!("Failed to query ICP balance: {:?}", e)))?;
        let (balance,): (candid::Nat,) = response
            .candid_tuple()
            .map_err(|e| CurrencyError::LedgerError(format!("Failed to decode ICP balance: {:?}", e)))?;
        
        // Convert candid::Nat to u64, ensuring it doesn't overflow
        let balance_str = balance.0.to_string();
        let balance_u128 = match balance_str.parse::<u128>() {
            Ok(val) => val,
            Err(_) => {
                return Err(CurrencyError::LedgerError(
                    format!("Failed to convert balance '{}' to u128", balance_str)
                ));
            }
        };

        Ok(balance_u128)
    }
}
