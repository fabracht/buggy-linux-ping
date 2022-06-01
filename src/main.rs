use log::{info, LevelFilter};

use crate::{config::Configuration, ping::run_ping};

use pnet::datalink::NetworkInterface;

mod config;
mod ping;
mod results;
mod utils;

fn main() {
    let mut env_logger = env_logger::builder();
    env_logger.filter_level(LevelFilter::Debug).init();

    let interfaces = pnet::datalink::interfaces();
    let default_interface_option: Option<NetworkInterface> = interfaces
        .into_iter()
        .find(|e| e.is_up() && !e.is_loopback() && !e.ips.is_empty());

    let default_interface = default_interface_option.unwrap();
    let ips = default_interface.ips.clone();
    let ipv4 = ips.into_iter().filter(|val| val.is_ipv4()).next().unwrap();
    // info!("{:?}", ipv4.ip());
    let mut config = Configuration::default();
    config.source_ip = ipv4.ip().to_string();
    let mut test_result = run_ping(&config);
    test_result.rtt_statistics.calculate();
    info!("Stats:\n{}", test_result.rtt_statistics);
}
