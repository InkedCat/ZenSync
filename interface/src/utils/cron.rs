use std::fs;
use std::io::{self, Write};
use std::path::Path;
use dirs::home_dir;
use std::fs::{File};
use whoami;

pub const VALID_FREQUENCIES: [&str; 12] = [
    "Aucune", "1min", "5min", "30min", "1hr", "2hr", "4hr", "6hr", "12hr", "24hr", "2days", "7days",
];

pub fn create_cron_file(frequency: &str) -> io::Result<()> {
    // Validate frequency
    if !VALID_FREQUENCIES.contains(&frequency) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Invalid frequency: {}", frequency),
        ));
    }

    // Skip creating the cron file for "Aucune"
    if frequency == "Aucune" {
        println!("No cron job will be created for frequency: {}", frequency);
        return Ok(());
    }

    println!("Frequency '{}' is valid.", frequency);

    // Path to the cron file
    let cron_file_path = "/etc/cron.d/my_rust_cron";
    let cron_job = generate_cron_job(frequency)?;

    println!("Generated cron job: {}", cron_job);

    // Check if the cron file exists
    if Path::new(cron_file_path).exists() {
        let existing_content = fs::read_to_string(cron_file_path)?;

        // If the existing cron job matches, do nothing
        if existing_content.trim() == cron_job.trim() {
            println!("Cron file already exists with the correct frequency.");
            return Ok(());
        }
    }

    // Create or overwrite the cron file
    let mut file = File::create(cron_file_path)?;
    file.write_all(cron_job.as_bytes())?;
    println!("Cron file created/updated with frequency: {}", frequency);

    Ok(())
}

pub fn generate_cron_job(frequency: &str) -> io::Result<String> {
    let schedule = match frequency {
        "1min" => "* * * * *",
        "5min" => "*/5 * * * *",
        "30min" => "*/30 * * * *",
        "1hr" => "0 * * * *",
        "2hr" => "0 */2 * * *",
        "4hr" => "0 */4 * * *",
        "6hr" => "0 */6 * * *",
        "12hr" => "0 */12 * * *",
        "24hr" => "0 0 * * *",
        "2days" => "0 0 */2 * *",
        "7days" => "0 0 * * 0",
        "Aucune" => return Ok(String::new()), // No schedule
        _ => unreachable!(),
    };

    let home = home_dir().ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "Failed to retrieve the home directory")
    })?;
    let username = whoami::username();
    let rust_path = "/home/kilian/Documents/test1/target/release/test1";
    Ok(format!("{} root {}\n", schedule,rust_path))
}

