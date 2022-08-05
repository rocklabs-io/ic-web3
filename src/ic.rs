use ic_cdk::export::{
    candid::CandidType,
    serde::{Deserialize, Serialize},
    Principal,
};
use std::str::FromStr;
use crate::types::Address;
use libsecp256k1::{PublicKey, PublicKeyFormat};
use ethereum_tx_sign::keccak256_hash;

const ECDSA_SIGN_CYCLES : u64 = 10_000_000_000;
// pub type Address = [u8; 20];

#[derive(CandidType, Serialize, Debug, Clone)]
pub enum EcdsaCurve {
    #[serde(rename = "secp256k1")]
    secp256k1,
}

#[derive(CandidType, Serialize, Debug, Clone)]
struct EcdsaKeyId {
    pub curve: EcdsaCurve,
    pub name: String,
}

#[derive(CandidType, Serialize, Debug, Clone)]
pub struct KeyInfo {
    pub derivation_path: Vec<Vec<u8>>,
    pub key_name: String,
}

#[derive(CandidType, Serialize, Debug)]
struct ECDSAPublicKey {
    pub canister_id: Option<Principal>,
    pub derivation_path: Vec<Vec<u8>>,
    pub key_id: EcdsaKeyId,
}

#[derive(CandidType, Deserialize, Debug)]
struct ECDSAPublicKeyReply {
    pub public_key: Vec<u8>,
    pub chain_code: Vec<u8>,
}

#[derive(CandidType, Serialize, Debug)]
struct ECDSASignPayload {
    pub message_hash: Vec<u8>,
    pub derivation_path: Vec<Vec<u8>>,
    pub key_id: EcdsaKeyId,
}

#[derive(CandidType, Deserialize, Debug)]
struct SignWithECDSAReply {
    pub signature: Vec<u8>,
}

/// get public key
/// derivation_path: 4-byte big-endian encoding of an unsigned integer less than 2^31
pub async fn get_public_key(
    canister_id: Option<Principal>, 
    derivation_path: Vec<Vec<u8>>,
    key_name: String
) -> Result<Vec<u8>, String> {
    let key_id = EcdsaKeyId {
        curve: EcdsaCurve::secp256k1,
        name: key_name,
    };
    let ic_canister_id = "aaaaa-aa";
    let ic = Principal::from_str(&ic_canister_id).unwrap();

    let request = ECDSAPublicKey {
        canister_id: canister_id,
        derivation_path: derivation_path,
        key_id: key_id.clone(),
    };
    let (res,): (ECDSAPublicKeyReply,) = ic_cdk::call(ic, "ecdsa_public_key", (request,))
        .await
        .map_err(|e| format!("Failed to call ecdsa_public_key {}", e.1))?;

    Ok(res.public_key)
}

/// public key to address
pub fn pubkey_to_address(pubkey: &[u8]) -> Result<Address, String> {
    let uncompressed_pubkey = match PublicKey::parse_slice(pubkey, Some(PublicKeyFormat::Compressed)) {
        Ok(key) => { key.serialize() },
        Err(_) => { return Err("uncompress public key failed: ".to_string()); },
    };
    let hash = keccak256_hash(&uncompressed_pubkey[1..65]);
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
    let path = if let Some(v) = derivation_path { v } else { vec![ic_cdk::caller().as_slice().to_vec()] };
    match get_public_key(canister_id, path, name).await {
        Ok(pubkey) => { return pubkey_to_address(&pubkey); },
        Err(e) => { return Err(e); },
    };
}

// pub async fn sign_eth_tx(tx: &dyn ethereum_tx_sign::Transaction, derivation_path: Vec<Vec<u8>>, key_name: String) -> Result<String, String> {
//     let chain_id = tx.chain();
//     let tx_hash = tx.hash().to_vec();
//     let tx_bytes = match sign_eth_msg(
//         tx_hash, 
//         derivation_path, 
//         key_name, 
//         chain_id
//     ).await {
//         Ok(sig) => {
//             tx.sign(&sig)
//         },
//         Err(e) => { return Err(e); },
//     };
//     Ok("0x".to_string() + &hex::encode(tx_bytes))
// }

// pub async fn ic_sign(
//     tx: &dyn Transaction, 
//     derivation_path: Vec<Vec<u8>>, 
//     key_name: String, 
//     chain_id: u64
// ) -> SignedTransaction {
//     let adjust_v_value = matches!(tx.transaction_type.map(|t| t.as_u64()), Some(LEGACY_TX_ID) | None);

//     let encoded = tx.encode(chain_id, None);

//     let hash = signing::keccak256(encoded.as_ref());

//     let res = match raw_sign(hash, derivation_path, key_name).await {
//         Ok(v) => { v },
//         Err(e) => { return Err(e); },
//     };

//     let signed = self.encode(chain_id, Some(&signature));
//     let transaction_hash = signing::keccak256(signed.as_ref()).into();

//     SignedTransaction {
//         message_hash: hash.into(),
//         v: 2 * chain_id + 35,
//         r: res[0..32].to_vec(),
//         s: res[32..64].to_vec(),
//         raw_transaction: signed.into(),
//         transaction_hash,
//     }
// }

// sign eth msg
// pub async fn sign_eth_msg(
//     message: Vec<u8>, 
//     derivation_path: Vec<Vec<u8>>,
//     key_name: String,
//     chain_id: u64
// ) -> Result<EcdsaSig, String> {
//     let res = match raw_sign(message, derivation_path, key_name).await {
//         Ok(v) => { v },
//         Err(e) => { return Err(e); },
//     };
//     Ok(EcdsaSig {
//         v: 2 * chain_id + 35,
//         r: res[0..32].to_vec(),
//         s: res[32..64].to_vec()
//     })
// }

/// use ic's threshold ecdsa to sign a message
pub async fn ic_raw_sign(
    message: Vec<u8>, 
    derivation_path: Vec<Vec<u8>>, 
    key_name: String
) -> Result<Vec<u8>, String> {
    assert!(message.len() == 32);

    let key_id = EcdsaKeyId {
        curve: EcdsaCurve::secp256k1,
        name: key_name,
    };
    let ic_canister_id = "aaaaa-aa";
    let ic = Principal::from_str(&ic_canister_id).unwrap();

    let request = ECDSASignPayload {
        message_hash: message.clone(),
        derivation_path: derivation_path,
        key_id,
    };
    let (res,): (SignWithECDSAReply,) =
        ic_cdk::api::call::call_with_payment(ic, "sign_with_ecdsa", (request,), ECDSA_SIGN_CYCLES)
            .await
            .map_err(|e| format!("Failed to call sign_with_ecdsa {}", e.1))?;

    Ok(res.signature)
}

/*
// recover public key from signature
pub fn recover(signature: Vec<u8>) -> Vec<u8> {

}

pub fn verify(pubkey: Vec<u8>, message: Vec<u8>, signature: Vec<u8>) -> Bool {
    unimplemented!()
}
*/