mod api;
mod app;
mod cmd;
mod config;
mod constants;
mod domain;
mod event;
mod keys;
mod panels;
mod run;
mod theme;
mod update;
mod util;
mod view;
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

fn main() -> color_eyre::Result<()> {
    let cli = Cli::parse();

    if cli.debug {
        tracing_subscriber::fmt::init();
    }

    color_eyre::install()?;

    // Handle --logout before TUI init
    if cli.logout {
        match auth::delete_token() {
            Ok(()) => tracing::info!("Stored token deleted."),
            Err(e) => tracing::warn!(error = %e, "no stored token to delete"),
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

    // Use larger stack to prevent overflow from tokio::select! in debug builds
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(16 * 1024 * 1024)
        .build()?;

    rt.block_on(async {
        let mut terminal = ratatui::init();
        let mut app = app::App::new(config, token)?;

        let result = run::run(&mut app, &mut terminal).await;

        ratatui::restore();
        result
    })
}
