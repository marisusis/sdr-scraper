use serde::{Deserialize, Serialize};

use crate::sdr::Tuning;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum SDRKind {
    KiwiSDR,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SDRStationConfig {
    pub name: String,
    pub kind: SDRKind,
    pub endpoint: String,
    pub password: Option<String>,
    pub agc: bool,
    pub tuning: Tuning,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub location: String,
    pub identity: String,
    pub stations: Vec<SDRStationConfig>,
}
