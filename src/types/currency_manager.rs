use std::borrow::Cow;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use candid::{CandidType, Decode, Encode, Principal};
use ic_ledger_types::DEFAULT_FEE;
use ic_stable_structures::{storable::Bound, Storable};
use serde::{Deserialize, Serialize};

use crate::{
    Currency, currency_error::CurrencyError, state::TransactionState, types::{
        canister_wallet::CanisterWallet,
        canister_wallets::{
            ckerc20_token_wallet::CKERC20TokenWallet, icp_canister_wallet::ICPCanisterWallet, test_icp_wallet::TestICPCanisterWallet,
        },
    }
};

use super::canister_wallets::{btc_token_wallet::CKBTCTokenWallet, icrc1_token_wallet::GenericICRC1TokenWallet};

const MAX_VALUE_SIZE_CURRENCY_MANAGER: u32 = 100000; // Adjust based on your needs

impl Storable for CurrencyManager {
    fn to_bytes(&self) -> std::borrow::Cow<'_, [u8]> {
        Cow::Owned(Encode!(self).unwrap_or_else(|e| {
            ic_cdk::println!("CurrencyManager serialization error: {:?}", e);
            vec![]
        }))
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap_or_else(|e| {
            ic_cdk::println!("CurrencyManager deserialization error: {:?}", e);
            // Return empty CurrencyManager as fallback
            CurrencyManager {
                icp: None,
                test_icp: None,
                ckerc20_tokens: vec![],
                btc: None,
                generic_icrc1_tokens: vec![],
            }
        })
    }

    fn into_bytes(self) -> Vec<u8> {
        Encode!(&self).unwrap_or_else(|e| {
            ic_cdk::println!("CurrencyManager into_bytes serialization error: {:?}", e);
            vec![]
        })
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: MAX_VALUE_SIZE_CURRENCY_MANAGER,
        is_fixed_size: false,
    };
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct CurrencyManager {
    pub icp: Option<ICPCanisterWallet>,
    pub test_icp: Option<TestICPCanisterWallet>,
    pub ckerc20_tokens: Vec<CKERC20TokenWallet>,
    pub btc: Option<CKBTCTokenWallet>,
    pub generic_icrc1_tokens: Vec<GenericICRC1TokenWallet>,
}

impl Default for CurrencyManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CurrencyManager {
    pub fn new() -> Self {
        Self {
            icp: Some(ICPCanisterWallet),
            test_icp: Some(TestICPCanisterWallet),
            ckerc20_tokens: Vec::new(),
            btc: Some(CKBTCTokenWallet::new()),
            generic_icrc1_tokens: Vec::new(),
        }
    }

    fn bytes_to_hex(bytes: &Option<Vec<u8>>) -> String {
        match bytes {
            Some(bytes) => bytes.iter().map(|byte| format!("{byte:02x}")).collect(),
            None => "none".to_string(),
        }
    }

    fn request_id_hash(request_id: &str) -> u128 {
        let mut hasher = DefaultHasher::new();
        request_id.hash(&mut hasher);
        hasher.finish() as u128
    }

    fn build_request_id(
        operation: &str,
        currency: &Currency,
        principal: Principal,
        subaccount: &Option<Vec<u8>>,
        amount: u64,
        memo: &Option<Vec<u8>>,
        created_at_time: Option<u64>,
    ) -> Result<(String, u64), CurrencyError> {
        let created_at_time = created_at_time.ok_or(CurrencyError::MissingCreatedAtTime)?;
        let request_id = format!(
            "{operation}|{currency:?}|{principal}|{}|{amount}|{}|{created_at_time}",
            Self::bytes_to_hex(subaccount),
            Self::bytes_to_hex(memo),
        );
        Ok((request_id, created_at_time))
    }

    fn ensure_request_not_seen(
        transaction_state: &mut TransactionState,
        request_id: &str,
        created_at_time: u64,
    ) -> Result<(), CurrencyError> {
        if transaction_state.check_and_record(request_id.to_string(), created_at_time) {
            Ok(())
        } else {
            Err(CurrencyError::DuplicateTransaction {
                id: Self::request_id_hash(request_id),
            })
        }
    }

    pub async fn add_currency(&mut self, currency: Currency) -> Result<(), CurrencyError> {
        match currency {
            Currency::ICP => {
                if self.icp.is_none() {
                    self.icp = Some(ICPCanisterWallet);
                }
            }
            Currency::TestICP => {
                if self.test_icp.is_none() {
                    self.test_icp = Some(TestICPCanisterWallet);
                }
            }
            Currency::CKETHToken(token) => {
                // Only add if this specific token doesn't exist yet
                if !self
                    .ckerc20_tokens
                    .iter()
                    .any(|w: &CKERC20TokenWallet| w.config.token_symbol == Currency::CKETHToken(token))
                {
                    self.ckerc20_tokens.push(CKERC20TokenWallet::new(token));
                }
            }
            Currency::BTC => {
                if self.btc.is_none() {
                    self.btc = Some(CKBTCTokenWallet::new());
                }
            }
            Currency::GenericICRC1(token) => {
                // Only add if this specific token doesn't exist yet
                if !self
                    .generic_icrc1_tokens
                    .iter()
                    .any(|w: &GenericICRC1TokenWallet| w.metadata.symbol == token.symbol_to_string())
                {
                    self.generic_icrc1_tokens.push(GenericICRC1TokenWallet::new(token.ledger_id).await?);
                }
            }
        }
        Ok(())
    }

    pub fn remove_currency(&mut self, currency: &Currency) {
        match currency {
            Currency::ICP => {
                self.icp = None;
            }
            Currency::TestICP => {
                self.test_icp = None;
            }
            Currency::CKETHToken(token) => {
                self.ckerc20_tokens
                    .retain(|w| w.config.token_symbol != Currency::CKETHToken(*token));
            }
            Currency::BTC => {
                self.btc = None;
            }
            Currency::GenericICRC1(token) => {
                self.generic_icrc1_tokens
                    .retain(|w| w.metadata.symbol != token.symbol_to_string());
            }
        }
    }

    pub async fn deposit(
        &self,
        transaction_state: &mut TransactionState,
        currency: &Currency,
        from_principal: Principal,
        subaccount: Option<Vec<u8>>,
        amount: u64,
        memo: Option<Vec<u8>>,
        created_at_time: Option<u64>,
    ) -> Result<(), CurrencyError> {
        let (request_id, request_timestamp) = Self::build_request_id(
            "deposit",
            currency,
            from_principal,
            &subaccount,
            amount,
            &memo,
            created_at_time,
        )?;
        Self::ensure_request_not_seen(transaction_state, &request_id, request_timestamp)?;

        let result = match currency {
            Currency::ICP => match &self.icp {
                Some(icp) => icp
                    .deposit(
                        from_principal,
                        subaccount.clone(),
                        amount,
                        memo,
                        created_at_time,
                    )
                    .await,
                None => Err(CurrencyError::WalletNotSet),
            },
            Currency::TestICP => match &self.test_icp {
                Some(test_icp) => test_icp
                    .deposit(
                        from_principal,
                        subaccount.clone(),
                        amount,
                        memo,
                        created_at_time,
                    )
                    .await,
                None => Err(CurrencyError::WalletNotSet),
            },
            Currency::CKETHToken(token) => {
                let wallet = self
                    .ckerc20_tokens
                    .iter()
                    .find(|w| w.config.token_symbol == Currency::CKETHToken(*token))
                    .ok_or(CurrencyError::WalletNotSet)?;
                wallet
                    .deposit(
                        from_principal,
                        subaccount.clone(),
                        amount,
                        memo,
                        created_at_time,
                    )
                    .await
            }
            Currency::BTC => match &self.btc {
                Some(wallet) => {
                    wallet
                        .deposit(
                            from_principal,
                            subaccount.clone(),
                            amount,
                            memo,
                            created_at_time,
                        )
                        .await
                }
                None => Err(CurrencyError::WalletNotSet),
            },
            Currency::GenericICRC1(token) => {
                let wallet = self
                    .generic_icrc1_tokens
                    .iter()
                    .find(|w| w.metadata.symbol == token.symbol_to_string())
                    .ok_or(CurrencyError::WalletNotSet)?;
                wallet
                    .deposit(
                        from_principal,
                        subaccount,
                        amount,
                        memo,
                        created_at_time,
                    )
                    .await
            }
        };

        if result.is_err() {
            transaction_state.remove_transaction(&request_id);
        }

        result
    }

    pub async fn validate_allowance(
        &self,
        currency: &Currency,
        from_principal: Principal,
        subaccount: Option<Vec<u8>>,
        amount: u64,
        memo: Option<Vec<u8>>,
        created_at_time: Option<u64>,
    ) -> Result<(), CurrencyError> {
        match currency {
            Currency::ICP => match &self.icp {
                Some(wallet) => {
                    wallet
                        .validate_allowance(from_principal, subaccount, amount, memo, created_at_time)
                        .await
                }
                None => Err(CurrencyError::WalletNotSet),
            },
            Currency::TestICP => match &self.test_icp {
                Some(test_icp) => {
                    test_icp
                        .validate_allowance(from_principal, subaccount, amount, memo, created_at_time)
                        .await
                }
                None => Err(CurrencyError::WalletNotSet),
            },
            Currency::CKETHToken(token) => {
                let wallet = self
                    .ckerc20_tokens
                    .iter()
                    .find(|w| w.config.token_symbol == Currency::CKETHToken(*token))
                    .ok_or(CurrencyError::WalletNotSet)?;
                wallet
                    .validate_allowance(
                        from_principal,
                        subaccount,
                        amount,
                        memo,
                        created_at_time,
                    )
                    .await
            }
            Currency::BTC => match &self.btc {
                Some(wallet) => {
                    wallet
                        .validate_allowance(from_principal, subaccount, amount, memo, created_at_time)
                        .await
                }
                None => Err(CurrencyError::WalletNotSet),
            },
            Currency::GenericICRC1(token) => {
                let wallet = self
                    .generic_icrc1_tokens
                    .iter()
                    .find(|w| w.metadata.symbol == token.symbol_to_string())
                    .ok_or(CurrencyError::WalletNotSet)?;
                wallet
                    .validate_allowance(
                        from_principal,
                        subaccount,
                        amount,
                        memo,
                        created_at_time,
                    )
                    .await
            }
        }
    }

    pub async fn withdraw(
        &self,
        transaction_state: &mut TransactionState,
        currency: &Currency,
        wallet_principal_id: Principal,
        subaccount: Option<Vec<u8>>,
        amount: u64,
        memo: Option<Vec<u8>>,
        created_at_time: Option<u64>,
    ) -> Result<(), CurrencyError> {
        let (request_id, request_timestamp) = Self::build_request_id(
            "withdraw",
            currency,
            wallet_principal_id,
            &subaccount,
            amount,
            &memo,
            created_at_time,
        )?;
        Self::ensure_request_not_seen(transaction_state, &request_id, request_timestamp)?;

        let result = match currency {
            Currency::ICP => match &self.icp {
                Some(wallet) => {
                    wallet
                        .withdraw(wallet_principal_id, subaccount, amount, memo, created_at_time)
                        .await
                }
                None => Err(CurrencyError::WalletNotSet),
            },
            Currency::TestICP => match &self.test_icp {
                Some(test_icp) => {
                    test_icp
                        .withdraw(wallet_principal_id, subaccount, amount, memo, created_at_time)
                        .await
                }
                None => Err(CurrencyError::WalletNotSet),
            },
            Currency::CKETHToken(token) => {
                let wallet = self
                    .ckerc20_tokens
                    .iter()
                    .find(|w| w.config.token_symbol == Currency::CKETHToken(*token))
                    .ok_or(CurrencyError::WalletNotSet)?;
                wallet
                    .withdraw(wallet_principal_id, subaccount, amount, memo, created_at_time)
                    .await
            }
            Currency::BTC => match &self.btc {
                Some(wallet) => {
                    wallet
                        .withdraw(wallet_principal_id, subaccount, amount, memo, created_at_time)
                        .await
                }
                None => Err(CurrencyError::WalletNotSet),
            },
            Currency::GenericICRC1(token) => {
                let wallet = self
                    .generic_icrc1_tokens
                    .iter()
                    .find(|w| w.metadata.symbol == token.symbol_to_string())
                    .ok_or(CurrencyError::WalletNotSet)?;
                wallet
                    .withdraw(wallet_principal_id, subaccount, amount, memo, created_at_time)
                    .await
            }
        };

        if result.is_err() {
            transaction_state.remove_transaction(&request_id);
        }

        result
    }

    pub async fn get_balance(&self, currency: &Currency, principal_id: Principal) -> Result<u128, CurrencyError> {
        match currency {
            Currency::ICP => match &self.icp {
                Some(wallet) => wallet.get_balance(principal_id).await,
                None => Err(CurrencyError::WalletNotSet),
            },
            Currency::TestICP => match &self.test_icp {
                Some(test_icp) => test_icp.get_balance(principal_id).await,
                None => Err(CurrencyError::WalletNotSet),
            },
            Currency::CKETHToken(token) => {
                let wallet = self
                    .ckerc20_tokens
                    .iter()
                    .find(|w| w.config.token_symbol == Currency::CKETHToken(*token))
                    .ok_or(CurrencyError::WalletNotSet)?;
                wallet.get_balance(principal_id).await
            }
            Currency::BTC => match &self.btc {
                Some(wallet) => wallet.get_balance(principal_id).await,
                None => Err(CurrencyError::WalletNotSet),
            },
            Currency::GenericICRC1(token) => {
                let wallet = self
                    .generic_icrc1_tokens
                    .iter()
                    .find(|w| w.metadata.symbol == token.symbol_to_string())
                    .ok_or(CurrencyError::WalletNotSet)?;
                wallet.get_balance(principal_id).await
            }
        }
    }

    pub async fn get_fee(&self, currency: &Currency) -> Result<u128, CurrencyError> {
        match currency {
            Currency::ICP => match &self.icp {
                Some(_) => Ok(DEFAULT_FEE.e8s() as u128),
                None => Err(CurrencyError::WalletNotSet),
            },
            Currency::TestICP => match &self.test_icp {
                Some(_) => Ok(DEFAULT_FEE.e8s() as u128),
                None => Err(CurrencyError::WalletNotSet),
            },
            Currency::CKETHToken(token) => {
                let wallet = self
                    .ckerc20_tokens
                    .iter()
                    .find(|w| w.config.token_symbol == Currency::CKETHToken(*token))
                    .ok_or(CurrencyError::WalletNotSet)?;
                Ok(wallet.config.fee)
            }
            Currency::BTC => match &self.btc {
                Some(wallet) => Ok(wallet.config.fee),
                None => Err(CurrencyError::WalletNotSet),
            },
            Currency::GenericICRC1(token) => {
                let wallet = self
                    .generic_icrc1_tokens
                    .iter()
                    .find(|w| w.metadata.symbol == token.symbol_to_string())
                    .ok_or(CurrencyError::WalletNotSet)?;
                Ok(wallet.metadata.fee)
            }
        }
    }

    pub async fn approve_allowance(
        &self,
        currency: &Currency,
        spender_principal: Principal,
        subaccount: Option<Vec<u8>>,
        amount: u128,
        memo: Option<Vec<u8>>,
        created_at_time: Option<u64>,
    ) -> Result<(), CurrencyError> {
        match currency {
            Currency::ICP => match &self.icp {
                Some(wallet) => {
                    wallet
                        .approve(spender_principal, amount, subaccount, memo, created_at_time)
                        .await
                }
                None => Err(CurrencyError::WalletNotSet),
            },
            Currency::TestICP => match &self.test_icp {
                Some(test_icp) => {
                    test_icp
                        .approve(spender_principal, amount, subaccount, memo, created_at_time)
                        .await
                }
                None => Err(CurrencyError::WalletNotSet),
            },
            Currency::CKETHToken(token) => {
                let wallet = self
                    .ckerc20_tokens
                    .iter()
                    .find(|w| w.config.token_symbol == Currency::CKETHToken(*token))
                    .ok_or(CurrencyError::WalletNotSet)?;
                wallet
                    .approve(
                        wallet.config.ledger_id,
                        spender_principal,
                        amount,
                        subaccount,
                        memo,
                        created_at_time,
                    )
                    .await
            }
            Currency::BTC => match &self.btc {
                Some(wallet) => {
                    wallet
                        .approve(spender_principal, amount, subaccount, memo, created_at_time)
                        .await
                }
                None => Err(CurrencyError::WalletNotSet),
            },
            Currency::GenericICRC1(token) => {
                let wallet = self
                    .generic_icrc1_tokens
                    .iter()
                    .find(|w| w.metadata.symbol == token.symbol_to_string())
                    .ok_or(CurrencyError::WalletNotSet)?;
                wallet
                    .approve(spender_principal, amount, subaccount, memo, created_at_time)
                    .await
            }
        }
    }
}
