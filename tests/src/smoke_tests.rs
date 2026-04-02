use candid::{decode_one, encode_one, Principal};

use crate::env::new_test_env;

#[test]
fn boots_expected_canisters_at_fixed_ids() {
    let env = new_test_env();

    assert_eq!(
        env.canister_ids.icp_ledger,
        Principal::from_text(crate::ICP_LEDGER_CANISTER_ID).unwrap()
    );
    assert_eq!(
        env.canister_ids.test_icp_ledger,
        Principal::from_text(crate::TEST_ICP_LEDGER_CANISTER_ID).unwrap()
    );
    assert_eq!(
        env.canister_ids.ckbtc_ledger,
        Principal::from_text(currency::types::constants::BTC_LEDGER_CANISTER_ID).unwrap()
    );
    assert_eq!(
        env.canister_ids.cketh_ledger,
        Principal::from_text(currency::types::constants::ETH_LEDGER_CANISTER_ID).unwrap()
    );
    assert_eq!(
        env.canister_ids.ckusdc_ledger,
        Principal::from_text(currency::types::constants::USDC_LEDGER_CANISTER_ID).unwrap()
    );
    assert_eq!(
        env.canister_ids.ckusdt_ledger,
        Principal::from_text(currency::types::constants::USDT_LEDGER_CANISTER_ID).unwrap()
    );
    assert_eq!(
        env.canister_ids.cksepoliaeth_ledger,
        Principal::from_text(crate::CKSEPOLIA_ETH_LEDGER_CANISTER_ID).unwrap()
    );
    assert_eq!(
        env.canister_ids.cksepoliausdc_ledger,
        Principal::from_text(crate::CKSEPOLIA_USDC_LEDGER_CANISTER_ID).unwrap()
    );
}

#[test]
fn installed_canisters_respond_to_lightweight_reads() {
    let env = new_test_env();

    let icp_symbol: String = decode_query(
        &env,
        env.canister_ids.icp_ledger,
        "icrc1_symbol",
        encode_one(()).unwrap(),
    );
    assert_eq!(icp_symbol, "ICP");

    let ckbtc_symbol: String = decode_query(
        &env,
        env.canister_ids.ckbtc_ledger,
        "icrc1_symbol",
        encode_one(()).unwrap(),
    );
    assert_eq!(ckbtc_symbol, "ckBTC");

    let ckusdc_symbol: String = decode_query(
        &env,
        env.canister_ids.ckusdc_ledger,
        "icrc1_symbol",
        encode_one(()).unwrap(),
    );
    assert_eq!(ckusdc_symbol, "ckUSDC");

    let ckusdt_symbol: String = decode_query(
        &env,
        env.canister_ids.ckusdt_ledger,
        "icrc1_symbol",
        encode_one(()).unwrap(),
    );
    assert_eq!(ckusdt_symbol, "ckUSDT");

    let cketh_symbol: String = decode_query(
        &env,
        env.canister_ids.cketh_ledger,
        "icrc1_symbol",
        encode_one(()).unwrap(),
    );
    assert_eq!(cketh_symbol, "ckETH");

    let cksepoliaeth_symbol: String = decode_query(
        &env,
        env.canister_ids.cksepoliaeth_ledger,
        "icrc1_symbol",
        encode_one(()).unwrap(),
    );
    assert_eq!(cksepoliaeth_symbol, "ckSepoliaETH");

    let cksepoliausdc_symbol: String = decode_query(
        &env,
        env.canister_ids.cksepoliausdc_ledger,
        "icrc1_symbol",
        encode_one(()).unwrap(),
    );
    assert_eq!(cksepoliausdc_symbol, "ckSepoliaUSDC");
}

fn decode_query<T: candid::CandidType + for<'de> candid::Deserialize<'de>>(
    env: &crate::TestEnv,
    canister_id: Principal,
    method: &str,
    payload: Vec<u8>,
) -> T {
    let result = env
        .pocket_ic
        .query_call(canister_id, Principal::anonymous(), method, payload)
        .unwrap_or_else(|err| panic!("query call `{method}` failed: {err:?}"));

    decode_one(&result).unwrap_or_else(|err| panic!("failed to decode reply from `{method}`: {err}"))
}
