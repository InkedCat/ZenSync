use sha2::{Sha256, Digest};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use crate::utils::conf_file;

pub struct ConfigIntegrity {
    file_path: String,
    hash_path: String,
}

impl ConfigIntegrity {
    /// Create a new instance of ConfigIntegrity
    pub fn new(file_path: &str, hash_path: &str) -> Self {
        Self {
            file_path: file_path.to_string(),
            hash_path: hash_path.to_string(),
        }
    }

    /// Compute the hash of the JSON file
    pub fn compute_hash(&self) -> Result<String, io::Error> {
        let content = fs::read_to_string(&self.file_path)?;
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        Ok(hash)
    }

    /// Save the hash to the hash file
    pub fn save_hash(&self) -> Result<(), io::Error> {
        let hash = self.compute_hash()?;
        let mut file = fs::File::create(&self.hash_path)?;
        file.write_all(hash.as_bytes())?;
        Ok(())
    }

    /// Verify the hash against the stored hash file
    pub fn verify_hash(&self) -> Result<bool, io::Error> {
        let current_hash = self.compute_hash()?;
        let stored_hash = fs::read_to_string(&self.hash_path)?.trim().to_string();
        Ok(current_hash == stored_hash)
    }
}

pub fn update_hash(){
    let conf_path = conf_file::get_conf_path();
    let hash_conf_path = conf_file::get_conf_hash_path();
    let integrity = ConfigIntegrity::new(&conf_path, &hash_conf_path);
    integrity.save_hash();
}

pub fn check_integrity() -> bool {
    // Example usage
    let conf_path = conf_file::get_conf_path();
    let hash_conf_path = conf_file::get_conf_hash_path();
    let integrity = ConfigIntegrity::new(&conf_path, &hash_conf_path);

    // Verify the hash
    match integrity.verify_hash() {
        Ok(is_valid) => {
            if is_valid {
                return true;
            } else {
                return false;
            }
        }
        Err(e) => true,
    }
}

