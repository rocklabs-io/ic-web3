use std::{cell::RefCell};

use candid::{CandidType, Deserialize, Principal, candid_method};
use ic_cdk::api::management_canister::http_request::{TransformArgs, HttpResponse};
use ic_cdk_macros::*;
use ic_web3::{transports::ICHttp, Web3};

// when the both request and response are max 2MB, the const of http calls is about 0.5T
const MAX_CYCLES_REQUIRES: u128 = 500_000_000_000; // 0.5T

#[derive(CandidType, Deserialize, Clone)]
struct State {
    owner: Principal,
    url: String,
    api_key: String,
}

#[derive(CandidType, Deserialize)]
struct RpcCallArgs {
    url: Option<String>,
    max_response_bytes: Option<u64>,
    body: String,
}

impl Default for State {
    fn default() -> Self {
        Self { 
            owner: Principal::management_canister(), 
            url: Default::default(), 
            api_key: Default::default(),
        }
    }
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

#[init]
#[candid_method(init)]
fn init(url: String, api_key: String,) {
    let owner = ic_cdk::caller();
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        state.owner = owner;
        state.url = url;
        state.api_key = api_key;
    });
}

#[query(name = "transform")]
#[candid_method(query, rename = "transform")]
fn transform(response: TransformArgs) -> HttpResponse {
    let res = response.response;
    // remove header
    HttpResponse { status: res.status, headers: Vec::default(), body: res.body }
}

// get state info
#[query(name = "getInfo")]
#[candid_method(query, rename = "getInfo")]
fn get_info() -> State {
    STATE.with(|s| {
        let state = s.borrow();
        state.clone()
    })
}

// set url
#[update(name = "setUrl", guard = "is_owner")]
#[candid_method(update, rename = "setUrl")]
async fn set_url(url: String) -> bool {
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        state.url = url;
    });
    true
}

// set api key
#[update(name = "setAPIKey", guard = "is_owner")]
#[candid_method(update, rename = "setAPIKey")]
async fn set_api_key(api_key: String) -> bool {
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        state.url = api_key;
    });
    true
}

/// rpcCallPrivate only owner can call
/// private call will not charge cycles from caller
#[update(name = "rpcCallPrivate", guard = "is_owner")]
#[candid_method(update, rename = "rpcCallPrivate")]
async fn rpc_call_private(args: RpcCallArgs) -> Result<String, String> {
    let url_with_key = if let Some(url) = args.url {
        // if url provided, use the url
        url
    } else {
        // otherwise, use the url and key in this canister
        let (url, api_key) = STATE.with(|s| {
            let state = s.borrow();
            (state.url.clone(), state.api_key.clone())
        });
        format!("{}/{}", url.trim_matches(|x| x == '/' || char::is_whitespace(x)), api_key.trim())
    };
    
    let w3 = match ICHttp::new(url_with_key.as_str(), args.max_response_bytes, None) {
        Ok(v) => { Web3::new(v) },
        Err(e) => { return Err(e.to_string()) },
    };

    let res = w3.json_rpc_call(args.body.as_str()).await.map_err(|e| format!("{}", e))?;

    ic_cdk::println!("result: {}", res);

    Ok(format!("{}", res))
}

/// rpcCall anyone can call, but url must be provided
/// accept max 0.5T cycles, the rest will be refunded
#[update(name = "rpcCall")]
#[candid_method(update, rename = "rpcCall")]
async fn rpc_call(args: RpcCallArgs) -> Result<String, String> {
    let cycles_call = ic_cdk::api::call::msg_cycles_available128();
    if cycles_call < MAX_CYCLES_REQUIRES {
        return Err(format!("requires {} cycles, get {} cycles", MAX_CYCLES_REQUIRES, cycles_call));
    }
   
    let url_with_key = if let Some(url) = args.url {
        // if url provided, use the url
        url
    } else {
        return Err("url must be provided".to_string())
    };
    
    let w3 = match ICHttp::new(url_with_key.as_str(), args.max_response_bytes, None) {
        Ok(v) => { Web3::new(v) },
        Err(e) => { return Err(e.to_string()) },
    };

    let cycles_before = ic_cdk::api::canister_balance128();
    let call_res = w3.json_rpc_call(args.body.as_str()).await;
    let cycles_after = ic_cdk::api::canister_balance128();

    // add 0.001T cycles as buffer
    let cycles_charged = ic_cdk::api::call::msg_cycles_accept128(cycles_before - cycles_after + 1_000_000_000u128);
    ic_cdk::println!("cycles charged: {}", cycles_charged);
    
    let res = call_res.map_err(|e| format!("{}", e))?;
    ic_cdk::println!("result: {}", res);

    Ok(format!("{}", res))
}

fn is_owner() -> Result<(), String> {
    let owner = STATE.with(|s| {
        let state = s.borrow();
        state.owner
    });

    if owner == ic_cdk::api::caller() {
        Ok(())
    } else {
        Err("unauthor".to_string())
    }
}

#[pre_upgrade]
fn pre_upgrade() {
    let state = STATE.with(|s| {
        s.replace(State::default())
    });
    ic_cdk::storage::stable_save((state, )).expect("pre upgrade error");
}

#[post_upgrade]
fn post_upgrade() {
    let (state, ): (State, ) = ic_cdk::storage::stable_restore().expect("post upgrade error");
    STATE.with(|s| {
        s.replace(state);
    });
}

#[cfg(not(any(target_arch = "wasm32", test)))]
fn main() {
    candid::export_service!();
    std::print!("{}", __export_service());
}

#[cfg(any(target_arch = "wasm32", test))]
fn main() {}