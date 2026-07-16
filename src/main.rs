use clap::Parser;
use config::{Config, PromptMode};
use server::serve;
use std::path::PathBuf;
use watcher::watch;

mod config;
mod server;
mod watcher;

/// OpenAI API-compatible LLM gateway
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the config file
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

}

// Entry point
fn main() {
    // Parse args and define config path
    let args = Args::parse();
    let config_path = args.config.unwrap_or_else(|| {
        let mut config_path = dirs::config_dir().unwrap();
        config_path.push("ollmo");
        config_path
    });

    // Build tokio runtime
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
        .block_on(async {
            // Enable logger
            let level = match args.verbose {
                0 => tracing::Level::INFO,
                1 => tracing::Level::DEBUG,
                _ => tracing::Level::TRACE,
            };
            tracing_subscriber::fmt()
                .with_max_level(level)
                .init();

            // Load config
            let path_copy = config_path.clone();
            let config = Config::load(&config_path).await.unwrap();
            let env_file = config.get_env_file();
            let (tx, rx) = tokio::sync::watch::channel(config);

            // Run watcher
            tokio::spawn(async {
                let _ = watch(config_path, env_file, tx).await;
            });

            // Start server
            serve(rx, path_copy).await
        })
}
