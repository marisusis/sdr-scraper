mod audio;
mod config;
mod sdr;

use std::future::IntoFuture;
use std::sync::Arc;

use axum::extract::State;
use axum::{Json, Router};
use colored::Colorize;

use reqwest::StatusCode;
use sdr::kiwi::{KiwiSDRScraper, KiwiSDRScraperSettings, KiwiScraperStats};
use sdr::Tuning;

use tokio::sync::Mutex;
use url::Url;

use crate::config::{Config, SDRKind};
use crate::sdr::{SDRScraper, ScraperStatus};

struct AppState {
    stats: Vec<KiwiScraperStats>,
}

async fn root(
    State(app_state): State<Arc<Mutex<AppState>>>,
) -> Result<Json<Vec<KiwiScraperStats>>, StatusCode> {
    let state = app_state.lock().await;
    Ok(Json(state.stats.clone()))
}

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

    let state = Arc::new(Mutex::new(AppState { stats: Vec::new() }));

    let router = Router::new()
        // `GET /` goes to `root`
        .route("/", axum::routing::get(root))
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    // axum::serve(listener, router).await.unwrap();
    //

    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let token_clone = cancellation_token.clone();
    tokio::spawn(async move {
        let future = axum::serve(listener, router).into_future();
        tokio::select! {
            _ = future => {}
            _ = token_clone.cancelled() => {}
        }
    });

    loop {
        let future1 = async {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let mut state = state.lock().await;
            let mut stats = Vec::new();
            for station in &stations {
                stats.push(station.get_stats());
            }

            state.stats = stats;
        };

        let future2 = tokio::signal::ctrl_c();

        tokio::select! {
            _ = future1 => {
            }
            _ = future2 => {
                log::info!("ctrl-c received");
                break;
            }
        }
    }

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
