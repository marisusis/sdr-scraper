use serde::{Deserialize, Serialize};

use crate::sdr::Tuning;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum SDRKind {
    KiwiSDR,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SDRStationConfig {
    pub name: String,
    pub endpoint: String,
    pub password: Option<String>,
    pub agc: bool,
    pub gain: Option<i32>,
    pub frequency: Vec<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub location: String,
    pub identity: String,
    pub stations: Vec<SDRStationConfig>,
}
