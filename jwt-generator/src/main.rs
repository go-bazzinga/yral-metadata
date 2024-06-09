mod args;

use std::time::{Duration, SystemTime};

use args::Args;
use clap::Parser;
use ed25519_dalek::{
    pkcs8::{spki::der::pem::LineEnding, DecodePrivateKey, EncodePrivateKey, EncodePublicKey},
    SigningKey,
};
use jsonwebtoken::{encode, Algorithm, DecodingKey, EncodingKey, Header};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub company: String,
    pub exp: u64,
}

fn random_encoding_key() -> (SigningKey, EncodingKey) {
    println!("Generating new private key...");
    let key = SigningKey::generate(&mut OsRng);
    let key_pem = key.to_pkcs8_pem(LineEnding::default()).unwrap();
    let ec_key = EncodingKey::from_ed_pem(key_pem.as_bytes()).unwrap();
    println!(
        "Private Key(Save this to generate new tokens):\n{}",
        key_pem.as_str()
    );
    (key, ec_key)
}

fn main() {
    let args = Args::parse();
    let (sign_key, ec_key) = args
        .private_key
        .map(|key| {
            (
                SigningKey::from_pkcs8_pem(&key).expect("Invalid Private Key"),
                EncodingKey::from_ed_pem(key.as_bytes()).unwrap(),
            )
        })
        .unwrap_or_else(random_encoding_key);

    let exp_dur = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        + Duration::from_secs(args.expiry * 24 * 60 * 60);
    let exp = exp_dur.as_secs();
    println!("Generating token with expiry {}\n", exp);
    let claims = Claims {
        sub: "off-chain-agent".to_string(),
        company: "gobazzinga".to_string(),
        exp,
    };
    let token = encode(&Header::new(Algorithm::EdDSA), &claims, &ec_key).unwrap();
    println!("JWT Token (Use this in off-chain agent): {token}\n");

    let vk = sign_key.verifying_key();
    let vk_pem = vk.to_public_key_pem(LineEnding::default()).unwrap();
    assert!(
        DecodingKey::from_ed_pem(vk_pem.as_bytes()).is_ok(),
        "ed25519-dalek returned invalid public key"
    );
    println!("Public Key(Use this in yral-metadata-server):\n{vk_pem}");
}
