mod config;
mod setup;
mod uri;
mod auth;
mod launcher;
mod updater;
mod doctor;
mod games;
mod crypto;

use clap::{Parser, Subcommand};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TempestError {
    #[error("Config error: {0}")]
    ConfigError(String),
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("Wine error: {0}")]
    WineError(String),
    #[error("Auth error: {0}")]
    AuthError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

#[derive(Parser)]
#[command(name = "tempest", about = "Linux launcher for Vortex")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Setup,
    Play { game_id: u32 },
    Login,
    List,
    Update,
    UriHandler { uri: String },
    Doctor,
    Uninstall,
}

#[tokio::main]
async fn main() {
    let filter = std::env::var("TEMPEST_LOG").unwrap_or_else(|_| "warn".to_string());
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Setup => setup::run().await,
        Commands::Play { game_id } => launcher::play(game_id).await,
        Commands::Login => auth::login().await,
        Commands::List => games::list().await,
        Commands::Update => updater::update().await,
        Commands::UriHandler { uri } => uri::handle(&uri).await,
        Commands::Doctor => doctor::run(),
        Commands::Uninstall => setup::uninstall(),
    }
}
