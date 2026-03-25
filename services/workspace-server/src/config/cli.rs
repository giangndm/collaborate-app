use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Path to configuration file
    #[arg(short, long, env = "CONSOLE_CONFIG")]
    pub config: Option<PathBuf>,

    /// Database path
    #[arg(short, long, env = "CONSOLE_DATABASE_URL")]
    pub database_url: Option<String>,

    /// Port to listen on
    #[arg(short, long, env = "CONSOLE_PORT")]
    pub port: Option<u16>,
}
