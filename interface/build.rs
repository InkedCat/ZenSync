use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    // Fetch the `OUT_DIR` environment variable
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");

    let status = Command::new("glib-compile-resources")
        .args([
            "./resources.gresource.xml",
            &format!("--target={}/resources.gresource", out_dir),
            "--sourcedir=./src/ui",
        ])
        .status()
        .expect("Failed to compile resources");

    println!("ok");
    assert!(status.success(), "Failed to compile resources");
    // Define the path of the file to be generated
    let dest_path = Path::new(&out_dir).join("hello.rs");

    // Write content to the file
    fs::write(&dest_path, "pub const GREETING: &str = \"Hello, world!\";")
        .expect("Unable to write file");
}
