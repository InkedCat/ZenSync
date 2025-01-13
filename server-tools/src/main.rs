mod cli;

use std::io::Read;

use anyhow::bail;
use base64::{prelude::BASE64_STANDARD, Engine};
use commons::keys_manager;

const ZSYNC_KEY_LEN_BASE64: usize = 46;

fn version_command() {
    println!("zen-sync-server v0.1.0");
}

fn help_command() {
    println!("Usage: zen-sync-server <command>\n");
    println!("Available commands:");
    println!("  version - Print the version of the server");
    println!("  gen-private - Generate a new private key and print it");
    println!("  gen-public - Generate a new public key from a private key and print it");
    println!("  help - Print this help message");
}

fn gen_private_command() -> anyhow::Result<()> {
    println!("{}", keys_manager::new_private_key()?);
    Ok(())
}

fn gen_public_command() -> anyhow::Result<()> {
    let mut stdin = std::io::stdin().take(ZSYNC_KEY_LEN_BASE64 as u64);
    let mut buffer = Vec::new();
    stdin.read_to_end(&mut buffer).map_err(|_| {
        return anyhow::anyhow!("Key is not the correct length or is not base64 encoded");
    })?;

    if buffer.len() > ZSYNC_KEY_LEN_BASE64 - 1 {
        bail!("Invalid private key length");
    }

    buffer.pop();

    let decoded_key = BASE64_STANDARD.decode(&buffer).map_err(|_| {
        return anyhow::anyhow!("Key is not the correct length or is not base64 encoded");
    })?;

    if decoded_key.len() != 32 {
        bail!("Invalid private key length");
    }

    let parsed_key: [u8; 32] = decoded_key
        .try_into()
        .map_err(|_| anyhow::anyhow!("Failed to parse private key"))?;

    println!("{}", commons::keys_manager::new_public_key(parsed_key)?);

    Ok(())
}

fn handle_command(command: String) -> anyhow::Result<()> {
    match command.as_str() {
        "version" => version_command(),
        "help" => help_command(),
        "gen-private" => gen_private_command()?,
        "gen-public" => gen_public_command()?,
        _ => println!("Unknown command: {}", command),
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = cli::get_args();

    if args.command != "" {
        return handle_command(args.command);
    }

    handle_command(String::from("help"))
}
