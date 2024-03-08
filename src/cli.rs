use clap::{Parser, Subcommand};
use std::{path::PathBuf, str::FromStr};

lazy_static::lazy_static! {

    pub static ref CONFIG_FILE: PathBuf = std::env::var("USER")
        .map(|user| PathBuf::from_str(&format!("/home/{user}/.config/mkanki.toml"))
            .expect("Invalid config file") )
        .expect("Could not find user config directory");



}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Path to the config file. If unspecified, looks in ~/.config/mkanki.toml
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Path and file name for the generated deck.
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// The deck to use
    #[arg(short, long, value_name = "FILE")]
    pub deck: Option<String>,

    pub input: String


}
