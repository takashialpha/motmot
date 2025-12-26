use app_base::cli::{CliArgs, InitArgs, RuntimeArgs};
use app_base::{AppConfigLocation, run};
use clap::{Parser, Subcommand};

mod action;
mod app;
mod config;
mod connection;
mod health;
mod logging;
mod proxy;
mod server;
mod tools;
mod webtransport;

const APP_NAME: &str = "motmot";
const TOML_CONFIG_DIR: &str = "/etc/motmot";

#[derive(Debug, Parser)]
#[command(
    name = APP_NAME,
    version,
    about = "A simple but modern and blazing-fast quic http3 ipv6 only made in rust"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(about = "Init motmot according to the config file")]
    Init {
        #[command(flatten)]
        init: InitArgs,
    },
}

fn main() {
    let cli = Cli::parse();

    let cli_args = match cli.command {
        Command::Init { init } => CliArgs::new(init, RuntimeArgs::default()),
    };

    let cfg = AppConfigLocation::new(APP_NAME).with_dir(TOML_CONFIG_DIR);

    if let Err(e) = run(app::MotMot, cfg, cli_args) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
