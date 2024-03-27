pub mod kiwi;
mod scraper;

use std::fmt::{self, Display, Formatter};

pub use scraper::{SDRScraper, ScraperStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum SDRError {
    NotConnected,
    NotReady,
    Timeout,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "mode")]
pub enum Tuning {
    AM {
        bandwidth: i32,
        frequency: f64,
    },
    FM {
        low_cut: i32,
        high_cut: i32,
        frequency: f64,
    },
    LSB {
        low_cut: i32,
        high_cut: i32,
        frequency: f64,
    },
    USB {
        low_cut: i32,
        high_cut: i32,
        frequency: f64,
    },
}

impl Display for Tuning {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Tuning::AM {
                bandwidth,
                frequency: freq,
            } => {
                write!(f, "AM: {} Hz, {} Hz bandwidth", freq, bandwidth)
            }
            Tuning::FM {
                low_cut,
                high_cut,
                frequency: freq,
            } => {
                write!(f, "FM: {} Hz, -{}->{} Hz", freq, low_cut, high_cut)
            }
            Tuning::LSB {
                low_cut,
                high_cut,
                frequency: freq,
            } => {
                write!(f, "LSB: {} Hz, -{}->{} Hz", freq, low_cut, high_cut)
            }
            Tuning::USB {
                low_cut,
                high_cut,
                frequency: freq,
            } => {
                write!(f, "USB: {} Hz, -{}->{} Hz", freq, low_cut, high_cut)
            }
        }
    }
}
