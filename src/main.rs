use app_base::{AppConfigLocation, run};
use clap::Parser;
use motmot::{APP_NAME, TOML_CONFIG_DIR, app, cli::Cli};

fn main() {
    let cli = Cli::parse();

    let cfg = AppConfigLocation::new(APP_NAME).with_dir(TOML_CONFIG_DIR);

    if let Err(e) = run(app::MotMot, Some(cfg), cli) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
