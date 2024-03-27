use std::sync::Arc;

use colored::Colorize;
use tokio::sync::Mutex;
use url::Url;

use crate::sdr::{
    scraper::{SDRScraper, ScraperStatus},
    Tuning,
};

use super::KiwiSDR;

#[derive(Clone)]
pub struct KiwiSDRScraperSettings {
    pub name: String,
    pub endpoint: Url,
    pub password: Option<String>,
    pub station: Tuning,
    pub agc: bool,
}

pub struct KiwiSDRScraper {
    settings: KiwiSDRScraperSettings,
    sdr: Option<Arc<Mutex<Box<KiwiSDR>>>>,
    status: ScraperStatus,
}

impl KiwiSDRScraper {
    pub fn new(settings: KiwiSDRScraperSettings) -> KiwiSDRScraper {
        KiwiSDRScraper {
            settings,
            sdr: None,
            status: ScraperStatus::Stopped,
        }
    }
}

#[async_trait::async_trait]
impl SDRScraper for KiwiSDRScraper {
    async fn start(&mut self) -> anyhow::Result<()> {
        match self.status {
            ScraperStatus::Running => {
                log::warn!("SDR for {} is already running", self.settings.name);
                return Ok(());
            }
            ScraperStatus::Stopped => {}
        }

        log::debug!("Starting scraper for {}", self.settings.name.green());

        let sdr = KiwiSDR::connect(
            self.settings.name.clone(),
            self.settings.endpoint.clone(),
            self.settings.password.clone(),
        )
        .await?;

        self.sdr = Some(Arc::new(Mutex::new(Box::new(sdr))));

        // Spawn watch task
        {
            let settings = self.settings.clone();
            let sdr = self.sdr.clone().unwrap();
            tokio::spawn(async move {
                log::debug!("Spawned watch thread for {}", settings.name.green());
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    let mut sdr = sdr.lock().await;
                    if !sdr.connected() {
                        log::warn!(
                            "{} is not connected, attempting to reconnect...",
                            settings.name
                        );
                        // new sdr
                        let new_sdr = KiwiSDR::connect(
                            settings.name.clone(),
                            settings.endpoint.clone(),
                            settings.password.clone(),
                        );

                        match new_sdr.await {
                            Ok(new_sdr) => {
                                *sdr = Box::new(new_sdr);
                                log::info!("Reconnected to {}", settings.name.green());
                            }
                            Err(e) => {
                                log::error!("Failed to reconnect to SDR: {}", e);
                            }
                        }
                    }
                }
            });
        }

        self.status = ScraperStatus::Running;

        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        log::debug!("Stopping scraper for {}", self.settings.name.green());

        if self.sdr.is_none() {
            log::warn!("SDR for {} is already stopped", self.settings.name);
            return Ok(());
        }

        let sdr = self.sdr.take().unwrap();
        let sdr = sdr.lock().await;

        sdr.shutdown()?;
        self.status = ScraperStatus::Stopped;

        Ok(())
    }

    fn status(&self) -> ScraperStatus {
        self.status.clone()
    }

    fn name(&self) -> &str {
        &self.settings.name
    }
}
