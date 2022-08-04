use candid::{CandidType, Principal, candid_method, export_service};
use ic_cdk::storage;
use ic_cdk_macros::{self, heartbeat, post_upgrade, pre_upgrade, query, update};
use serde::{Deserialize, Serialize};
use serde_json::{self, json, Value};
use std::cell::{RefCell, RefMut};
use hex::FromHex;
// use ethereum_tx_sign::{LegacyTransaction, Transaction};
// use ic_web3::model::{Block, JsonRpcResult};
// use ic_web3::web3::Web3;
// use ic_web3::ecdsa::*;

//const url = "https://eth-mainnet.g.alchemy.com/v2/UZzgeJY-eQAovXu7aupjTx062NdxBNuB";
// goerli testnet
const URL: &str = "https://eth-goerli.g.alchemy.com/v2/0QCHDmgIEFRV48r1U1QbtOyFInib3ZAm";
const CHAIN_ID: u64 = 5;
const KEY_NAME: &str = "dfx_test_key";

// #[update(name = "get_eth_block")]
// #[candid_method(update, rename = "get_eth_block")]
// async fn get_eth_block() -> Result<Block, String> {
//     let w3: Web3 = Web3::new(URL.to_string(), None);
//     w3.eth_get_block_by_number("latest").await
// }

#[update(name = "get_eth_gas_price")]
#[candid_method(update, rename = "get_eth_gas_price")]
async fn get_eth_gas_price() -> Result<String, String> {
    // let w3: Web3 = Web3::new(URL.to_string(), None);
    // w3.eth_gas_price().await
    let ic_http = web3::transports::ICHttp::new(URL, None).map_err(|e| "init ic http transport failed".to_string())?;
    let web3 = web3::Web3::new(ic_http);
    let gas_price = web3.eth().gas_price().await.map_err(|e| format!("get gas price failed: {}", e))?;
    ic_cdk::println!("gas price: {}", gas_price);
    Ok(format!("{}", gas_price))
}

// // get canister's ethereum address
// #[update(name = "get_canister_addr")]
// #[candid_method(update, rename = "get_canister_addr")]
// async fn get_canister_addr() -> Result<String, String> {
//     match get_eth_addr(None, None, KEY_NAME.to_string()).await {
//         Ok(addr) => { Ok("0x".to_string() + &hex::encode(addr.to_vec())) },
//         Err(e) => { Err(e) },
//     }
// }

// #[update(name = "get_eth_balance")]
// #[candid_method(update, rename = "get_eth_balance")]
// async fn get_eth_balance(addr: String) -> Result<u64, String> {
//     let w3: Web3 = Web3::new(URL.to_string(), None);
//     match w3.eth_get_balance(&addr, None).await {
//         Ok(v) => { Ok(u64::from_str_radix(&v.trim_start_matches("0x"), 16).unwrap()) },
//         Err(e) => { Err(e) },
//     }
// }

// // send tx
// #[update(name = "send_eth")]
// #[candid_method(update, rename = "send_eth")]
// async fn send_eth(to: String, value: u64) -> Result<String, String> {
//     // ecdsa key info
//     let derivation_path = vec![ic_cdk::caller().as_slice().to_vec()];

//     // get canister eth address
//     let from_addr = match get_eth_addr(None, None, "dfx_test_key".to_string()).await {
//         Ok(addr) => { "0x".to_string() + &hex::encode(addr.to_vec()) },
//         Err(e) => { return Err(e); },
//     };
//     // get canister the address tx count
//     let w3: Web3 = Web3::new(URL.to_string(), None);
//     let tx_count = match w3.eth_get_transaction_count(&from_addr, None).await {
//         Ok(v) => { 
//             ic_cdk::println!("tx count: {}", v);
//             u128::from_str_radix(&v.trim_start_matches("0x"), 16).unwrap()
//         },
//         Err(e) => { return Err(e); },
//     };
//     ic_cdk::println!("canister eth address {} tx count: {}", from_addr, tx_count);
//     // construct a transaction
//     let to = if to.starts_with("0x") {
//         to.chars().skip(2).collect()
//     } else { to };
//     let to_addr = <[u8; 20]>::from_hex(to).expect("address decode failed");
//     let tx = LegacyTransaction {
//         chain: CHAIN_ID, // goerli chain id
//         nonce: tx_count, // remember to fetch nonce first
//         to: Some(to_addr),
//         value: value as u128,
//         gas_price: 20 * 10u128.pow(9), // 20 gwei
//         gas: 21000,
//         data: vec![]
//     };
//     // sign the transaction and get serialized transaction + signature
//     let tx_str = match sign_eth_tx(&tx, derivation_path, KEY_NAME.to_string()).await {
//         Ok(v) => { v },
//         Err(e) => { return Err(e); },
//     };
//     ic_cdk::println!("tx: {}", tx_str.clone());
//     // send the transaction
//     w3.eth_send_raw_transaction(&tx_str).await
// }

// call a contract, query & update
// #[update(name = "eth_call")]
// #[candid_method(update, rename = "eth_call")]
// async fn eth_call(data: Value) -> Result<String, String> {
//     let w3: Web3 = Web3::new(URL.to_string(), None);
//     w3.eth_call(data).await
// }


fn main() {
    candid::export_service!();
    std::print!("{}", __export_service());
}