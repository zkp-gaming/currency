use std::{cell::RefCell, collections::HashMap};

use currency::cksol_minter_canister_interface::{
    CKSOLMinterInfo, CKSOLWithdrawalStatus, GetDepositAddressArg, ProcessDepositArg,
    ProcessDepositSuccess, WithdrawToSolArg, WithdrawToSolSuccess, WithdrawalStatusArg,
};
use ic_cdk_macros::{init, query, update};

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

#[derive(Default)]
struct State {
    process_deposit_required_cycles: u128,
    next_deposit_id: u64,
    next_withdrawal_block_index: u64,
    processed_signatures: HashMap<String, ProcessDepositSuccess>,
    withdrawal_statuses: HashMap<u64, CKSOLWithdrawalStatus>,
}

#[init]
fn init(args: CKSOLMinterInfo) {
    STATE.with(|state| {
        *state.borrow_mut() = State {
            process_deposit_required_cycles: args.process_deposit_required_cycles,
            next_deposit_id: 1,
            next_withdrawal_block_index: 1,
            processed_signatures: HashMap::new(),
            withdrawal_statuses: HashMap::new(),
        };
    });
}

#[query]
fn get_minter_info() -> CKSOLMinterInfo {
    STATE.with(|state| CKSOLMinterInfo {
        process_deposit_required_cycles: state.borrow().process_deposit_required_cycles,
    })
}

#[query]
fn get_deposit_address(args: GetDepositAddressArg) -> String {
    let owner = args.owner.unwrap_or_else(ic_cdk::api::msg_caller);
    let subaccount = args
        .subaccount
        .as_ref()
        .map(|bytes| bytes.iter().map(|byte| format!("{byte:02x}")).collect::<String>())
        .unwrap_or_else(|| "default".to_string());
    format!("sol:{}:{}", owner.to_text(), subaccount)
}

#[update]
fn process_deposit(args: ProcessDepositArg) -> Result<ProcessDepositSuccess, String> {
    let available_cycles = ic_cdk::api::msg_cycles_available();
    let required_cycles = STATE.with(|state| state.borrow().process_deposit_required_cycles);
    if available_cycles < required_cycles {
        return Err(format!(
            "insufficient cycles: required {required_cycles}, got {available_cycles}"
        ));
    }
    let _accepted = ic_cdk::api::msg_cycles_accept(required_cycles);

    STATE.with(|state| {
        let mut state = state.borrow_mut();
        if let Some(existing) = state.processed_signatures.get(&args.signature) {
            return Err(format!("signature already processed: {:?}", existing));
        }

        let deposit_id = state.next_deposit_id;
        state.next_deposit_id += 1;
        let result = ProcessDepositSuccess::Minted {
            block_index: 1_000 + deposit_id,
            minted_amount: 990_000_000,
            deposit_id,
        };
        state
            .processed_signatures
            .insert(args.signature, result.clone());
        Ok(result)
    })
}

#[update]
fn withdraw(args: WithdrawToSolArg) -> Result<WithdrawToSolSuccess, String> {
    if args.address.is_empty() {
        return Err("destination address must not be empty".to_string());
    }

    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let block_index = state.next_withdrawal_block_index;
        state.next_withdrawal_block_index += 1;

        let status = if args.address.contains("pending") {
            CKSOLWithdrawalStatus::Pending
        } else if args.address.contains("sent") {
            CKSOLWithdrawalStatus::TxSent {
                signature: format!("sent-signature-{block_index}"),
            }
        } else if args.address.contains("failed") {
            CKSOLWithdrawalStatus::Failed(format!("failed-signature-{block_index}"))
        } else {
            CKSOLWithdrawalStatus::TxFinalized {
                signature: format!("finalized-signature-{block_index}"),
            }
        };

        state.withdrawal_statuses.insert(block_index, status);
        Ok(WithdrawToSolSuccess { block_index })
    })
}

#[query]
fn withdrawal_status(args: WithdrawalStatusArg) -> CKSOLWithdrawalStatus {
    STATE.with(|state| {
        state
            .borrow()
            .withdrawal_statuses
            .get(&args.block_index)
            .cloned()
            .unwrap_or(CKSOLWithdrawalStatus::Pending)
    })
}
