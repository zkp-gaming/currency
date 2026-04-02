#!/bin/bash

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
POCKET_IC_BIN="${REPO_ROOT}/tests/pocket-ic"
POCKET_IC_SERVER_VERSION="13.0.0"
WASM_DIR="${REPO_ROOT}/tests/wasms"
LOCAL_CANISTER_TARGET_DIR="${REPO_ROOT}/tests/target/canisters"
LOCAL_CANISTER_MANIFEST="${REPO_ROOT}/tests/canisters/currency_manager_host/Cargo.toml"
DEFAULT_IC_RELEASE="03dd6ee6de80c2202f66948692c69c61eb6af54d"
IC_RELEASE="${IC_RELEASE:-$DEFAULT_IC_RELEASE}"
BASE_URL="https://download.dfinity.systems/ic/${IC_RELEASE}/canisters"

CANISTER_FILES=(
  "ledger-canister_notify-method"
  "ic-icrc1-ledger"
  "ic-icrc1-ledger-u256"
)

if [[ "${OSTYPE:-}" == "linux-gnu"* ]] || [[ "${RUNNER_OS:-}" == "Linux" ]]; then
  PLATFORM="linux"
elif [[ "${OSTYPE:-}" == "darwin"* ]] || [[ "${RUNNER_OS:-}" == "macOS" ]]; then
  PLATFORM="darwin"
else
  echo "OS not supported: ${OSTYPE:-${RUNNER_OS:-unknown}}"
  exit 1
fi

ensure_pocket_ic() {
  if [[ -f "${POCKET_IC_BIN}" ]]; then
    return
  fi

  mkdir -p "${REPO_ROOT}/tests"
  echo "PocketIC download starting"
  curl -Ls "https://github.com/dfinity/pocketic/releases/download/${POCKET_IC_SERVER_VERSION}/pocket-ic-x86_64-${PLATFORM}.gz" -o "${POCKET_IC_BIN}.gz"
  gzip -df "${POCKET_IC_BIN}.gz"
  chmod +x "${POCKET_IC_BIN}"
  echo "PocketIC download completed"
}

extract_etag() {
  local url="$1"
  curl -sSI "$url" | tr -d '\r' | awk 'BEGIN{IGNORECASE=1} /^etag:/ {sub(/^etag:[[:space:]]*/, "", $0); print; exit}'
}

ensure_remote_wasm() {
  local local_name="$1"
  local remote_name
  remote_name="$(resolve_remote_name "${local_name}")"
  local local_path="${WASM_DIR}/${local_name}.wasm.gz"
  local etag_path="${local_path}.etag"
  local url="${BASE_URL}/${remote_name}.wasm.gz"

  mkdir -p "${WASM_DIR}"

  local remote_etag
  remote_etag="$(extract_etag "${url}")"

  if [[ -f "${local_path}" && -n "${remote_etag}" && -f "${etag_path}" ]]; then
    local cached_etag
    cached_etag="$(cat "${etag_path}")"
    if [[ "${cached_etag}" == "${remote_etag}" ]]; then
      echo "${local_name} is current"
      return
    fi
  fi

  if [[ -f "${local_path}" && -z "${remote_etag}" ]]; then
    local tmp_file
    tmp_file="$(mktemp "${WASM_DIR}/${local_name}.XXXXXX")"
    curl -fsSL "${url}" -o "${tmp_file}"

    local current_hash
    local remote_hash
    current_hash="$(shasum -a 256 "${local_path}" | awk '{print $1}')"
    remote_hash="$(shasum -a 256 "${tmp_file}" | awk '{print $1}')"

    if [[ "${current_hash}" == "${remote_hash}" ]]; then
      rm -f "${tmp_file}"
      echo "${local_name} is current"
      return
    fi

    mv "${tmp_file}" "${local_path}"
    echo "${local_name} refreshed"
    return
  fi

  curl -fsSL "${url}" -o "${local_path}"
  if [[ -n "${remote_etag}" ]]; then
    printf '%s\n' "${remote_etag}" > "${etag_path}"
  else
    rm -f "${etag_path}"
  fi
  echo "${local_name} downloaded"
}

resolve_remote_name() {
  local local_name="$1"
  case "${local_name}" in
    ledger-canister_notify-method) printf '%s\n' "ledger-canister_notify-method" ;;
    ic-icrc1-ledger) printf '%s\n' "ic-icrc1-ledger" ;;
    ic-icrc1-ledger-u256) printf '%s\n' "ic-icrc1-ledger-u256" ;;
    *)
      echo "Unknown canister artifact: ${local_name}" >&2
      exit 1
      ;;
  esac
}

ensure_all_remote_wasms() {
  for canister_name in "${CANISTER_FILES[@]}"; do
    ensure_remote_wasm "${canister_name}"
  done
}

build_local_canisters() {
  echo "Building local PocketIC test canisters"
  cargo build \
    --target wasm32-unknown-unknown \
    --release \
    --target-dir "${LOCAL_CANISTER_TARGET_DIR}" \
    --manifest-path "${LOCAL_CANISTER_MANIFEST}"
}

ensure_pocket_ic
ensure_all_remote_wasms
build_local_canisters

export POCKET_IC_BIN

PASSED_TESTS=()
FAILED_TESTS=()
OVERALL_STATUS=0

run_and_track() {
  local label="$1"
  shift

  echo "Running test: ${label}"
  if env RUST_TEST_THREADS=1 "$@"; then
    PASSED_TESTS+=("${label}")
  else
    FAILED_TESTS+=("${label}")
    OVERALL_STATUS=1
  fi
}

if [[ $# -gt 0 ]]; then
  for test_name in "$@"; do
    run_and_track "${test_name}" cargo test --manifest-path "${REPO_ROOT}/tests/Cargo.toml" "${test_name}" -- --test-threads=1 --nocapture
  done
else
  run_and_track "all tests" cargo test --manifest-path "${REPO_ROOT}/tests/Cargo.toml" -- --test-threads=1 --nocapture
fi

echo ""
echo "===================="
echo "Test Run Summary"
echo "===================="

if [[ ${#PASSED_TESTS[@]} -gt 0 ]]; then
  echo "Passed:"
  for test in "${PASSED_TESTS[@]}"; do
    echo "  ✔ ${test}"
  done
else
  echo "No tests passed"
fi

if [[ ${#FAILED_TESTS[@]} -gt 0 ]]; then
  echo "Failed:"
  for test in "${FAILED_TESTS[@]}"; do
    echo "  ✖ ${test}"
  done
fi

exit "${OVERALL_STATUS}"
