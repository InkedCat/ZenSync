use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Bind address
    #[arg(short, long, default_value = "localhost")]
    pub addr: String,

    /// Config file path
    #[arg(short, long, default_value = "config.toml")]
    pub config_path: String,
}

pub fn get_args() -> Args {
    Args::parse()
}
