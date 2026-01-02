use crate::APP_NAME;
use app_base::app::ConfigPath;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser, Clone)]
#[command(
    name = APP_NAME,
    version,
    about = "A simple but modern and blazing-fast quic http3 ipv6 only made in rust"
)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand, Clone)]
enum Command {
    #[command(about = "Init motmot according to the config file")]
    Init {
        #[command(flatten)]
        init: InitArgs,
    },
}

#[derive(Debug, Args, Clone)]
struct InitArgs {
    #[arg(long, value_name = "file")]
    config: Option<PathBuf>,
}

impl ConfigPath for Cli {
    fn config_path(&self) -> Option<PathBuf> {
        match &self.command {
            Command::Init { init } => init.config.clone(),
        }
    }
}
