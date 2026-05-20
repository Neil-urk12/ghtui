mod api;
mod app;
mod cmd;
mod config;
mod event;
mod keys;
mod panels;
mod run;
mod theme;
mod util;
mod widgets;

use std::sync::Arc;

use clap::Parser;

use crate::api::auth::resolve_token;
use crate::api::client::GitHubClient;

#[derive(Parser)]
#[command(name = "ghtui", about = "A lazygit-style TUI for GitHub")]
struct Cli {
    /// Enable debug logging
    #[arg(long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    let cli = Cli::parse();

    if cli.debug {
        tracing_subscriber::fmt::init();
    }

    color_eyre::install()?;

    let token = resolve_token()?;
    let client = Arc::new(GitHubClient::new(token));

    let config = config::Config::load();
    let mut terminal = ratatui::init();
    let mut app = app::App::new(config, client);

    let result = run::run(&mut app, &mut terminal).await;

    ratatui::restore();
    result
}
