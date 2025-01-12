use std::fs;
use std::error::Error;
use std::path::Path;
use std::fs::File;

pub fn write_data(filename: &str, data: &str) -> Result<(), Box<dyn Error>> {
    fs::write(filename, data)?;
    Ok(())
}

pub fn read_data(file_path: &str) -> Result<String, Box<dyn Error>> {
    let data = fs::read_to_string(file_path)?;
    Ok(data)
}

pub fn create_file(file_path: &str) -> Result<(), Box<dyn Error>> {
    let file = File::create(file_path)?;
    Ok(())
}

pub fn check_file_exists(file_path: &str) -> bool {
   Path::new(file_path).exists() 
}
