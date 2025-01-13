use base64::{prelude::BASE64_STANDARD, Engine};
use x25519_dalek::{PublicKey, StaticSecret};

use crate::NOISE_PARAMS;

pub fn new_private_key() -> anyhow::Result<String> {
    let generator = snow::Builder::new(NOISE_PARAMS.clone());
    let pair = generator.generate_keypair()?;

    Ok(BASE64_STANDARD.encode(pair.private))
}

pub fn new_public_key(private_key: [u8; 32]) -> anyhow::Result<String> {
    let private_key = StaticSecret::from(private_key);
    let public_key = PublicKey::from(&private_key);

    Ok(BASE64_STANDARD.encode(public_key.as_bytes()))
}
