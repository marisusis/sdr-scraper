mod audio;
mod config;
mod sdr;

use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt};
use colored::Colorize;
use futures_util::{SinkExt, StreamExt};
use hound::{WavSpec, WavWriter};
use rand::Rng;
use sdr::kiwi::{KiwiSDRScraper, KiwiSDRScraperSettings};
use sdr::{kiwi::TuneMessage, Tuning};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use std::{path::Path, pin::Pin, sync::Arc};
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;
use url::Url;

use crate::config::{Config, SDRKind};
use crate::sdr::{SDRScraper, ScraperStatus};
use crate::{
    audio::ima_adpcm::IMA_ADPCM_Decoder,
    sdr::kiwi::{
        AgcMessage, KiwiMessage, LoginMessage, SetCompressionMessage, SetIdentityMessage,
        SetLocationMessage,
    },
};

fn setup_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            let color = match record.level() {
                log::Level::Error => "red",
                log::Level::Warn => "yellow",
                log::Level::Info => "green",
                log::Level::Debug => "blue",
                log::Level::Trace => "magenta",
            };

            let colored_level = format!("{}", record.level()).color(color);
            out.finish(format_args!(
                "[{} {} {}] {}",
                humantime::format_rfc3339_millis(SystemTime::now()),
                colored_level,
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}

#[tokio::main]
async fn main() {
    // setup_logger().unwrap();
    simple_logger::init_with_level(log::Level::Debug).unwrap();
    log::info!("welcome to {}!", "SDR Scraper".bold().white());

    // Read sdrs.json
    let config = match std::fs::read_to_string("config.json") {
        Ok(sdrs) => sdrs,
        Err(e) => {
            log::error!("error reading config.json: {}", e.to_string().red());
            std::process::exit(1);
        }
    };

    // Parse JSON
    let config: Config = match serde_json::from_str(&config) {
        Ok(sdrs) => sdrs,
        Err(e) => {
            log::error!("error parsing config.json: {}", e.to_string().red());
            std::process::exit(1);
        }
    };

    log::info!(
        "using identity {} from {}",
        config.identity.green(),
        config.location.green()
    );

    // Iterate for each station
    log::info!(
        "loading {} stations...",
        config.stations.len().to_string().green()
    );
    let mut stations: Vec<Box<dyn SDRScraper>> = Vec::new();
    config
        .stations
        .iter()
        .for_each(|station_config| match station_config.kind {
            SDRKind::KiwiSDR => {
                let endpoint = "ws://".to_owned() + &station_config.endpoint;
                let endpoint = Url::parse(&endpoint).unwrap();

                log::debug!(
                    "found {} at {}",
                    "KiwiSDR".green(),
                    endpoint.to_string().green()
                );
                stations.push(Box::new(KiwiSDRScraper::new(KiwiSDRScraperSettings {
                    name: station_config.name.clone(),
                    endpoint: endpoint,
                    password: station_config.password.clone(),
                    agc: station_config.agc,
                    station: station_config.tuning.clone(),
                })));
            }
        });

    for station in &mut stations {
        match station.start().await {
            Ok(_) => {}
            Err(e) => log::error!("error starting {}: {}", station.name(), e.to_string().red()),
        }
    }

    tokio::signal::ctrl_c().await.unwrap();

    for station in &mut stations {
        if station.status() == ScraperStatus::Stopped {
            continue;
        }
        match station.stop().await {
            Ok(_) => {}
            Err(e) => log::error!("error stopping {}: {}", station.name(), e.to_string().red()),
        }
    }

    log::info!("{}", "goodbye!")
}
