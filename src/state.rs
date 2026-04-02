use candid::{CandidType, Decode, Encode};
use ic_stable_structures::{storable::Bound, Storable};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, collections::HashMap};

const REMOVE_PERCENTAGE: usize = 20;
const MAX_VALUE_SIZE_TRANSACTION_STATE: u32 = 2_000_000;

#[derive(Debug, Clone, PartialEq, CandidType, Deserialize, Serialize)]
pub struct TransactionState {
    processed_transactions: HashMap<String, u64>,
}

impl Default for TransactionState {
    fn default() -> Self {
        Self::new()
    }
}

impl TransactionState {
    pub fn new() -> TransactionState {
        TransactionState {
            processed_transactions: HashMap::new(),
        }
    }

    pub fn add_transaction(&mut self, transaction_id: String, timestamp: u64) {
        if self.processed_transactions.len() >= MAX_VALUE_SIZE_TRANSACTION_STATE as usize / 100 {
            let mut transactions: Vec<(u64, String)> = self
                .processed_transactions
                .iter()
                .map(|(tx_id, timestamp)| (*timestamp, tx_id.clone()))
                .collect();

            // Sort by timestamp (oldest first)
            transactions.sort_by_key(|(timestamp, _)| *timestamp);

            // Remove oldest transactions
            let remove_count = (self.processed_transactions.len() * REMOVE_PERCENTAGE) / 100;
            let keep_transactions: HashMap<String, u64> = transactions
                .into_iter()
                .skip(remove_count) // Skip oldest transactions
                .map(|(timestamp, tx_id)| (tx_id, timestamp))
                .collect();

            self.processed_transactions = keep_transactions;
        }

        self.processed_transactions.insert(transaction_id, timestamp);
    }

    pub fn transaction_exists(&self, transaction_id: &str) -> bool {
        self.processed_transactions.contains_key(transaction_id)
    }

    pub fn check_and_record(&mut self, transaction_id: String, timestamp: u64) -> bool {
        if self.transaction_exists(&transaction_id) {
            return false;
        }

        self.add_transaction(transaction_id, timestamp);
        true
    }

    pub fn remove_transaction(&mut self, transaction_id: &str) {
        self.processed_transactions.remove(transaction_id);
    }
}

impl Storable for TransactionState {
    /// Serializes the struct into a byte array.
    fn to_bytes(&self) -> std::borrow::Cow<'_, [u8]> {
        match Encode!(self) {
            Ok(bytes) => Cow::Owned(bytes),
            Err(e) => {
                ic_cdk::println!("Serialization error: {:?}", e);
                // Return empty state bytes rather than empty vec
                let empty_state = TransactionState::new();
                Cow::Owned(Encode!(&empty_state).unwrap_or_else(|_| vec![]))
            }
        }
    }

    /// Deserializes the struct from a byte array.
    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap_or_else(|e| {
            ic_cdk::println!("Deserialization error: {:?}", e);
            TransactionState::default()
        })
    }

    fn into_bytes(self) -> Vec<u8> {
        match Encode!(&self) {
            Ok(bytes) => bytes,
            Err(e) => {
                ic_cdk::println!("into_bytes serialization error: {:?}", e);
                let empty_state = TransactionState::new();
                Encode!(&empty_state).unwrap_or_else(|_| vec![])
            }
        }
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: MAX_VALUE_SIZE_TRANSACTION_STATE,
        is_fixed_size: false,
    };
}

#[cfg(test)]
mod tests {
    use super::TransactionState;

    #[test]
    fn check_and_record_rejects_duplicates() {
        let mut state = TransactionState::new();

        assert!(state.check_and_record("request-a".to_string(), 1));
        assert!(!state.check_and_record("request-a".to_string(), 1));
    }

    #[test]
    fn remove_transaction_allows_retry() {
        let mut state = TransactionState::new();

        assert!(state.check_and_record("request-a".to_string(), 1));
        state.remove_transaction("request-a");
        assert!(state.check_and_record("request-a".to_string(), 1));
    }
}
