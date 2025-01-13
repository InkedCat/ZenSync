use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Generic command
    #[arg(index = 1)]
    pub command: String,
}

pub fn get_args() -> Args {
    Args::parse()
}
