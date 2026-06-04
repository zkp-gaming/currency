use std::{
    fs, io::Read,
    path::{Path, PathBuf},
};

use lazy_static::lazy_static;

pub type CanisterWasm = Vec<u8>;

lazy_static! {
    pub static ref ICP_LEDGER: CanisterWasm = get_remote_wasm("ledger-canister_notify-method");
    pub static ref ICRC1_LEDGER: CanisterWasm = get_remote_wasm("ic-icrc1-ledger");
    pub static ref ICRC1_LEDGER_U256: CanisterWasm = get_remote_wasm("ic-icrc1-ledger-u256");
    pub static ref CURRENCY_MANAGER_HOST: CanisterWasm = get_local_wasm("currency_manager_host");
    pub static ref CKSOL_MINTER_MOCK: CanisterWasm = get_local_wasm("cksol_minter_mock");
}

fn get_canister_wasm<P: AsRef<Path>>(path: P) -> CanisterWasm {
    let path = path.as_ref();

    let mut file = fs::File::open(path).unwrap_or_else(|e| {
        panic!("Failed to open file: {}, reason: {}", path.display(), e);
    });
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).unwrap_or_else(|e| {
        panic!("Failed to read file: {}, reason: {}", path.display(), e);
    });
    bytes
}

fn get_remote_wasm(canister_name: &str) -> CanisterWasm {
    let path = format!("wasms/{}.wasm.gz", canister_name);
    get_canister_wasm(path)
}

fn get_local_wasm(canister_name: &str) -> CanisterWasm {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/canisters/wasm32-unknown-unknown/release")
        .join(format!("{canister_name}.wasm"));

    get_canister_wasm(path)
}
