use std::fs::create_dir_all;
use std::path::Path;
use dirs;

pub fn get_conf_path() -> String {
    let conf_path = dirs::home_dir()
        .expect("Failed to retrieve home directory")
        .join(".config/zen-sync/conf.json");

    // Ensure the parent directories exist
    if let Some(parent_dir) = conf_path.parent() {
        if !parent_dir.exists() {
            create_dir_all(parent_dir).expect("Failed to create directories");
        }
    }

    // Return the path as a string
    match conf_path.to_str() {
        Some(conf_path_str) => conf_path_str.to_string(),
        None => String::from("Invalid path"), // Handle the case when the path can't be converted to a string
    }
}

pub fn get_conf_hash_path() -> String {
    let conf_path = dirs::home_dir()
        .expect("Failed to retrieve home directory")
        .join(".config/zen-sync/config.hash");

    // Ensure the parent directories exist
    if let Some(parent_dir) = conf_path.parent() {
        if !parent_dir.exists() {
            create_dir_all(parent_dir).expect("Failed to create directories");
        }
    }

    // Return the path as a string
    match conf_path.to_str() {
        Some(conf_path_str) => conf_path_str.to_string(),
        None => String::from("Invalid path"), // Handle the case when the path can't be converted to a string
    }
}

pub fn get_log_path() -> String {
    let conf_path = dirs::home_dir()
        .expect("Failed to retrieve home directory")
        .join(".config/zen-sync/log.txt");

    // Ensure the parent directories exist
    if let Some(parent_dir) = conf_path.parent() {
        if !parent_dir.exists() {
            create_dir_all(parent_dir).expect("Failed to create directories");
        }
    }

    // Check if the file exists and return accordingly
    if conf_path.exists() {
        conf_path
            .to_str()
            .expect("Path contains invalid UTF-8")
            .to_string()
    } else {
        String::from("")
    }
}

