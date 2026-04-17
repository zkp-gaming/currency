use crate::{
    cketh_minter_canister_interface::{
        Eip1559TransactionPrice, Eip1559TransactionPriceArg, EventPayload, GetEventsArg,
        GetEventsRet, LedgerError, MinterInfo, TxFinalizedStatus, WithdrawErc20Arg,
        WithdrawErc20Error, WithdrawErc20Ret, WithdrawEthRet, WithdrawalArg, WithdrawalDetail,
        WithdrawalSearchParameter, WithdrawalStatus,
    }, currency_error::CurrencyError, icrc1_types::{Account, Allowance, AllowanceArgs, ApproveArgs, ApproveError, TransferFromArg, TransferFromError}, transfer::transfer_icrc1
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

use crate::{
    types::{
        canister_wallet::CanisterWallet,
        constants::{
            CKSEPOLIA_ETH_DECIMALS, CKSEPOLIA_ETH_LEDGER_CANISTER_ID,
            CKSEPOLIA_ETH_MINTER_CANISTER_ID, CKSEPOLIA_USDC_DECIMALS,
            CKSEPOLIA_USDC_LEDGER_CANISTER_ID, CKSEPOLIA_USDC_MINTER_CANISTER_ID,
            ETH_DECIMALS, ETH_LEDGER_CANISTER_ID, ETH_MINTER_CANISTER_ID, USDC_DECIMALS,
            USDC_LEDGER_CANISTER_ID, USDC_MINTER_CANISTER_ID, USDT_DECIMALS,
            USDT_LEDGER_CANISTER_ID, USDT_MINTER_CANISTER_ID,
        },
        currency::{CKTokenConfig, CKTokenSymbol},
    },
    utils::get_canister_state,
};

#[derive(Debug, Clone, CandidType, Serialize, Deserialize)]
pub enum CKTokenWithdrawalStatus {
    Pending,
    TxCreated,
    TxSent {
        transaction_hash: String,
    },
    TxFinalized {
        transaction_hash: String,
        effective_fee: Option<u64>,
    },
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct CKERC20TokenWallet {
    pub config: CKTokenConfig,
}

impl CKERC20TokenWallet {
    pub fn new(symbol: CKTokenSymbol) -> Self {
        let config = match symbol {
            CKTokenSymbol::USDC => CKTokenConfig {
                minter_id: Principal::from_text(USDC_MINTER_CANISTER_ID).unwrap(),
                ledger_id: Principal::from_text(USDC_LEDGER_CANISTER_ID).unwrap(),
                token_symbol: crate::Currency::CKETHToken(symbol),
                decimals: USDC_DECIMALS,
                fee: 10_000,
            },
            CKTokenSymbol::USDT => CKTokenConfig {
                minter_id: Principal::from_text(USDT_MINTER_CANISTER_ID).unwrap(),
                ledger_id: Principal::from_text(USDT_LEDGER_CANISTER_ID).unwrap(),
                token_symbol: crate::Currency::CKETHToken(symbol),
                decimals: USDT_DECIMALS,
                fee: 10_000,
            },
            CKTokenSymbol::ETH => CKTokenConfig {
                minter_id: Principal::from_text(ETH_MINTER_CANISTER_ID).unwrap(),
                ledger_id: Principal::from_text(ETH_LEDGER_CANISTER_ID).unwrap(),
                token_symbol: crate::Currency::CKETHToken(symbol),
                decimals: ETH_DECIMALS,
                fee: 2_000_000_000_000,
            },
            CKTokenSymbol::SepoliaETH => CKTokenConfig {
                minter_id: Principal::from_text(CKSEPOLIA_ETH_MINTER_CANISTER_ID).unwrap(),
                ledger_id: Principal::from_text(CKSEPOLIA_ETH_LEDGER_CANISTER_ID).unwrap(),
                token_symbol: crate::Currency::CKETHToken(symbol),
                decimals: CKSEPOLIA_ETH_DECIMALS,
                fee: 10_000_000_000,
            },
            CKTokenSymbol::SepoliaUSDC => CKTokenConfig {
                minter_id: Principal::from_text(CKSEPOLIA_USDC_MINTER_CANISTER_ID).unwrap(),
                ledger_id: Principal::from_text(CKSEPOLIA_USDC_LEDGER_CANISTER_ID).unwrap(),
                token_symbol: crate::Currency::CKETHToken(symbol),
                decimals: CKSEPOLIA_USDC_DECIMALS,
                fee: 4_000,
            },
        };
        Self { config }
    }

    pub async fn get_deposit_address(&self) -> Result<Option<String>, CurrencyError> {
        // Call the minter's smart_contract_address function directly
        let response =
            ic_cdk::call::Call::unbounded_wait(self.config.minter_id, "smart_contract_address")
                .await
                .map_err(|e| CurrencyError::CanisterCallFailed(format!("{:?}", e)))?;
        let (deposit_address,): (Option<String>,) = response
            .candid_tuple()
            .map_err(|e| CurrencyError::CanisterCallFailed(format!("{:?}", e)))?;

        Ok(deposit_address)
    }

    /// Gets the special helper contract address that allows USDC deposits to be minted as ckUSDC
    /// directly to the table canister's principal.
    ///
    /// This differs from the regular smart contract address (get_deposit_address) because:
    /// 1. Users send USDC to this helper contract instead of the regular deposit contract
    /// 2. The user must include our table canister's principal (ic_cdk::api::id()) in their transaction data
    /// 3. The helper contract will mint ckUSDC directly to the table canister rather than to the user
    ///
    /// Returns:
    /// - The Ethereum address of the helper contract that supports specifying a receiving principal
    /// - Error if the helper contract address is not configured in the minter
    pub async fn get_deposit_address_for_principal(&self) -> Result<String, CurrencyError> {
        // Get the deposit with subaccount helper contract address
        let response =
            ic_cdk::call::Call::unbounded_wait(self.config.minter_id, "get_minter_info")
                .await
                .map_err(|e| CurrencyError::CanisterCallFailed(format!("{:?}", e)))?;
        let (minter_info,): (MinterInfo,) = response
            .candid_tuple()
            .map_err(|e| CurrencyError::CanisterCallFailed(format!("{:?}", e)))?;

        minter_info
            .deposit_with_subaccount_helper_contract_address
            .ok_or(CurrencyError::NoDepositAddress)
    }

    /// Monitors for a ckToken minting event from a specific Ethereum transaction
    /// Returns the block number where the ckTokens were minted once found.
    /// Should be used after sending tokens to the address recieved from `get_deposit_address_for_principal`
    pub async fn get_mint_block_number(
        &self,
        eth_transaction_hash: String,
    ) -> Result<u64, CurrencyError> {
        // We'll try a few times since there might be a delay
        for _ in 0..10 {
            let events_arg = GetEventsArg {
                // Look at last 100 events - might need adjustment
                start: 0,
                length: 100,
            };

            let response = ic_cdk::call::Call::unbounded_wait(self.config.minter_id, "get_events")
                .with_arg(events_arg)
                .await
                .map_err(|e| CurrencyError::CanisterCallFailed(format!("{:?}", e)))?;
            let (events,): (GetEventsRet,) = response
                .candid_tuple()
                .map_err(|e| CurrencyError::CanisterCallFailed(format!("{:?}", e)))?;

            // Look for MintedCkErc20 event with matching transaction hash
            for event in events.events {
                if let EventPayload::MintedCkErc20 {
                    event_source,
                    mint_block_index,
                    ..
                } = event.payload
                {
                    if event_source.transaction_hash == eth_transaction_hash {
                        return mint_block_index.0.try_into().map_err(|_| {
                            CurrencyError::QueryError("Block number too large".to_string())
                        });
                    }
                }
            }

            // Wait a bit before trying again
        }

        Err(CurrencyError::TransactionNotFound)
    }

    async fn get_erc20_withdrawal_fee(&self) -> Result<u128, CurrencyError> {
        let response = ic_cdk::call::Call::unbounded_wait(
            self.config.minter_id,
            "eip1559_transaction_price",
        )
        .with_arg(Eip1559TransactionPriceArg {
            ckerc20_ledger_id: self.config.ledger_id,
        })
        .await
        .map_err(|e| CurrencyError::CanisterCallFailed(format!("{:?}", e)))?;
        let (price,): (Eip1559TransactionPrice,) = response
            .candid_tuple()
            .map_err(|e| CurrencyError::CanisterCallFailed(format!("{:?}", e)))?;
        price
            .max_transaction_fee
            .0
            .try_into()
            .map_err(|_| CurrencyError::QueryError("gas fee too large for u128".to_string()))
    }

    /// Initiates a withdrawal back to an Ethereum address via the ckETH minter.
    /// Handles the required `icrc2_approve` calls before invoking the minter.
    /// Returns the withdrawal block index, which can be passed to `check_withdrawal_status`.
    /// - ETH / SepoliaETH: approves ckETH ledger, calls `withdraw_eth`
    /// - USDC / USDT / SepoliaUSDC: approves ckETH (gas) + ckERC20 ledgers, calls `withdraw_erc20`
    pub async fn withdraw_icrc1_token_to_eth_address(
        &self,
        eth_address: String,
        amount: u64,
    ) -> Result<u64, CurrencyError> {
        let is_eth = matches!(
            self.config.token_symbol,
            crate::Currency::CKETHToken(CKTokenSymbol::ETH)
                | crate::Currency::CKETHToken(CKTokenSymbol::SepoliaETH)
        );

        if is_eth {
            // Approve minter to burn ckETH for the withdrawal amount
            self.approve(
                self.config.ledger_id,
                self.config.minter_id,
                amount as u128,
                None,
                None,
                None,
            )
            .await?;

            let withdraw_arg = WithdrawalArg {
                amount: amount.into(),
                recipient: eth_address,
                from_subaccount: None,
            };

            let response =
                ic_cdk::call::Call::unbounded_wait(self.config.minter_id, "withdraw_eth")
                    .with_arg(withdraw_arg)
                    .await
                    .map_err(|e| CurrencyError::CanisterCallFailed(format!("{:?}", e)))?;
            let (result,): (WithdrawEthRet,) = response
                .candid_tuple()
                .map_err(|e| CurrencyError::CanisterCallFailed(format!("{:?}", e)))?;

            match result {
                WithdrawEthRet::Ok(req) => req
                    .block_index
                    .0
                    .try_into()
                    .map_err(|_| CurrencyError::QueryError("block index too large for u64".to_string())),
                WithdrawEthRet::Err(e) => Err(CurrencyError::WithdrawalFailed(format!("{:?}", e))),
            }
        } else {
            // Determine the ckETH ledger for gas fee approval
            let cketh_ledger_id = match self.config.token_symbol {
                crate::Currency::CKETHToken(CKTokenSymbol::SepoliaUSDC) => {
                    Principal::from_text(CKSEPOLIA_ETH_LEDGER_CANISTER_ID).unwrap()
                }
                _ => Principal::from_text(ETH_LEDGER_CANISTER_ID).unwrap(),
            };

            // Approve minter to burn ckETH for gas fees
            let gas_fee = self.get_erc20_withdrawal_fee().await?;
            self.approve(cketh_ledger_id, self.config.minter_id, gas_fee, None, None, None)
                .await?;

            // Approve minter to burn the ckERC20 tokens
            self.approve(
                self.config.ledger_id,
                self.config.minter_id,
                amount as u128,
                None,
                None,
                None,
            )
            .await?;

            let withdraw_arg = WithdrawErc20Arg {
                amount: amount.into(),
                ckerc20_ledger_id: self.config.ledger_id,
                recipient: eth_address,
                from_cketh_subaccount: None,
                from_ckerc20_subaccount: None,
            };

            let response =
                ic_cdk::call::Call::unbounded_wait(self.config.minter_id, "withdraw_erc20")
                    .with_arg(withdraw_arg)
                    .await
                    .map_err(|e| CurrencyError::CanisterCallFailed(format!("{:?}", e)))?;
            let (result,): (WithdrawErc20Ret,) = response
                .candid_tuple()
                .map_err(|e| CurrencyError::CanisterCallFailed(format!("{:?}", e)))?;

            match result {
                WithdrawErc20Ret::Ok(req) => req
                    .cketh_block_index
                    .0
                    .try_into()
                    .map_err(|_| CurrencyError::QueryError("cketh block index too large for u64".to_string())),
                WithdrawErc20Ret::Err(e) => {
                    let error_msg = match e {
                        WithdrawErc20Error::TokenNotSupported { supported_tokens } => {
                            format!(
                                "Token not supported. Supported tokens: {:?}",
                                supported_tokens
                                    .iter()
                                    .map(|t| t.ckerc20_token_symbol.clone())
                                    .collect::<Vec<_>>()
                            )
                        }
                        WithdrawErc20Error::TemporarilyUnavailable(msg) => {
                            format!("Service temporarily unavailable: {}", msg)
                        }
                        WithdrawErc20Error::CkErc20LedgerError {
                            error,
                            cketh_block_index: _,
                        } => match error {
                            LedgerError::TemporarilyUnavailable(msg) => {
                                format!("Ledger temporarily unavailable: {}", msg)
                            }
                            LedgerError::InsufficientAllowance {
                                token_symbol,
                                allowance,
                                failed_burn_amount,
                                ..
                            } => {
                                format!(
                                    "Insufficient allowance for {}: have {} need {}",
                                    token_symbol, allowance, failed_burn_amount
                                )
                            }
                            LedgerError::AmountTooLow {
                                minimum_burn_amount,
                                token_symbol,
                                failed_burn_amount,
                                ..
                            } => {
                                format!(
                                    "Amount too low for {}: minimum {} got {}",
                                    token_symbol, minimum_burn_amount, failed_burn_amount
                                )
                            }
                            LedgerError::InsufficientFunds {
                                balance,
                                token_symbol,
                                failed_burn_amount,
                                ..
                            } => {
                                format!(
                                    "Insufficient funds for {}: have {} need {}",
                                    token_symbol, balance, failed_burn_amount
                                )
                            }
                        },
                        WithdrawErc20Error::CkEthLedgerError { error } => match error {
                            LedgerError::TemporarilyUnavailable(msg) => {
                                format!("ckETH ledger temporarily unavailable: {}", msg)
                            }
                            LedgerError::InsufficientAllowance {
                                token_symbol,
                                allowance,
                                failed_burn_amount,
                                ..
                            } => {
                                format!(
                                    "Insufficient {} allowance: have {} need {}",
                                    token_symbol, allowance, failed_burn_amount
                                )
                            }
                            LedgerError::AmountTooLow {
                                minimum_burn_amount,
                                failed_burn_amount,
                                ..
                            } => {
                                format!(
                                    "ckETH amount too low: minimum {} got {}",
                                    minimum_burn_amount, failed_burn_amount
                                )
                            }
                            LedgerError::InsufficientFunds {
                                balance,
                                failed_burn_amount,
                                ..
                            } => {
                                format!(
                                    "Insufficient ckETH funds: have {} need {}",
                                    balance, failed_burn_amount
                                )
                            }
                        },
                        WithdrawErc20Error::RecipientAddressBlocked { address } => {
                            format!("Recipient address blocked: {}", address)
                        }
                    };
                    Err(CurrencyError::WithdrawalFailed(error_msg))
                }
            }
        }
    }

    /// Check the status of a withdrawal after it's been initiated
    pub async fn check_withdrawal_status(
        &self,
        withdrawal_id: u64,
    ) -> Result<CKTokenWithdrawalStatus, CurrencyError> {
        let response = ic_cdk::call::Call::unbounded_wait(
            self.config.minter_id,
            "withdrawal_status",
        )
        .with_arg(WithdrawalSearchParameter::ByWithdrawalId(withdrawal_id))
        .await
        .map_err(|e| CurrencyError::CanisterCallFailed(format!("{:?}", e)))?;
        let (status,): (Vec<WithdrawalDetail>,) = response
            .candid_tuple()
            .map_err(|e| CurrencyError::CanisterCallFailed(format!("{:?}", e)))?;

        let detail = status.first().ok_or(CurrencyError::WithdrawalFailed(
            "Withdrawal not found".to_string(),
        ))?;

        Ok(match &detail.status {
            WithdrawalStatus::Pending => CKTokenWithdrawalStatus::Pending,
            WithdrawalStatus::TxCreated => CKTokenWithdrawalStatus::TxCreated,
            WithdrawalStatus::TxSent(tx) => CKTokenWithdrawalStatus::TxSent {
                transaction_hash: tx.transaction_hash.clone(),
            },
            WithdrawalStatus::TxFinalized(status) => match status {
                TxFinalizedStatus::Success {
                    transaction_hash,
                    effective_transaction_fee,
                } => CKTokenWithdrawalStatus::TxFinalized {
                    transaction_hash: transaction_hash.clone(),
                    effective_fee: effective_transaction_fee
                        .as_ref()
                        .map(|f| f.clone().0.try_into().unwrap_or(0)),
                },
                TxFinalizedStatus::Reimbursed {
                    transaction_hash, ..
                } => CKTokenWithdrawalStatus::Failed(format!(
                    "Transaction failed and was reimbursed. Tx hash: {}",
                    transaction_hash
                )),
                TxFinalizedStatus::PendingReimbursement(tx) => {
                    CKTokenWithdrawalStatus::Failed(format!(
                        "Transaction failed, reimbursement pending. Tx hash: {}",
                        tx.transaction_hash
                    ))
                }
            },
        })
    }

    // Helper functions for implementations
    pub async fn check_allowance(
        &self,
        ledger: Principal,
        account: Account,
        spender: Account,
    ) -> Result<Allowance, CurrencyError> {
        let args = AllowanceArgs { account, spender };

        let response = ic_cdk::call::Call::unbounded_wait(ledger, "icrc2_allowance")
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
        ledger: Principal,
        from: Account,
        to: Account,
        amount: u128,
        memo: Option<Vec<u8>>,
        created_at_time: Option<u64>,
    ) -> Result<u128, CurrencyError> {
        let args = TransferFromArg {
            spender_subaccount: None,
            from,
            to,
            amount,
            fee: Some(self.config.fee),
            memo,
            created_at_time,
        };

        let response = ic_cdk::call::Call::unbounded_wait(ledger, "icrc2_transfer_from")
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
        ledger: Principal,
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

        let response = ic_cdk::call::Call::unbounded_wait(ledger, "icrc2_approve")
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

impl CanisterWallet for CKERC20TokenWallet {
    async fn deposit(
        &self,
        from_principal: Principal,
        subaccount: Option<Vec<u8>>,
        amount: u64,
        memo: Option<Vec<u8>>,
        created_at_time: Option<u64>,
    ) -> Result<(), CurrencyError> {
        let canister_state = get_canister_state();

        let from_account = Account {
            owner: from_principal,
            subaccount,
        };
        let spender_account = Account {
            owner: canister_state.owner,
            subaccount: None,
        };

        // Transfer tokens using allowance
        self.transfer_from(
            self.config.ledger_id,
            from_account,
            spender_account,
            amount.into(),
            memo,
            created_at_time
        )
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
        let canister_state = get_canister_state();

        let from_account = Account {
            owner: from_principal,
            subaccount,
        };
        let spender_account = Account {
            owner: canister_state.owner,
            subaccount: None,
        };
        // Check the allowance to make sure it's sufficient
        let allowance = self.check_allowance(self.config.ledger_id, from_account, spender_account).await?;
        
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
            created_at_time
        )
        .await?;
        Ok(())
    }

    async fn get_balance(&self, principal_id: Principal) -> Result<u128, CurrencyError> {
        let default_subaccount = {
            let canister_state = get_canister_state();
            canister_state.default_subaccount.0
        };

        let account = Account {
            owner: principal_id,
            subaccount: Some(default_subaccount.to_vec()),
        };
        
        let response = ic_cdk::call::Call::unbounded_wait(
            self.config.ledger_id,
            "icrc1_balance_of",
        )
        .with_arg(account)
        .await
        .map_err(|e| CurrencyError::LedgerError(
            format!("Failed to query {:?} balance: {:?}", self.config.token_symbol, e)
        ))?;
        let (balance,): (candid::Nat,) = response
            .candid_tuple()
            .map_err(|e| CurrencyError::LedgerError(
                format!("Failed to decode {:?} balance: {:?}", self.config.token_symbol, e)
            ))?;
        
        // Convert the candid::Nat to u128
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
