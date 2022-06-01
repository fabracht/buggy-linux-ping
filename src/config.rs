use std::time::Duration;

#[allow(clippy::too_many_arguments)]
#[derive(Debug, Clone)]
pub struct Configuration {
    pub duration: Duration,
    pub host: String,
    pub number_of_pings: u8,
    pub payload_size: usize,
    pub ping_interval: u64,
    pub ttl: u8,
    pub dscp: u8,
    pub source_ip: String,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            duration: Duration::from_secs(3),
            host: "172.217.14.238".to_string(),
            number_of_pings: 20,
            payload_size: 60,    // bytes
            ping_interval: 2000, //microseconds
            ttl: 15,
            dscp: 0,
            source_ip: "192.168.1.71".to_string(),
        }
    }
}
