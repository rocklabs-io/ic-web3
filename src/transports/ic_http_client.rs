use serde::{self, Deserialize, Serialize};
use candid::CandidType;
use jsonrpc_core::Request;
use candid::Principal;

#[derive(CandidType, Clone, Deserialize, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct HttpHeader {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, CandidType, Eq, Hash, Serialize, Deserialize)]
pub enum HttpMethod {
    GET,
    POST,
    HEAD,
}

#[derive(CandidType, Deserialize, Debug)]
pub struct CanisterHttpRequestArgs {
    pub url: String,
    pub max_response_bytes: Option<u64>,
    pub headers: Vec<HttpHeader>,
    pub body: Option<Vec<u8>>,
    pub http_method: HttpMethod,
    pub transform_method_name: Option<String>,
}

#[derive(CandidType, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CanisterHttpResponsePayload {
    pub status: u64,
    pub headers: Vec<HttpHeader>,
    pub body: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct ICHttpClient {
    pub cycles: u64,
}

impl ICHttpClient {
    pub fn new(default_cycles: Option<u64>) -> Self {
        ICHttpClient {
            cycles: if let Some(v) = default_cycles { v } else { 1_000_000_000_000 },
        }
    }

    async fn request(
        &self, 
        url: String,
        req_type: HttpMethod, 
        req_headers: Vec<HttpHeader>, 
        payload: &Request,
        cycles: Option<u64>
    ) -> Result<Vec<u8>, String> {
        let request = CanisterHttpRequestArgs {
            url: url.clone(),
            http_method: req_type,
            body: Some(serde_json::to_vec(&payload).unwrap()),
            max_response_bytes: None,
            transform_method_name: None,
            headers: req_headers,
        };
        let body = candid::utils::encode_one(&request).unwrap();

        match ic_cdk::api::call::call_raw(
            Principal::management_canister(),
            "http_request",
            &body[..],
            if let Some(v) = cycles { v } else { self.cycles },
        )
        .await
        {
            Ok(result) => {
                // decode the result
                let decoded_result: CanisterHttpResponsePayload =
                    candid::utils::decode_one(&result).expect("IC http_request failed!");
                // let decoded_body = String::from_utf8(decoded_result.body)
                    // .expect("Remote service response is not UTF-8 encoded.");
                // Ok(decoded_body)
                Ok(decoded_result.body)
            }
            Err((r, m)) => {
                let message =
                    format!("The http_request resulted into error. RejectionCode: {r:?}, Error: {m}");
                ic_cdk::api::print(message.clone());
                Err(message)
            }
        }
    }

    pub async fn get(&self, url: String, payload: &Request, cycles: Option<u64>) -> Result<Vec<u8>, String> {
        let request_headers = vec![
            HttpHeader {
                name: "Content-Type".to_string(),
                value: "application/json".to_string(),
            },
        ];
        // self.handler(self.request(url, HttpMethod::GET, request_headers, payload, cycles).await)
        self.request(url, HttpMethod::GET, request_headers, payload, cycles).await
    }

    pub async fn post(&self, url: String, payload: &Request, cycles: Option<u64>) -> Result<Vec<u8>, String> {
        let request_headers = vec![
            HttpHeader {
                name: "Content-Type".to_string(),
                value: "application/json".to_string(),
            },
        ];

        // self.handler(self.request(url, HttpMethod::POST, request_headers, payload, cycles).await)
        self.request(url, HttpMethod::POST, request_headers, payload, cycles).await
    }

    // fn handler(&self, response: Result<String, String>) -> Result<String, String> {
    //     match response {
    //         Ok(res) => {
    //             if res.find("error") != None {
    //                 let e: JsonRpcError<HashMap<String, String>> = serde_json::from_str(res.as_str()).expect("response deserilize error");
    //                 return Err(e.error.message);
    //             }
    //             return Ok(res);
    //         },
    //         Err(e) => { return Err(e); },
    //     }
    // }
}

