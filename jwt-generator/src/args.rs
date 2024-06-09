use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    /// Base64 Encoded private key
    /// a new key is generated if not specified
    #[clap(short, long)]
    pub private_key: Option<String>,
    /// The token expiry
    /// in days
    #[clap(short, long, default_value = "365", value_name = "DAYS")]
    pub expiry: u64,
}
