use std::{cell::RefCell, collections::{HashSet, HashMap}};

use candid::{CandidType, Deserialize, Principal, candid_method};
use ic_cdk::api::management_canister::http_request::{TransformArgs, HttpResponse, CanisterHttpRequestArgument, HttpHeader, HttpMethod, TransformContext, TransformFunc, http_request};
use ic_cdk_macros::*;
use jsonrpc_core::Call;

const MIN_CYCLES_REQUIRED: u128 = 10_000_000_000; // 10B cycles minimum for each call
const SERVICE_FEE: u128 = 100_000_000; // 0.1B cycles for service fee

#[derive(CandidType, Deserialize)]
struct State {
    owner: Principal,
    controllers: HashSet<Principal>,
    registered: HashMap<Registered, String>,
}

#[derive(CandidType, Deserialize, Eq, PartialEq, Hash, Clone)]
struct Registered {
    chain_id: u64,
    api_provider: String,
}

#[derive(CandidType, Deserialize, Clone)]
enum RpcTarget {
    #[serde(rename = "registered")]
    Registered(Registered),
    #[serde(rename = "url_with_api_key")]
    UrlWithApiKey(String),
}

impl Default for State {
    fn default() -> Self {
        Self { 
            owner: Principal::management_canister(), 
            controllers: Default::default(),
            registered: Default::default(),
        }
    }
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

#[init]
#[candid_method(init)]
fn init() {
    let owner = ic_cdk::caller();
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        state.owner = owner;
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
#[query(name = "registrations")]
#[candid_method(query, rename = "registrations")]
fn registrations() -> Vec<Registered> {
    STATE.with(|s| {
        let state = s.borrow();
        state.registered.keys().cloned().collect::<Vec<Registered>>()
    })
}

// add controllers
#[update(name = "add_controller", guard = "is_owner")]
#[candid_method(update, rename = "add_controller")]
async fn add_controller(controller: Principal) {
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        state.controllers.insert(controller);
    });
}

// register api and key
#[update(name = "register_api_key", guard = "is_authorized")]
#[candid_method(update, rename = "register_api_key")]
async fn register_api_key(chain_id: u64, api_provider: String, url_with_key: String) {
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        state.registered.insert(Registered{
            chain_id,
            api_provider,
        }, url_with_key);
    });
}

// json rpc call
#[update(name = "json_rpc")]
#[candid_method(update, rename = "json_rpc")]
async fn json_rpc(payload: String, target: RpcTarget, max_response_bytes: Option<u64>) -> Result<String, String> {
    let cycles_call = ic_cdk::api::call::msg_cycles_available128();
    if cycles_call < MIN_CYCLES_REQUIRED {
        return Err(format!("requires at least 10B cycles, get {} cycles", cycles_call));
    }
    let request_body: Call = serde_json::from_str(payload.as_ref()).map_err(|e| format!("Fail to decode json body: {:?}", e))?;
    let max_resp = max_response_bytes.unwrap_or(get_default_max_response_bytes_by_call(&request_body));
    let cycles_estimated = calculate_required_cycles(payload.clone(), max_resp, target.clone());
    if cycles_call < cycles_estimated {
        return Err(format!("requires {} cycles, get {} cycles", cycles_estimated, cycles_call));
    }
    // charge cycles
    let cycles_charged = ic_cdk::api::call::msg_cycles_accept128(cycles_estimated);
    ic_cdk::println!("cycles charged: {}", cycles_charged);
   
    let url_with_key = match target {
        RpcTarget::Registered(registered) => {
            STATE.with(|s| {
                s.borrow().registered.get(&registered).cloned().unwrap_or_default()
            })
        }
        RpcTarget::UrlWithApiKey(url_with_api_key) => {
            url_with_api_key
        }
    };
    if url_with_key.is_empty() {
        return Err("url is empty".to_string())
    };

    let call_res = json_rpc_call(&request_body, url_with_key, max_resp).await;
    
    let res = call_res.map_err(|e| format!("{}", e))?;
    ic_cdk::println!("result: {}", res);

    Ok(format!("{}", res))
}

/// Call json rpc directly
pub async fn json_rpc_call(request_body: &Call, url: String, max_response_bytes: u64) -> Result<String, String> {
    let request_headers = vec![
            HttpHeader {
                name: "Content-Type".to_string(),
                value: "application/json".to_string(),
            },
        ];
    // call http
    let request = CanisterHttpRequestArgument {
        url: url,
        max_response_bytes: Some(max_response_bytes),
        method: HttpMethod::POST,
        headers: request_headers,
        body: Some(serde_json::to_vec(request_body).unwrap()),
        transform: Some(TransformContext {
            function: TransformFunc(candid::Func {
                    principal: ic_cdk::api::id(),
                    method: "transform".to_string(),
                }),
            context: vec![],
        }),
    };

    match http_request(request).await {
        Ok((result, )) => {
            Ok(String::from_utf8_lossy(result.body.as_ref()).to_string())
        }
        Err((r, m)) => {
            let message = format!("The http_request resulted into error. RejectionCode: {r:?}, Error: {m}");
            ic_cdk::api::print(message.clone());
            Err(message)
        }
    }
}

fn get_default_max_response_bytes_by_call(rpc_call: &Call) -> u64 {
    // TODO define the max response bytes by call method
    return 500_000;
}

fn is_owner() -> Result<(), String> {
    STATE.with(|s| {
        let state = s.borrow();
        if state.owner == ic_cdk::api::caller() {
            Ok(())
        } else {
            Err("unauthorized".to_string())
        }
    })
}

fn is_authorized() -> Result<(), String> {
    STATE.with(|s| {
        let state = s.borrow();
        let caller = ic_cdk::api::caller();
        if state.owner == caller || state.controllers.contains(&caller){
            Ok(())
        } else {
            Err("unauthorized".to_string())
        }
    })
}

// calculate the estimated cycles required
// refer to https://internetcomputer.org/docs/current/developer-docs/gas-cost
fn calculate_required_cycles(payload: String, max_response_bytes: u64, target: RpcTarget) -> u128 {
    let arg_raw = candid::utils::encode_args((payload, max_response_bytes, target)).expect("Failed to encode arguments.");
    // 1.2M is ingress message received
    // 2K per byte received in an ingress message
    // 400M is HTTPS outcall request
    // assuming ingress message size is almost the same size of http request size, 100K cycles per byte
    1_200_000u128 + 
        2_000u128 * arg_raw.len() as u128 + 
        400_000_000u128 + 
        100_000u128 * (arg_raw.len() as u128 + max_response_bytes as u128) +
        SERVICE_FEE
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