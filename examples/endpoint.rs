use std::{cell::RefCell};

use candid::{CandidType, Deserialize, Principal, candid_method};
use ic_cdk::api::management_canister::http_request::{TransformArgs, HttpResponse};
use ic_cdk_macros::*;
use ic_web3::{transports::ICHttp, Web3};

#[derive(CandidType, Deserialize, Clone)]
struct State {
    owner: Principal,
    url: String,
    api_key: String,
}

#[derive(CandidType, Deserialize)]
struct RpcCallArgs {
    max_response_bytes: Option<u64>,
    cycles: Option<u64>,
    body: String,
}

impl Default for State {
    fn default() -> Self {
        Self { 
            owner: Principal::management_canister(), 
            url: Default::default(), 
            api_key: Default::default() 
        }
    }
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

#[init]
#[candid_method(init)]
fn init(url: String, api_key: String) {
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

#[update(name = "rpcCall")]
#[candid_method(update, rename = "rpcCall")]
async fn rpc_call(args: RpcCallArgs) -> Result<String, String> {
    let (url, api_key) = STATE.with(|s| {
        let state = s.borrow();
        (state.url.clone(), state.api_key.clone())
    });
    let url_with_key = format!("{}/{}", url.trim_matches(|x| x == '/' || char::is_whitespace(x)), api_key.trim());
    let w3 = match ICHttp::new(url_with_key.as_str(), args.max_response_bytes, args.cycles) {
        Ok(v) => { Web3::new(v) },
        Err(e) => { return Err(e.to_string()) },
    };

    let res = w3.json_rpc_call(args.body.as_str()).await.map_err(|e| format!("{}", e))?;

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