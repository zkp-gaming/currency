use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

use crate::{
    cksol_minter_canister_interface::{
        CKSOLMinterInfo, CKSOLWithdrawalStatus, GetDepositAddressArg, ProcessDepositArg,
        ProcessDepositSuccess, WithdrawToSolArg, WithdrawToSolSuccess, WithdrawalStatusArg,
    },
    currency_error::CurrencyError,
    icrc1_types::{Account, Allowance, AllowanceArgs, ApproveArgs, ApproveError, TransferFromArg, TransferFromError},
    transfer::transfer_icrc1,
    types::{
        canister_wallet::CanisterWallet,
        constants::{
            CKDEVNETSOL_DECIMALS, CKDEVNETSOL_LEDGER_CANISTER_ID, CKDEVNETSOL_MINTER_CANISTER_ID,
            CKSOL_DECIMALS, CKSOL_LEDGER_CANISTER_ID, CKSOL_MINTER_CANISTER_ID,
        },
        currency::{CKSOLTokenSymbol, CKTokenConfig},
    },
    utils::get_canister_state,
};

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct CKSOLTokenWallet {
    pub config: CKTokenConfig,
}

impl CKSOLTokenWallet {
    pub async fn new(symbol: CKSOLTokenSymbol) -> Result<Self, CurrencyError> {
        let (minter_id, ledger_id, decimals) = match symbol {
            CKSOLTokenSymbol::DevnetSOL => (
                Principal::from_text(CKDEVNETSOL_MINTER_CANISTER_ID).unwrap(),
                Principal::from_text(CKDEVNETSOL_LEDGER_CANISTER_ID).unwrap(),
                CKDEVNETSOL_DECIMALS,
            ),
            CKSOLTokenSymbol::SOL => (
                Principal::from_text(CKSOL_MINTER_CANISTER_ID).unwrap(),
                Principal::from_text(CKSOL_LEDGER_CANISTER_ID).unwrap(),
                CKSOL_DECIMALS,
            ),
        };

        let fee = Self::query_fee(ledger_id).await?;

        Ok(Self {
            config: CKTokenConfig {
                minter_id,
                ledger_id,
                token_symbol: crate::Currency::CKSOLToken(symbol),
                decimals,
                fee,
            },
        })
    }

    async fn query_fee(ledger_id: Principal) -> Result<u128, CurrencyError> {
        let response = ic_cdk::call::Call::unbounded_wait(ledger_id, "icrc1_fee")
            .await
            .map_err(|e| CurrencyError::CanisterCallFailed(format!("Failed to query ckSOL fee: {:?}", e)))?;
        let (fee,): (candid::Nat,) = response
            .candid_tuple()
            .map_err(|e| CurrencyError::CanisterCallFailed(format!("Failed to decode ckSOL fee: {:?}", e)))?;

        fee.0
            .to_string()
            .parse::<u128>()
            .map_err(|e| CurrencyError::LedgerError(format!("Failed to parse ckSOL fee: {e}")))
    }

    pub async fn get_minter_info(&self) -> Result<CKSOLMinterInfo, CurrencyError> {
        let response = ic_cdk::call::Call::unbounded_wait(self.config.minter_id, "get_minter_info")
            .await
            .map_err(|e| CurrencyError::CanisterCallFailed(format!("{:?}", e)))?;
        let (info,): (CKSOLMinterInfo,) = response
            .candid_tuple()
            .map_err(|e| CurrencyError::CanisterCallFailed(format!("{:?}", e)))?;
        Ok(info)
    }

    pub async fn get_deposit_address(
        &self,
        owner: Principal,
        subaccount: Option<Vec<u8>>,
    ) -> Result<String, CurrencyError> {
        let response =
            ic_cdk::call::Call::unbounded_wait(self.config.minter_id, "get_deposit_address")
                .with_arg(GetDepositAddressArg {
                    owner: Some(owner),
                    subaccount,
                })
                .await
                .map_err(|e| CurrencyError::CanisterCallFailed(format!("{:?}", e)))?;
        let (address,): (String,) = response
            .candid_tuple()
            .map_err(|e| CurrencyError::CanisterCallFailed(format!("{:?}", e)))?;
        Ok(address)
    }

    pub async fn process_deposit(
        &self,
        owner: Principal,
        subaccount: Option<Vec<u8>>,
        signature: String,
        cycles: u128,
    ) -> Result<ProcessDepositSuccess, CurrencyError> {
        let response = ic_cdk::call::Call::unbounded_wait(self.config.minter_id, "process_deposit")
            .with_arg(ProcessDepositArg {
                owner: Some(owner),
                subaccount,
                signature,
            })
            .with_cycles(cycles)
            .await
            .map_err(|e| CurrencyError::CanisterCallFailed(format!("{:?}", e)))?;
        let (result,): (Result<ProcessDepositSuccess, String>,) = response
            .candid_tuple()
            .map_err(|e| CurrencyError::CanisterCallFailed(format!("{:?}", e)))?;
        result.map_err(CurrencyError::CanisterCallFailed)
    }

    pub async fn withdraw_to_sol_address(
        &self,
        address: String,
        amount: u64,
        from_subaccount: Option<Vec<u8>>,
    ) -> Result<WithdrawToSolSuccess, CurrencyError> {
        let response = ic_cdk::call::Call::unbounded_wait(self.config.minter_id, "withdraw")
            .with_arg(WithdrawToSolArg {
                address,
                amount,
                from_subaccount,
            })
            .await
            .map_err(|e| CurrencyError::CanisterCallFailed(format!("{:?}", e)))?;
        let (result,): (Result<WithdrawToSolSuccess, String>,) = response
            .candid_tuple()
            .map_err(|e| CurrencyError::CanisterCallFailed(format!("{:?}", e)))?;
        result.map_err(CurrencyError::WithdrawalFailed)
    }

    pub async fn check_withdrawal_status(
        &self,
        block_index: u64,
    ) -> Result<CKSOLWithdrawalStatus, CurrencyError> {
        let response = ic_cdk::call::Call::unbounded_wait(self.config.minter_id, "withdrawal_status")
            .with_arg(WithdrawalStatusArg { block_index })
            .await
            .map_err(|e| CurrencyError::CanisterCallFailed(format!("{:?}", e)))?;
        let (status,): (CKSOLWithdrawalStatus,) = response
            .candid_tuple()
            .map_err(|e| CurrencyError::CanisterCallFailed(format!("{:?}", e)))?;
        Ok(status)
    }

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

        let response = ic_cdk::call::Call::unbounded_wait(self.config.ledger_id, "icrc2_allowance")
            .with_arg(args)
            .await
            .map_err(|e| CurrencyError::AllowanceCheckFailed(format!("{:?}", e)))?;
        let (allowance,): (Allowance,) = response
            .candid_tuple()
            .map_err(|e| CurrencyError::AllowanceCheckFailed(format!("{:?}", e)))?;
        Ok(allowance)
    }

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
            fee: Some(self.config.fee),
            memo,
            created_at_time,
        };

        let response = ic_cdk::call::Call::unbounded_wait(self.config.ledger_id, "icrc2_transfer_from")
            .with_arg(args)
            .await
            .map_err(|e| CurrencyError::TransferFromFailed(format!("{:?}", e)))?;
        let (result,): (Result<u128, TransferFromError>,) = response
            .candid_tuple()
            .map_err(|e| CurrencyError::TransferFromFailed(format!("{:?}", e)))?;

        match result {
            Ok(block_index) => Ok(block_index),
            Err(TransferFromError::InsufficientAllowance { .. }) => {
                Err(CurrencyError::InsufficientAllowance)
            }
            Err(TransferFromError::Duplicate { duplicate_of }) => {
                Err(CurrencyError::DuplicateTransaction { id: duplicate_of })
            }
            Err(e) => Err(CurrencyError::TransferFromFailed(format!("{:?}", e))),
        }
    }

    pub async fn approve(
        &self,
        spender: Principal,
        amount: u128,
        from_subaccount: Option<Vec<u8>>,
        memo: Option<Vec<u8>>,
        created_at_time: Option<u64>,
    ) -> Result<(), CurrencyError> {
        let approve_args = ApproveArgs {
            spender: Account {
                owner: spender,
                subaccount: None,
            },
            amount,
            expected_allowance: None,
            expires_at: None,
            fee: Some(self.config.fee),
            memo,
            from_subaccount,
            created_at_time,
        };

        let response = ic_cdk::call::Call::unbounded_wait(self.config.ledger_id, "icrc2_approve")
            .with_arg(approve_args)
            .await
            .map_err(|e| CurrencyError::ApproveFailed(format!("{:?}", e)))?;
        let (result,): (Result<u128, ApproveError>,) = response
            .candid_tuple()
            .map_err(|e| CurrencyError::ApproveFailed(format!("{:?}", e)))?;

        match result {
            Ok(_) => Ok(()),
            Err(ApproveError::Duplicate { duplicate_of }) => {
                Err(CurrencyError::DuplicateTransaction { id: duplicate_of })
            }
            Err(e) => Err(CurrencyError::ApproveFailed(format!("{:?}", e))),
        }
    }
}

impl CanisterWallet for CKSOLTokenWallet {
    async fn deposit(
        &self,
        from_principal: Principal,
        subaccount: Option<Vec<u8>>,
        amount: u64,
        memo: Option<Vec<u8>>,
        created_at_time: Option<u64>,
    ) -> Result<(), CurrencyError> {
        self.transfer_from(from_principal, subaccount, amount, memo, created_at_time)
            .await?;
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
        let allowance = self.check_allowance(from_principal, subaccount).await?;
        if allowance.allowance < amount as u128 {
            return Err(CurrencyError::InsufficientAllowance);
        }

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
        let from_subaccount =
            subaccount.or_else(|| Some(get_canister_state().default_subaccount.0.to_vec()));

        transfer_icrc1(
            self.config.ledger_id,
            amount,
            from_subaccount,
            Some(get_canister_state().default_subaccount.0.to_vec()),
            wallet_principal_id,
            Some(self.config.fee),
            memo,
            created_at_time,
        )
        .await?;
        Ok(())
    }

    async fn get_balance(&self, principal_id: Principal) -> Result<u128, CurrencyError> {
        let account = Account {
            owner: principal_id,
            subaccount: Some(get_canister_state().default_subaccount.0.to_vec()),
        };

        let response = ic_cdk::call::Call::unbounded_wait(self.config.ledger_id, "icrc1_balance_of")
            .with_arg(account)
            .await
            .map_err(|e| CurrencyError::LedgerError(format!("Failed to query ckSOL balance: {:?}", e)))?;
        let (balance,): (candid::Nat,) = response
            .candid_tuple()
            .map_err(|e| CurrencyError::LedgerError(format!("Failed to decode ckSOL balance: {:?}", e)))?;

        balance
            .0
            .to_string()
            .parse::<u128>()
            .map_err(|e| CurrencyError::LedgerError(format!("Failed to convert ckSOL balance: {e}")))
    }
}
