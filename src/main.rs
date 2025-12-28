use app_base::{AppConfigLocation, cli::CliArgs, run};
use clap::{Parser, Subcommand};

mod app;
mod config;
mod features;
mod helpers;
mod http;
mod logging;
mod net;

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
        init: CliArgs,
    },
}

fn main() {
    let cli = Cli::parse();

    let cli_args = match cli.command {
        Command::Init { init } => CliArgs {
            config: init.config,
        },
    };

    let cfg = AppConfigLocation::new(APP_NAME).with_dir(TOML_CONFIG_DIR);

    if let Err(e) = run(app::MotMot, Some(cfg), cli_args) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
