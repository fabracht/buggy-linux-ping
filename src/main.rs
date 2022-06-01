use log::{info, LevelFilter};

use crate::{config::Configuration, ping::run_ping};

mod config;
mod ping;
mod results;
mod utils;

fn main() {
    let mut env_logger = env_logger::builder();
    env_logger.filter_level(LevelFilter::Debug).init();
    let config = Configuration::default();
    let mut test_result = run_ping(&config);
    test_result.rtt_statistics.calculate();
    info!("Stats:\n{}", test_result.rtt_statistics);
}
