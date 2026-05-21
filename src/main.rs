mod api;
mod app;
mod cmd;
mod config;
mod domain;
mod event;
mod keys;
mod navigation;
mod panels;
mod run;
mod theme;
mod update;
mod util;
mod widgets;

use clap::Parser;

use crate::api::auth;

#[derive(Parser)]
#[command(name = "ghtui", about = "A lazygit-style TUI for GitHub")]
struct Cli {
    /// Enable debug logging
    #[arg(long)]
    debug: bool,
    /// Force re-authentication via device flow
    #[arg(long)]
    login: bool,
    /// Delete stored token and exit
    #[arg(long)]
    logout: bool,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    let cli = Cli::parse();

    if cli.debug {
        tracing_subscriber::fmt::init();
    }

    color_eyre::install()?;

    // Handle --logout before TUI init
    if cli.logout {
        match auth::delete_token() {
            Ok(()) => println!("Stored token deleted."),
            Err(e) => eprintln!("No stored token to delete: {}", e),
        }
        return Ok(());
    }

    // Resolve token: --login forces device flow, otherwise check all methods
    let token = if cli.login {
        auth::resolve_token_force_device()?
    } else {
        auth::resolve_token()?
    };

    let config = config::Config::load();
    let mut terminal = ratatui::init();
    let mut app = app::App::new(config, token);

    let result = run::run(&mut app, &mut terminal).await;

    ratatui::restore();
    result
}
