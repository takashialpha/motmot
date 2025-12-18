use app_base::app::{AppConfigLocation, run};

mod app;
mod config;
mod logging;
mod server;

fn main() {
    let cfg = AppConfigLocation::new("motmot", "motmot-config.toml");

    if let Err(e) = run(app::MotMot, cfg) {
        eprint!("{}", e);
        std::process::exit(1);
    }
}
