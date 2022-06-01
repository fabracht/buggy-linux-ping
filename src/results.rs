use std::{collections::HashMap, fmt::Display};

#[derive(Debug, Clone, Default)]
pub struct PingResults {
    pub result: u8,
    pub host_address: String,
    pub host_tested: String,
    pub host_name: String,
    pub dscp_changed: bool,
    pub init_dscp_return: u8,
    pub original_dscp: u8,
    pub rtt_statistics: Statistics,
}

#[derive(Default, Clone, Debug)]
pub struct Statistics {
    mean: f64,
    median: f64,
    total_sample: usize,
    std_deviation: f64,
    variance: f64,
    pub replies: HashMap<usize, (u128, u8)>,
}

impl Statistics {
    pub fn calculate(&mut self) {
        self.set_total_length();
        self.calculate_average();
        self.calculate_variance();
        self.calculate_std_dev();
        self.calculate_median();
    }

    fn set_total_length(&mut self) {
        self.total_sample = self.replies.len();
    }

    fn calculate_variance(&mut self) {
        self.variance = self
            .replies
            .iter()
            .map(|(_seq_number, (duration, _icmp_type))| (*duration as f64 - self.mean).powi(2))
            .sum::<f64>()
            / (self.total_sample) as f64;
    }

    fn calculate_std_dev(&mut self) {
        self.std_deviation = self.variance.powf(0.5);
    }

    fn calculate_average(&mut self) {
        self.mean = self
            .replies
            .iter()
            .map(|(_seq_number, (duration, _icmp_type))| *duration)
            .sum::<u128>() as f64
            / self.total_sample as f64;
    }

    fn calculate_median(&mut self) {
        self.median = 0.0;
    }
}

pub trait PingStatistics {
    fn number_of_no_responses(&self) -> u64;
    fn percentage_of_no_responses(&self) -> f64;
    fn number_of_bad_packets(&self) -> u64;
    fn min_delay(&self) -> u128;
    fn max_delay(&self) -> u128;
    fn avg_delay(&self) -> f64;
    fn median_delay(&self) -> f64;
    fn percent_of_lost_packets(&self) -> f64;
    fn average_jitter(&self) -> f64;
}

impl PingStatistics for Statistics {
    fn number_of_no_responses(&self) -> u64 {
        self.replies
            .iter()
            .filter(|(_seq_number, (_duration, icmp_type))| *icmp_type == 10)
            .count() as u64
    }

    fn percentage_of_no_responses(&self) -> f64 {
        self.number_of_no_responses() as f64 / self.replies.capacity() as f64
    }

    fn number_of_bad_packets(&self) -> u64 {
        0
    }

    fn min_delay(&self) -> u128 {
        let min = self
            .replies
            .iter()
            .map(|(_seq_number, (duration, _icmp_type))| duration)
            .min()
            .unwrap();
        min / 2 // Approx delay ~= RTT/2
    }

    fn max_delay(&self) -> u128 {
        let max = self
            .replies
            .iter()
            .map(|(_seq_number, (duration, _icmp_type))| duration)
            .max()
            .unwrap();
        max / 2
    }

    fn avg_delay(&self) -> f64 {
        self.mean
    }

    fn median_delay(&self) -> f64 {
        self.median
    }

    fn percent_of_lost_packets(&self) -> f64 {
        (self.replies.capacity() - self.replies.len()) as f64 / self.replies.len() as f64
    }

    fn average_jitter(&self) -> f64 {
        self.variance
    }
}

impl Display for Statistics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "n_of_replies: {}\navg_delay: {:.3} ms\navg_jitter: {:.3} ms\nstd_dev: {:.3} ms",
            self.total_sample,
            self.mean / 1000.,
            self.variance / 1000000.,
            self.std_deviation / 1000.
        )
    }
}
