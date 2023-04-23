# ic-web3
RPC client for canisters on the Internet Computer to access Ethereum networks, powered by the Internet Computer's threshold ECDSA signature and outbound http call features.

[![Crates.io](https://img.shields.io/crates/v/ic-web3)](https://crates.io/crates/ic-web3)

### Features

* Perform RPC calls to Ethereum networks within canisters
* Sign messages with IC's threshold ECDSA
* Send transactions to Ethereum networks within canisters
* Query/call Ethereum contracts within canisters

### Usage

Add the following to your `Cargo.toml`:

```
[dependencies]
ic-web3 = { git = "https://github.com/rocklabs-io/ic-web3" }
```

**Note:** It requires ipv6 compatibility for the RPC provider. For ipv6 compitable providers, you can refer to this [list](  https://forum.dfinity.org/t/long-term-r-d-integration-with-the-ethereum-network/9382/81).


### Examples

Note: you should have dfx 0.11.2 or above.

Please refer to [example](./examples/example.rs) for the complete example.

```rust
use candid::candid_method;
use ic_cdk_macros::{self, update};
use std::str::FromStr;

use ic_web3::transports::ICHttp;
use ic_web3::Web3;
use ic_web3::ic::{get_eth_addr, KeyInfo};
use ic_web3::{
    contract::{Contract, Options},
    ethabi::ethereum_types::{U64, U256},
    types::{Address, TransactionParameters, BlockId, BlockNumber, Block},
};

const URL: &str = "<RPC-URL>"; / https://ethereum.publicnode.com
const CHAIN_ID: u64 = 1;
const KEY_NAME: &str = "dfx_test_key";
const TOKEN_ABI: &[u8] = include_bytes!("../src/contract/res/token.json");

type Result<T, E> = std::result::Result<T, E>;

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
        Ok(addr) => { Ok(hex::encode(addr)) },
        Err(e) => { Err(e) },
    }
}

// send tx to eth
#[update(name = "send_eth")]
#[candid_method(update, rename = "send_eth")]
async fn send_eth(to: String, value: u64) -> Result<String, String> {
    // ecdsa key info
    let derivation_path = vec![ic_cdk::id().as_slice().to_vec()];
    let key_info = KeyInfo{ derivation_path: derivation_path, key_name: KEY_NAME.to_string() };

    // get canister eth address
    let from_addr = get_eth_addr(None, None, KEY_NAME.to_string())
        .await
        .map_err(|e| format!("get canister eth addr failed: {}", e))?;
    // get canister the address tx count
    let w3 = match ICHttp::new(URL, None) {
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
        .sign_transaction(tx, key_info, CHAIN_ID)
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
```

Start a local replica:

```
dfx start --background --clean --enable-canister-http
```

Deploy the example canister:

```
dfx deploy
```

### Endpoint Canister

The public endpoint canister is deployed at: `3ondx-siaaa-aaaam-abf3q-cai`, [code](./examples/endpoint.rs). You can access Ethereum Mainnet data by passing RPC calls to the endpoint canister.

### Acknowledgment

This repo is modified from the [rust-web3](https://github.com/tomusdrw/rust-web3) project.
