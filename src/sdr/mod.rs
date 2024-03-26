mod client;
pub mod kiwi;

use std::fmt::{self, Display, Formatter};

pub use client::SDRClient;
pub use kiwi::KiwiSDR;

#[derive(Debug)]
pub enum SDRError {
    NotConnected,
    NotReady,
    Timeout,
}

pub struct AMTuning {
    pub bandwidth: i32,
    pub freq: f64,
}

pub struct GeneralTuning {
    pub low_cut: i32,
    pub high_cut: i32,
    pub freq: f64,
}

pub enum Station {
    AM(AMTuning),
    FM(GeneralTuning),
    LSB(GeneralTuning),
    USB(GeneralTuning),
}

impl Display for Station {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Station::AM(config) => {
                write!(
                    f,
                    "AM: {} Hz, {} Hz bandwidth",
                    config.freq, config.bandwidth
                )
            }
            Station::FM(config) => {
                write!(
                    f,
                    "FM: {} Hz, -{}->{} Hz",
                    config.freq, config.low_cut, config.high_cut
                )
            }
            Station::LSB(config) => {
                write!(
                    f,
                    "LSB: {} Hz, -{}->{} Hz",
                    config.freq, config.low_cut, config.high_cut
                )
            }
            Station::USB(config) => {
                write!(
                    f,
                    "USB: {} Hz, -{}->{} Hz",
                    config.freq, config.low_cut, config.high_cut
                )
            }
        }
    }
}
