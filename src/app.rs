use crate::config::AppConfig;
use app_base::{App, app::Context, error::AppError};

pub struct MotMot;

impl App for MotMot {
    type Config = AppConfig;

    fn run(&self, ctx: Context<Self::Config>) -> Result<(), AppError> {
        println!("config: {:?}", ctx.config);
        println!("runtime flags: {:?}", ctx.runtime);
        Ok(())
    }
}
