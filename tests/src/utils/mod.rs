use candid::{decode_one, encode_args, utils::ArgumentEncoder, CandidType, Deserialize, Nat, Principal};
use currency::{
    currency_error::CurrencyError,
    icrc1_types::{Account, Allowance, AllowanceArgs, ApproveArgs, ApproveError, TransferArg, TransferErrorIcrc1},
    types::currency::CKTokenSymbol,
    Currency,
};
use ic_ledger_types::{
    AccountIdentifier, Memo, Tokens, TransferArgs, TransferResult, DEFAULT_FEE,
    DEFAULT_SUBACCOUNT,
};

use crate::TestEnv;

#[derive(CandidType, Clone, candid::Deserialize, Debug, PartialEq, Eq)]
pub struct AccountView {
    pub owner: Principal,
    pub subaccount: Option<Vec<u8>>,
}

pub fn test_principal(label: &str) -> Principal {
    Principal::self_authenticating(label.as_bytes())
}

pub fn update_call<Args, Reply>(
    env: &TestEnv,
    canister_id: Principal,
    sender: Principal,
    method: &str,
    args: Args,
) -> Reply
where
    Args: ArgumentEncoder,
    Reply: for<'de> Deserialize<'de> + CandidType,
{
    let payload =
        encode_args(args).unwrap_or_else(|err| panic!("failed to encode `{method}` args: {err}"));
    let result = env
        .pocket_ic
        .update_call(canister_id, sender, method, payload)
        .unwrap_or_else(|err| panic!("update call `{method}` failed: {err:?}"));

    decode_one(&result).unwrap_or_else(|err| panic!("failed to decode `{method}` reply: {err}"))
}

pub fn query_call<Args, Reply>(
    env: &TestEnv,
    canister_id: Principal,
    sender: Principal,
    method: &str,
    args: Args,
) -> Reply
where
    Args: ArgumentEncoder,
    Reply: for<'de> Deserialize<'de> + CandidType,
{
    let payload =
        encode_args(args).unwrap_or_else(|err| panic!("failed to encode `{method}` args: {err}"));
    let result = env
        .pocket_ic
        .query_call(canister_id, sender, method, payload)
        .unwrap_or_else(|err| panic!("query call `{method}` failed: {err:?}"));

    decode_one(&result).unwrap_or_else(|err| panic!("failed to decode `{method}` reply: {err}"))
}

pub fn ledger_id_for_currency(env: &TestEnv, currency: Currency) -> Principal {
    match currency {
        Currency::ICP => env.canister_ids.icp_ledger,
        Currency::TestICP => env.canister_ids.test_icp_ledger,
        Currency::BTC => env.canister_ids.ckbtc_ledger,
        Currency::CKETHToken(CKTokenSymbol::USDC) => env.canister_ids.ckusdc_ledger,
        Currency::CKETHToken(CKTokenSymbol::USDT) => env.canister_ids.ckusdt_ledger,
        Currency::CKETHToken(CKTokenSymbol::ETH) => env.canister_ids.cketh_ledger,
        Currency::GenericICRC1(_) => panic!("GenericICRC1 is not covered in these tests"),
    }
}

pub fn fee_for_currency(currency: Currency) -> u128 {
    match currency {
        Currency::ICP | Currency::TestICP => DEFAULT_FEE.e8s() as u128,
        Currency::BTC => 10,
        Currency::CKETHToken(CKTokenSymbol::USDC) => 10_000,
        Currency::CKETHToken(CKTokenSymbol::USDT) => 10_000,
        Currency::CKETHToken(CKTokenSymbol::ETH) => 2_000_000_000_000,
        Currency::GenericICRC1(_) => panic!("GenericICRC1 is not covered in these tests"),
    }
}

pub fn fund_principal(env: &TestEnv, currency: Currency, recipient: Principal, amount: u64) {
    match currency {
        Currency::ICP | Currency::TestICP => fund_principal_with_icp_like(
            env,
            ledger_id_for_currency(env, currency),
            recipient,
            amount,
        ),
        Currency::BTC | Currency::CKETHToken(_) => fund_principal_with_icrc1(
            env,
            ledger_id_for_currency(env, currency),
            recipient,
            amount,
        ),
        Currency::GenericICRC1(_) => panic!("GenericICRC1 is not covered in these tests"),
    }
}

fn fund_principal_with_icp_like(
    env: &TestEnv,
    ledger_id: Principal,
    recipient: Principal,
    amount_e8s: u64,
) {
    let transfer_args = TransferArgs {
        memo: Memo(0),
        amount: Tokens::from_e8s(amount_e8s),
        fee: DEFAULT_FEE,
        from_subaccount: None,
        to: AccountIdentifier::new(&recipient, &DEFAULT_SUBACCOUNT),
        created_at_time: None,
    };

    let result: TransferResult = update_call(
        env,
        ledger_id,
        env.treasury_principal,
        "transfer",
        (transfer_args,),
    );
    result.unwrap_or_else(|err| panic!("treasury transfer failed: {err:?}"));
}

fn fund_principal_with_icrc1(
    env: &TestEnv,
    ledger_id: Principal,
    recipient: Principal,
    amount: u64,
) {
    let result: Result<u128, TransferErrorIcrc1> = update_call(
        env,
        ledger_id,
        env.minting_principal,
        "icrc1_transfer",
        (TransferArg {
            to: Account {
                owner: recipient,
                subaccount: None,
            },
            fee: None,
            memo: None,
            from_subaccount: None,
            created_at_time: None,
            amount: amount as u128,
        },),
    );
    result.unwrap_or_else(|err| panic!("minting transfer failed: {err:?}"));
}

pub fn approve_spender(env: &TestEnv, currency: Currency, owner: Principal, spender: Principal, amount: u128) {
    let approve_args = ApproveArgs {
        spender: Account {
            owner: spender,
            subaccount: None,
        },
        amount,
        expected_allowance: None,
        expires_at: None,
        fee: Some(fee_for_currency(currency)),
        memo: None,
        from_subaccount: None,
        created_at_time: None,
    };

    let result: Result<u128, ApproveError> = update_call(
        env,
        ledger_id_for_currency(env, currency),
        owner,
        "icrc2_approve",
        (approve_args,),
    );
    result.unwrap_or_else(|err| panic!("approve failed: {err:?}"));
}

pub fn allowance_of(env: &TestEnv, currency: Currency, owner: Principal, spender: Principal) -> Allowance {
    query_call(
        env,
        ledger_id_for_currency(env, currency),
        Principal::anonymous(),
        "icrc2_allowance",
        (AllowanceArgs {
            account: Account {
                owner,
                subaccount: None,
            },
            spender: Account {
                owner: spender,
                subaccount: None,
            },
        },),
    )
}

pub fn balance_of(
    env: &TestEnv,
    currency: Currency,
    owner: Principal,
    subaccount: Option<Vec<u8>>,
) -> u128 {
    let balance: Nat = query_call(
        env,
        ledger_id_for_currency(env, currency),
        Principal::anonymous(),
        "icrc1_balance_of",
        (Account { owner, subaccount },),
    );

    balance
        .0
        .to_string()
        .parse::<u128>()
        .unwrap_or_else(|err| panic!("failed to convert balance to u128: {err}"))
}

pub fn manager_canister_account(env: &TestEnv, currency: Currency) -> AccountView {
    query_call(
        env,
        env.canister_ids.currency_manager_host,
        Principal::anonymous(),
        "get_account_for_currency",
        (currency,),
    )
}

pub fn manager_get_fee(env: &TestEnv, currency: Currency) -> Result<u128, CurrencyError> {
    update_call(
        env,
        env.canister_ids.currency_manager_host,
        Principal::anonymous(),
        "get_fee",
        (currency,),
    )
}

pub fn manager_validate_allowance(
    env: &TestEnv,
    currency: Currency,
    from_principal: Principal,
    amount: u64,
) -> Result<(), CurrencyError> {
    update_call(
        env,
        env.canister_ids.currency_manager_host,
        Principal::anonymous(),
        "validate_allowance",
        (currency, from_principal, amount),
    )
}

pub fn manager_deposit(
    env: &TestEnv,
    currency: Currency,
    from_principal: Principal,
    amount: u64,
) -> Result<(), CurrencyError> {
    update_call(
        env,
        env.canister_ids.currency_manager_host,
        Principal::anonymous(),
        "deposit",
        (currency, from_principal, amount, Option::<Vec<u8>>::None, Option::<u64>::None),
    )
}

pub fn manager_get_balance(env: &TestEnv, currency: Currency) -> Result<u128, CurrencyError> {
    update_call(
        env,
        env.canister_ids.currency_manager_host,
        Principal::anonymous(),
        "get_balance",
        (currency,),
    )
}

pub fn manager_withdraw(
    env: &TestEnv,
    currency: Currency,
    to_principal: Principal,
    amount: u64,
) -> Result<(), CurrencyError> {
    update_call(
        env,
        env.canister_ids.currency_manager_host,
        Principal::anonymous(),
        "withdraw",
        (currency, to_principal, amount, Option::<Vec<u8>>::None, Option::<u64>::None),
    )
}

pub fn manager_approve_allowance(
    env: &TestEnv,
    currency: Currency,
    spender_principal: Principal,
    subaccount: Option<Vec<u8>>,
    amount: u128,
) -> Result<(), CurrencyError> {
    update_call(
        env,
        env.canister_ids.currency_manager_host,
        Principal::anonymous(),
        "approve_allowance",
        (
            currency,
            spender_principal,
            subaccount,
            amount,
            Option::<Vec<u8>>::None,
            Option::<u64>::None,
        ),
    )
}

pub fn default_subaccount() -> Vec<u8> {
    DEFAULT_SUBACCOUNT.0.to_vec()
}
