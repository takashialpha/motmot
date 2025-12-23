use app_base::app::{AppConfigLocation, run};
use app_base::cli::{CliArgs, InitArgs, RuntimeArgs};
use clap::{Parser, Subcommand};

mod app;
mod config;
mod logging;
mod server;

const APP_NAME: &str = "motmot";
const TOML_CONFIG_DIR: &str = "/etc/motmot";

/// MotMot Application
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
    Init {
        #[command(flatten)]
        init: InitArgs,
    },
}

fn main() {
    let cli = Cli::parse();

    // Construct CliArgs from parsed CLI
    let cli_args = match cli.command {
        Command::Init { init } => CliArgs::new(init, RuntimeArgs::default()),
    };

    let cfg = AppConfigLocation::new(APP_NAME).with_dir(TOML_CONFIG_DIR);

    if let Err(e) = run(app::MotMot, cfg, cli_args) {
        eprint!("{}", e);
        std::process::exit(1);
    }
}
