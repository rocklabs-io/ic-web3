//! IC's threshold ECDSA related functions

use ic_cdk::export::{
    candid::CandidType,
    serde::{Deserialize, Serialize},
    Principal,
};
use std::str::FromStr;
use crate::types::Address;
use crate::signing;
use libsecp256k1::{PublicKey, PublicKeyFormat, Message, Signature, RecoveryId, recover};

const ECDSA_SIGN_CYCLES : u64 = 10_000_000_000;
// pub type Address = [u8; 20];

// #[derive(CandidType, Serialize, Debug, Clone)]
// pub enum EcdsaCurve {
//     #[serde(rename = "secp256k1")]
//     secp256k1,
// }

use ic_cdk::api::management_canister::ecdsa::*;

#[derive(CandidType, Serialize, Debug, Clone)]
pub struct KeyInfo {
    pub derivation_path: Vec<Vec<u8>>,
    pub key_name: String,
    pub ecdsa_sign_cycles: Option<u64>,
}

/// get public key from ic, 
/// derivation_path: 4-byte big-endian encoding of an unsigned integer less than 2^31
pub async fn get_public_key(
    canister_id: Option<Principal>, 
    derivation_path: Vec<Vec<u8>>,
    key_name: String
) -> Result<Vec<u8>, String> {
    let key_id = EcdsaKeyId {
        curve: EcdsaCurve::Secp256k1,
        name: key_name,
    };
    let ic_canister_id = "aaaaa-aa";
    let ic = Principal::from_str(&ic_canister_id).unwrap();


    let request = EcdsaPublicKeyArgument {
        canister_id: canister_id,
        derivation_path: derivation_path,
        key_id: key_id.clone(),
    };
    let (res,): (EcdsaPublicKeyResponse,) = ic_cdk::call(ic, "ecdsa_public_key", (request,))
        .await
        .map_err(|e| format!("Failed to call ecdsa_public_key {}", e.1))?;

    Ok(res.public_key)
}

/// convert compressed public key to ethereum address
pub fn pubkey_to_address(pubkey: &[u8]) -> Result<Address, String> {
    let uncompressed_pubkey = match PublicKey::parse_slice(pubkey, Some(PublicKeyFormat::Compressed)) {
        Ok(key) => { key.serialize() },
        Err(_) => { return Err("uncompress public key failed: ".to_string()); },
    };
    let hash = signing::keccak256(&uncompressed_pubkey[1..65]);
	let mut result = [0u8; 20];
	result.copy_from_slice(&hash[12..]);
	Ok(Address::from(result))
}

/// get canister's eth address
pub async fn get_eth_addr(
    canister_id: Option<Principal>, 
    derivation_path: Option<Vec<Vec<u8>>>,
    name: String
) -> Result<Address, String> {
    let path = if let Some(v) = derivation_path { v } else { vec![ic_cdk::id().as_slice().to_vec()] };
    match get_public_key(canister_id, path, name).await {
        Ok(pubkey) => { return pubkey_to_address(&pubkey); },
        Err(e) => { return Err(e); },
    };
}

/// use ic's threshold ecdsa to sign a message
pub async fn ic_raw_sign(
    message: Vec<u8>,
    key_info: KeyInfo,
) -> Result<Vec<u8>, String> {
    assert!(message.len() == 32);

    let key_id = EcdsaKeyId {
        curve: EcdsaCurve::Secp256k1,
        name: key_info.key_name,
    };
    let ic = Principal::management_canister();

    let request = SignWithEcdsaArgument {
        message_hash: message.clone(),
        derivation_path: key_info.derivation_path,
        key_id,
    };

    let ecdsa_sign_cycles = key_info.ecdsa_sign_cycles.unwrap_or(ECDSA_SIGN_CYCLES);

    let (res,): (SignWithEcdsaResponse,) =
        ic_cdk::api::call::call_with_payment(ic, "sign_with_ecdsa", (request,), ecdsa_sign_cycles)
            .await
            .map_err(|e| format!("Failed to call sign_with_ecdsa {}", e.1))?;

    Ok(res.signature)
}


// recover address from signature
// rec_id < 4
pub fn recover_address(msg: Vec<u8>, sig: Vec<u8>, rec_id: u8) -> String {
    let message = Message::parse_slice(&msg).unwrap();
    let signature = Signature::parse_overflowing_slice(&sig).unwrap();
    let recovery_id = RecoveryId::parse(rec_id).unwrap();

    match recover(&message, &signature, &recovery_id) {
        Ok(pubkey) => {
            let uncompressed_pubkey = pubkey.serialize();
            // let hash = keccak256_hash(&uncompressed_pubkey[1..65]);
            let hash = signing::keccak256(&uncompressed_pubkey[1..65]);
            let mut result = [0u8; 20];
            result.copy_from_slice(&hash[12..]);
            hex::encode(result)
        },
        Err(_) => { "".into() }
    }
}

/*
pub fn verify(pubkey: Vec<u8>, message: Vec<u8>, signature: Vec<u8>) -> Bool {
    unimplemented!()
}
*/