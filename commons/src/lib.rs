use lazy_static::lazy_static;
use snow::params::NoiseParams;

pub mod file_manager;
pub mod keys_manager;
pub mod packeter;

lazy_static! {
    pub static ref NOISE_PARAMS: NoiseParams = "Noise_IK_25519_ChaChaPoly_BLAKE2s".parse().unwrap();
}
