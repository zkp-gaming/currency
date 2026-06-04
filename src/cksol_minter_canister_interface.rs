use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct GetDepositAddressArg {
    pub owner: Option<Principal>,
    pub subaccount: Option<Vec<u8>>,
}

#[derive(Debug, Clone, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct ProcessDepositArg {
    pub owner: Option<Principal>,
    pub subaccount: Option<Vec<u8>>,
    pub signature: String,
}

#[derive(Debug, Clone, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct CKSOLMinterInfo {
    pub process_deposit_required_cycles: u128,
}

#[derive(Debug, Clone, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub enum ProcessDepositSuccess {
    Minted {
        block_index: u64,
        minted_amount: u64,
        deposit_id: u64,
    },
}

#[derive(Debug, Clone, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct WithdrawToSolArg {
    pub address: String,
    pub amount: u64,
    pub from_subaccount: Option<Vec<u8>>,
}

#[derive(Debug, Clone, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct WithdrawToSolSuccess {
    pub block_index: u64,
}

#[derive(Debug, Clone, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub struct WithdrawalStatusArg {
    pub block_index: u64,
}

#[derive(Debug, Clone, CandidType, Deserialize, Serialize, PartialEq, Eq)]
pub enum CKSOLWithdrawalStatus {
    Pending,
    TxSent {
        signature: String,
    },
    TxFinalized {
        signature: String,
    },
    Failed(String),
}
