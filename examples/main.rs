use candid::{CandidType, Principal, candid_method, export_service};
use ic_cdk::storage;
use ic_cdk_macros::{self, heartbeat, post_upgrade, pre_upgrade, query, update};
use serde::{Deserialize, Serialize};
use serde_json::{self, json, Value};
use std::cell::{RefCell, RefMut};
use hex::{FromHex, ToHex};
use std::str::FromStr;

use ic_web3::transports::ICHttp;
use ic_web3::Web3;
use ic_web3::ic::{get_eth_addr, get_public_key, KeyInfo};
// use ic_web3::tx_helpers::ic_sign;
use ic_web3::{
    ethabi::ethereum_types::U256,
    types::{Address, TransactionRequest, TransactionParameters},
};

//const url = "https://eth-mainnet.g.alchemy.com/v2/UZzgeJY-eQAovXu7aupjTx062NdxBNuB";
// goerli testnet
const URL: &str = "https://eth-goerli.g.alchemy.com/v2/0QCHDmgIEFRV48r1U1QbtOyFInib3ZAm";
const CHAIN_ID: u64 = 5;
const KEY_NAME: &str = "dfx_test_key";

type Result<T, E> = std::result::Result<T, E>;

// #[update(name = "get_eth_block")]
// #[candid_method(update, rename = "get_eth_block")]
// async fn get_eth_block() -> Result<Block, String> {
//     // let w3: Web3 = Web3::new(URL.to_string(), None);
//     // w3.eth_get_block_by_number("latest").await
//     let w3 = match ICHttp::new(URL, None) {
//         Ok(v) => { Web3::new(v) },
//         Err(e) => { return Err(e.to_string()) },
//     };
//     let gas_price = w3.eth().gas_price().await.map_err(|e| format!("get gas price failed: {}", e))?;
//     ic_cdk::println!("gas price: {}", gas_price);
//     Ok(format!("{}", gas_price))
// }

#[update(name = "get_eth_gas_price")]
#[candid_method(update, rename = "get_eth_gas_price")]
async fn get_eth_gas_price() -> Result<String, String> {
    let w3 = match ICHttp::new(URL, None) {
        Ok(v) => { Web3::new(v) },
        Err(e) => { return Err(e.to_string()) },
    };
    let gas_price = w3.eth().gas_price().await.map_err(|e| format!("get gas price failed: {}", e))?;
    ic_cdk::println!("gas price: {}", gas_price);
    Ok(format!("{}", gas_price))
}

// get canister's ethereum address
#[update(name = "get_canister_addr")]
#[candid_method(update, rename = "get_canister_addr")]
async fn get_canister_addr() -> Result<String, String> {
    match get_eth_addr(None, None, KEY_NAME.to_string()).await {
        Ok(addr) => { Ok("0x".to_string() + &hex::encode(addr.to_vec())) },
        Err(e) => { Err(e) },
    }
}

#[update(name = "get_eth_balance")]
#[candid_method(update, rename = "get_eth_balance")]
async fn get_eth_balance(addr: String) -> Result<String, String> {
    let w3 = match ICHttp::new(URL, None) {
        Ok(v) => { Web3::new(v) },
        Err(e) => { return Err(e.to_string()) },
    };
    let balance = w3.eth().balance(Address::from_str(&addr).unwrap(), None).await.map_err(|e| format!("get balance failed: {}", e))?;
    Ok(format!("{}", balance))
}

// send tx
#[update(name = "send_eth")]
#[candid_method(update, rename = "send_eth")]
async fn send_eth(to: String, value: u64) -> Result<String, String> {
    // ecdsa key info
    let derivation_path = vec![ic_cdk::caller().as_slice().to_vec()];

    // get canister eth address
    let from_addr = match get_eth_addr(None, None, "dfx_test_key".to_string()).await {
        Ok(addr) => { "0x".to_string() + &hex::encode(addr.to_vec()) },
        Err(e) => { return Err(e); },
    };
    // get canister the address tx count
    let w3 = match ICHttp::new(URL, None) {
        Ok(v) => { Web3::new(v) },
        Err(e) => { return Err(e.to_string()) },
    };
    let tx_count = match w3.eth().transaction_count(Address::from_str(&from_addr).unwrap(), None).await {
        Ok(v) => { 
            // ic_cdk::println!("tx count: {}", v);
            // u128::from_str_radix(&v.trim_start_matches("0x"), 16).unwrap()
            v // U256
        },
        Err(e) => { return Err("get tx count error".into()); },
    };
    ic_cdk::println!("canister eth address {} tx count: {}", from_addr, tx_count);
    // construct a transaction
    let to = Address::from_str(&to).unwrap();
    let tx = TransactionParameters {
        to: Some(to),
        nonce: Some(tx_count), // remember to fetch nonce first
        value: U256::from(value),
        gas_price: Some(U256::exp10(10)), // 10 gwei
        gas: U256::from(21000),
        ..Default::default()
    };
    // sign the transaction and get serialized transaction + signature
    let key_info = KeyInfo{ derivation_path: derivation_path, key_name: KEY_NAME.to_string() };
    let signed_tx = w3.accounts().sign_transaction(tx, key_info, CHAIN_ID).await.expect("sign tx error");
    match w3.eth().send_raw_transaction(signed_tx.raw_transaction).await {
        Ok(txhash) => { 
            ic_cdk::println!("txhash: {}", hex::encode(txhash.0));
            Ok(format!("{}", hex::encode(txhash.0)))
        },
        Err(e) => { Err(e.to_string()) },
    }
}

/* 
// send tx
#[update(name = "send_eth")]
#[candid_method(update, rename = "send_eth")]
async fn send_eth(to: String, value: u64) -> Result<String, String> {
    // ecdsa key info
    let derivation_path = vec![ic_cdk::caller().as_slice().to_vec()];

    // get canister eth address
    let from_addr = match get_eth_addr(None, None, "dfx_test_key".to_string()).await {
        Ok(addr) => { "0x".to_string() + &hex::encode(addr.to_vec()) },
        Err(e) => { return Err(e); },
    };
    // get canister the address tx count
    let w3 = match ICHttp::new(URL, None) {
        Ok(v) => { Web3::new(v) },
        Err(e) => { return Err(e.to_string()) },
    };
    let tx_count = match w3.eth().transaction_count(Address::from_str(&from_addr), None).await {
        Ok(v) => { 
            // ic_cdk::println!("tx count: {}", v);
            // u128::from_str_radix(&v.trim_start_matches("0x"), 16).unwrap()
            v // U256
        },
        Err(e) => { return Err(e); },
    };
    ic_cdk::println!("canister eth address {} tx count: {}", from_addr, tx_count);
    // construct a transaction
    let to = Address::from_str(&to);
    let tx = TransactionRequest {
        to: Some(to),
        nonce: tx_count, // remember to fetch nonce first
        value: Some(value),
        gas_price: 20 * 10u128.pow(9), // 20 gwei
        gas: 21000,
        data: vec![]
        ..Default::default()
    };
    // sign the transaction and get serialized transaction + signature
    let signed_tx = ic_sign(tx, derivation_path, KEY_NAME.to_string()).await?;
    match w3.eth().send_raw_transaction(signed_tx.raw_transaction).await {
        Ok(txhash) => { Ok(txhash.to_string()) },
        Err(e) => { Err(e.to_string()) },
    }
}*/

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