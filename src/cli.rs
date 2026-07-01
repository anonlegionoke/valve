use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Port to bind the proxy server to
    #[arg(short, long, default_value_t = 8080)]
    pub port: u16,

    /// Path to the configuration file
    #[arg(short, long, default_value = "config.toml")]
    pub config: String,
}
