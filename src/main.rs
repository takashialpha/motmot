use app_base::app::{AppConfigLocation, run};
mod app;
mod config;
mod logging;
mod server;

const APP_NAME: &str = "motmot";
const TOML_CONFIG_DIR: &str = "/etc/motmot";

fn main() {
    let cfg = AppConfigLocation::new(APP_NAME).with_dir(TOML_CONFIG_DIR);

    if let Err(e) = run(app::MotMot, cfg) {
        eprint!("{}", e);
        std::process::exit(1);
    }
}
