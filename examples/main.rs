use candid::candid_method;
use ic_cdk_macros::{self, update};
use std::str::FromStr;

use ic_web3::transports::ICHttp;
use ic_web3::Web3;
use ic_web3::ic::{get_eth_addr, KeyInfo};
use ic_web3::{
    contract::{Contract, Options},
    ethabi::ethereum_types::{U64, U256},
    types::{Address, TransactionParameters, BlockId, BlockNumber},
};

// goerli testnet rpc url
const URL: &str = "https://goerli.infura.io/v3/93ca33aa55d147f08666ac82d7cc69fd";
const CHAIN_ID: u64 = 5;
const KEY_NAME: &str = "dfx_test_key";
const TOKEN_ABI: &[u8] = include_bytes!("../src/contract/res/token.json");

type Result<T, E> = std::result::Result<T, E>;

#[update(name = "get_block")]
#[candid_method(update, rename = "get_block")]
async fn get_block(number: Option<u64>) -> Result<String, String> {
    let w3 = match ICHttp::new(URL, None, None) {
        Ok(v) => { Web3::new(v) },
        Err(e) => { return Err(e.to_string()) },
    };
    let block_id = match number {
        Some(id) => { BlockId::from(U64::from(id)) },
        None => { BlockId::Number(BlockNumber::Latest) },
    };
    let block = w3.eth().block(block_id).await.map_err(|e| format!("get block error: {}", e))?;
    ic_cdk::println!("block: {:?}", block.clone().unwrap());

    Ok(serde_json::to_string(&block.unwrap()).unwrap())
}

#[update(name = "get_eth_gas_price")]
#[candid_method(update, rename = "get_eth_gas_price")]
async fn get_eth_gas_price() -> Result<String, String> {
    let w3 = match ICHttp::new(URL, None, None) {
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
        Ok(addr) => { Ok(hex::encode(addr)) },
        Err(e) => { Err(e) },
    }
}

#[update(name = "get_eth_balance")]
#[candid_method(update, rename = "get_eth_balance")]
async fn get_eth_balance(addr: String) -> Result<String, String> {
    let w3 = match ICHttp::new(URL, None, None) {
        Ok(v) => { Web3::new(v) },
        Err(e) => { return Err(e.to_string()) },
    };
    let balance = w3.eth().balance(Address::from_str(&addr).unwrap(), None).await.map_err(|e| format!("get balance failed: {}", e))?;
    Ok(format!("{}", balance))
}

#[update(name = "batch_request")]
#[candid_method(update, rename = "batch_request")]
async fn batch_request() -> Result<String, String> {
    let http = ICHttp::new(URL, None, None).map_err(|e| format!("init ICHttp failed: {}", e))?;
    let w3 = Web3::new(ic_web3::transports::Batch::new(http));

    let block_number = w3.eth().block_number();
    let gas_price = w3.eth().gas_price();
    let balance = w3.eth().balance(Address::from([0u8; 20]), None);

    let result = w3.transport().submit_batch().await.map_err(|e| format!("batch request err: {}", e))?;
    ic_cdk::println!("batch request result: {:?}", result);

    let block_number = block_number.await.map_err(|e| format!("get block number err: {}", e))?;
    ic_cdk::println!("block number: {:?}", block_number);

    let gas_price = gas_price.await.map_err(|e| format!("get gas price err: {}", e))?;
    ic_cdk::println!("gas price: {:?}", gas_price);

    let balance = balance.await.map_err(|e| format!("get balance err: {}", e))?;
    ic_cdk::println!("balance: {:?}", balance);

    Ok("done".into())
}

// send tx to eth
#[update(name = "send_eth")]
#[candid_method(update, rename = "send_eth")]
async fn send_eth(to: String, value: u64) -> Result<String, String> {
    // ecdsa key info
    let derivation_path = vec![ic_cdk::id().as_slice().to_vec()];
    let key_info = KeyInfo{ derivation_path: derivation_path, key_name: KEY_NAME.to_string(), ecdsa_sign_cycles: None };

    // get canister eth address
    let from_addr = get_eth_addr(None, None, KEY_NAME.to_string())
        .await
        .map_err(|e| format!("get canister eth addr failed: {}", e))?;
    // get canister the address tx count
    let w3 = match ICHttp::new(URL, None, None) {
        Ok(v) => { Web3::new(v) },
        Err(e) => { return Err(e.to_string()) },
    };
    let tx_count = w3.eth()
        .transaction_count(from_addr, None)
        .await
        .map_err(|e| format!("get tx count error: {}", e))?;
        
    ic_cdk::println!("canister eth address {} tx count: {}", hex::encode(from_addr), tx_count);
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
    let signed_tx = w3.accounts()
        .sign_transaction(tx, hex::encode(from_addr), key_info, CHAIN_ID)
        .await
        .map_err(|e| format!("sign tx error: {}", e))?;
    match w3.eth().send_raw_transaction(signed_tx.raw_transaction).await {
        Ok(txhash) => { 
            ic_cdk::println!("txhash: {}", hex::encode(txhash.0));
            Ok(format!("{}", hex::encode(txhash.0)))
        },
        Err(e) => { Err(e.to_string()) },
    }
}

// query a contract, token balance
#[update(name = "token_balance")]
#[candid_method(update, rename = "token_balance")]
async fn token_balance(contract_addr: String, addr: String) -> Result<String, String> {
    // goerli weth: 0xb4fbf271143f4fbf7b91a5ded31805e42b2208d6
    // account: 0x9c9fcF808B82e5fb476ef8b7A1F5Ad61Dc597625
    let w3 = match ICHttp::new(URL, None, None) {
        Ok(v) => { Web3::new(v) },
        Err(e) => { return Err(e.to_string()) },
    };
    let contract_address = Address::from_str(&contract_addr).unwrap();
    let contract = Contract::from_json(
        w3.eth(),
        contract_address,
        TOKEN_ABI
    ).map_err(|e| format!("init contract failed: {}", e))?;

    let addr = Address::from_str(&addr).unwrap();
    let balance: U256 = contract
        .query("balanceOf", (addr,), None, Options::default(), None)
        .await
        .map_err(|e| format!("query contract error: {}", e))?;
    ic_cdk::println!("balance of {} is {}", addr, balance);
    Ok(format!("{}", balance))
}

// call a contract, transfer some token to addr
#[update(name = "send_token")]
#[candid_method(update, rename = "send_token")]
async fn send_token(token_addr: String, addr: String, value: u64) -> Result<String, String> {
    // ecdsa key info
    let derivation_path = vec![ic_cdk::id().as_slice().to_vec()];
    let key_info = KeyInfo{ derivation_path: derivation_path, key_name: KEY_NAME.to_string(), ecdsa_sign_cycles: None };

    let w3 = match ICHttp::new(URL, None, None) {
        Ok(v) => { Web3::new(v) },
        Err(e) => { return Err(e.to_string()) },
    };
    let contract_address = Address::from_str(&token_addr).unwrap();
    let contract = Contract::from_json(
        w3.eth(),
        contract_address,
        TOKEN_ABI
    ).map_err(|e| format!("init contract failed: {}", e))?;

    let canister_addr = get_eth_addr(None, None, KEY_NAME.to_string())
        .await
        .map_err(|e| format!("get canister eth addr failed: {}", e))?;
    // add nonce to options
    let tx_count = w3.eth()
        .transaction_count(canister_addr, None)
        .await
        .map_err(|e| format!("get tx count error: {}", e))?;
    // get gas_price
    let gas_price = w3.eth()
        .gas_price()
        .await
        .map_err(|e| format!("get gas_price error: {}", e))?;
    // legacy transaction type is still ok
    let options = Options::with(|op| { 
        op.nonce = Some(tx_count);
        op.gas_price = Some(gas_price);
        op.transaction_type = Some(U64::from(2)) //EIP1559_TX_ID
    });
    let to_addr = Address::from_str(&addr).unwrap();
    let txhash = contract
        .signed_call("transfer", (to_addr, value,), options, hex::encode(canister_addr), key_info, CHAIN_ID)
        .await
        .map_err(|e| format!("token transfer failed: {}", e))?;

    ic_cdk::println!("txhash: {}", hex::encode(txhash));

    Ok(format!("{}", hex::encode(txhash)))
}


fn main() {
    candid::export_service!();
    std::print!("{}", __export_service());
}
