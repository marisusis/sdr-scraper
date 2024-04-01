mod audio;
mod config;
mod sdr;

use colored::Colorize;

use sdr::kiwi::{KiwiSDRScraper, KiwiSDRScraperSettings};
use sdr::Tuning;

use url::Url;

use crate::config::{Config, SDRKind};
use crate::sdr::{SDRScraper, ScraperStatus};

#[tokio::main]
// Use multi threading
async fn main() {
    // setup_logger().unwrap();
    simple_logger::init_with_level(log::Level::Info).unwrap();
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
    config.stations.iter().for_each(|station_config| {
        let endpoint = "ws://".to_owned() + &station_config.endpoint;
        let endpoint = Url::parse(&endpoint).unwrap();

        log::debug!(
            "found {} at {}",
            "KiwiSDR".green(),
            endpoint.to_string().green()
        );

        station_config.frequency.iter().for_each(|frequency| {
            // name in megahertz
            let name = format!("{}_{:.0}", station_config.name.clone(), frequency / 1_000.0);
            log::debug!("tuning to {}", frequency.to_string().green());
            stations.push(Box::new(KiwiSDRScraper::new(KiwiSDRScraperSettings {
                name: name,
                endpoint: endpoint.clone(),
                password: station_config.password.clone(),
                agc: station_config.agc,
                station: Tuning::USB {
                    low_cut: 300,
                    high_cut: 2700,
                    frequency: frequency.to_owned(),
                },
            })));
        });
    });

    for station in &mut stations {
        log::info!("starting {}", station.name().green());
        match station.start().await {
            Ok(_) => {}
            Err(e) => log::error!("error starting {}: {}", station.name(), e.to_string().red()),
        }
    }

    tokio::signal::ctrl_c().await.unwrap();
    println!();

    for station in &mut stations {
        log::info!("stopping {}", station.name().green());
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
