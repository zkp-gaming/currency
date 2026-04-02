#![cfg(test)]

mod currency_manager_btc_tests;
mod currency_manager_cketh_tests;
mod currency_manager_ckusdc_tests;
mod currency_manager_ckusdt_tests;
mod currency_manager_icp_tests;
mod currency_manager_test_support;
mod currency_manager_test_icp_tests;
mod env;
mod smoke_tests;
mod utils;
mod wasms;

use candid::{encode_one, CandidType, Nat, Principal};
use currency::{
    ckusdc_canister_interface::{
        Account as IcrcAccount, ArchiveOptions, FeatureFlags, InitArgs as IcrcLedgerInitArgs,
        LedgerArgument,
    },
    types::constants::{BTC_LEDGER_CANISTER_ID, ETH_LEDGER_CANISTER_ID, USDC_LEDGER_CANISTER_ID, USDT_LEDGER_CANISTER_ID},
};
use ic_ledger_types::{AccountIdentifier, Tokens, DEFAULT_SUBACCOUNT};
use pocket_ic::{PocketIc, PocketIcBuilder};
use std::collections::{HashMap, HashSet};
use wasms::CanisterWasm;

pub const INIT_CYCLES_BALANCE: u128 = 10_000_000_000_000;
pub const TREASURY_ICP_BALANCE_E8S: u64 = 10_000_000_000_000;
pub const ICP_LEDGER_CANISTER_ID: &str = "ryjl3-tyaaa-aaaaa-aaaba-cai";
pub const TEST_ICP_LEDGER_CANISTER_ID: &str = "xafvr-biaaa-aaaai-aql5q-cai";

pub const CKSEPOLIA_ETH_LEDGER_CANISTER_ID: &str = "apia6-jaaaa-aaaar-qabma-cai";
pub const CKSEPOLIA_USDC_LEDGER_CANISTER_ID: &str = "yfumr-cyaaa-aaaar-qaela-cai";
pub const TREASURY_PRINCIPAL_SEED: &[u8] = b"currency-e2e-treasury";

pub struct TestEnv {
    pub pocket_ic: PocketIc,
    pub canister_ids: CanisterIds,
    pub minting_principal: Principal,
    pub treasury_principal: Principal,
}

pub struct CanisterIds {
    pub icp_ledger: Principal,
    pub test_icp_ledger: Principal,
    pub ckbtc_ledger: Principal,
    pub cketh_ledger: Principal,
    pub ckusdc_ledger: Principal,
    pub ckusdt_ledger: Principal,
    pub cksepoliaeth_ledger: Principal,
    pub cksepoliausdc_ledger: Principal,
    pub currency_manager_host: Principal,
}

#[derive(CandidType)]
struct NnsLedgerCanisterInitArgs {
    minting_account: String,
    initial_values: HashMap<String, Tokens>,
    send_whitelist: HashSet<Principal>,
    transfer_fee: Option<Tokens>,
}

impl Default for TestEnv {
    fn default() -> Self {
        Self::new()
    }
}

impl TestEnv {
    pub fn new() -> Self {
        let pocket_ic = PocketIcBuilder::new()
            .with_nns_subnet()
            .with_sns_subnet()
            .with_fiduciary_subnet()
            .with_bitcoin_subnet()
            .with_application_subnet()
            .build();

        let minting_principal = Principal::self_authenticating(b"currency-e2e-minter");
        let treasury_principal = Principal::self_authenticating(TREASURY_PRINCIPAL_SEED);

        let icp_ledger = install_canister_with_id(
            &pocket_ic,
            ICP_LEDGER_CANISTER_ID,
            icp_ledger_init_args(minting_principal, treasury_principal),
            wasms::ICP_LEDGER.clone(),
        );

        let test_icp_ledger = install_canister_with_id(
            &pocket_ic,
            TEST_ICP_LEDGER_CANISTER_ID,
            icp_ledger_init_args(minting_principal, treasury_principal),
            wasms::ICP_LEDGER.clone(),
        );

        let ckbtc_ledger = install_canister_with_id(
            &pocket_ic,
            BTC_LEDGER_CANISTER_ID,
            icrc1_ledger_init_args(
                "ckBTC",
                "ckBTC",
                8,
                10,
                minting_principal,
            ),
            wasms::ICRC1_LEDGER.clone(),
        );

        let cketh_ledger = install_canister_with_id(
            &pocket_ic,
            ETH_LEDGER_CANISTER_ID,
            icrc1_ledger_init_args(
                "ckETH",
                "ckETH",
                18,
                2_000_000_000_000,
                minting_principal,
            ),
            wasms::ICRC1_LEDGER_U256.clone(),
        );

        let ckusdc_ledger = install_canister_with_id(
            &pocket_ic,
            USDC_LEDGER_CANISTER_ID,
            icrc1_ledger_init_args(
                "ckUSDC",
                "ckUSDC",
                6,
                10_000,
                minting_principal,
            ),
            wasms::ICRC1_LEDGER.clone(),
        );

        let ckusdt_ledger = install_canister_with_id(
            &pocket_ic,
            USDT_LEDGER_CANISTER_ID,
            icrc1_ledger_init_args(
                "ckUSDT",
                "ckUSDT",
                6,
                10_000,
                minting_principal,
            ),
            wasms::ICRC1_LEDGER.clone(),
        );

        let cksepoliaeth_ledger = install_canister_with_id(
            &pocket_ic,
            CKSEPOLIA_ETH_LEDGER_CANISTER_ID,
            icrc1_ledger_init_args(
                "ckSepoliaETH",
                "Chain key Sepolia Ethereum",
                18,
                10_000_000_000,
                minting_principal,
            ),
            wasms::ICRC1_LEDGER_U256.clone(),
        );

        let cksepoliausdc_ledger = install_canister_with_id(
            &pocket_ic,
            CKSEPOLIA_USDC_LEDGER_CANISTER_ID,
            icrc1_ledger_init_args(
                "ckSepoliaUSDC",
                "Chain key Sepolia USDC",
                6,
                4_000,
                minting_principal,
            ),
            wasms::ICRC1_LEDGER_U256.clone(),
        );

        let currency_manager_host = pocket_ic.create_canister();
        pocket_ic.add_cycles(currency_manager_host, INIT_CYCLES_BALANCE);
        pocket_ic.install_canister(currency_manager_host, wasms::CURRENCY_MANAGER_HOST.clone(), vec![], None);

        Self {
            pocket_ic,
            canister_ids: CanisterIds {
                icp_ledger,
                test_icp_ledger,
                ckbtc_ledger,
                cketh_ledger,
                ckusdc_ledger,
                ckusdt_ledger,
                cksepoliaeth_ledger,
                cksepoliausdc_ledger,
                currency_manager_host,
            },
            minting_principal,
            treasury_principal,
        }
    }
}

fn install_canister_with_id<A: CandidType>(
    pocket_ic: &PocketIc,
    canister_id: &str,
    args: A,
    canister_wasm: CanisterWasm,
) -> Principal {
    let canister_id = Principal::from_text(canister_id).expect("invalid canister id");
    pocket_ic
        .create_canister_with_id(None, None, canister_id)
        .expect("failed to create canister with id");
    pocket_ic.add_cycles(canister_id, INIT_CYCLES_BALANCE);
    pocket_ic.install_canister(
        canister_id,
        canister_wasm,
        encode_one(args).expect("failed to encode install args"),
        None,
    );
    canister_id
}

fn icp_ledger_init_args(
    minting_principal: Principal,
    treasury_principal: Principal,
) -> NnsLedgerCanisterInitArgs {
    let minting_account = AccountIdentifier::new(&minting_principal, &DEFAULT_SUBACCOUNT);
    let treasury_account = AccountIdentifier::new(&treasury_principal, &DEFAULT_SUBACCOUNT);

    let mut initial_values = HashMap::new();
    initial_values.insert(
        treasury_account.to_string(),
        Tokens::from_e8s(TREASURY_ICP_BALANCE_E8S),
    );

    NnsLedgerCanisterInitArgs {
        minting_account: minting_account.to_string(),
        initial_values,
        send_whitelist: HashSet::new(),
        transfer_fee: Some(Tokens::from_e8s(10_000)),
    }
}

fn icrc1_ledger_init_args(
    token_symbol: &str,
    token_name: &str,
    decimals: u8,
    transfer_fee: u128,
    minting_owner: Principal,
) -> LedgerArgument {
    LedgerArgument::Init(IcrcLedgerInitArgs {
        decimals: Some(decimals),
        token_symbol: token_symbol.to_string(),
        transfer_fee: Nat::from(transfer_fee),
        metadata: vec![],
        minting_account: IcrcAccount {
            owner: minting_owner,
            subaccount: None,
        },
        initial_balances: vec![],
        maximum_number_of_accounts: None,
        accounts_overflow_trim_quantity: None,
        fee_collector_account: None,
        archive_options: ArchiveOptions {
            num_blocks_to_archive: 1_000,
            max_transactions_per_response: None,
            trigger_threshold: 2_000,
            more_controller_ids: None,
            max_message_size_bytes: None,
            cycles_for_archive_creation: Some(100_000_000_000_000),
            node_max_memory_size_bytes: Some(3_221_225_472),
            controller_id: minting_owner,
        },
        max_memo_length: Some(80),
        token_name: token_name.to_string(),
        feature_flags: Some(FeatureFlags { icrc2: true }),
    })
}
