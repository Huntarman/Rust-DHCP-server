use std::fs;
use std::io::{self, Write};
use sha2::{Sha256, Digest};

pub fn calculate_config_hash(file_path: &str) -> Result<String, io::Error> {
    let config_data = fs::read(file_path)?;
    let mut hasher = Sha256::new();
    hasher.update(config_data);
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn store_hash(hash: &str) -> Result<(), io::Error> {
    let mut file = fs::File::create(".last_config_hash")?;
    file.write_all(hash.as_bytes())?;
    Ok(())
}

pub fn load_previous_hash() -> Result<Option<String>, io::Error> {
    match fs::read_to_string(".last_config_hash") {
        Ok(hash) => Ok(Some(hash)),
        Err(_) => Ok(None),
    }
}
//CHECK IF CONFIG FILE HAS CHANGED
pub fn check_config_changed(file_path: &str) -> Result<bool, io::Error> {
    let new_hash = calculate_config_hash(file_path)?;
    if let Some(previous_hash) = load_previous_hash()? {
        Ok(new_hash != previous_hash)
    } else {
        Ok(true)
    }
}
