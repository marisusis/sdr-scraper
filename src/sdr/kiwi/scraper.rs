use std::sync::Arc;

use colored::Colorize;
use tokio::sync::Mutex;
use url::Url;

use crate::sdr::{
    kiwi::event::{KiwiCloseReason, KiwiEvent},
    scraper::{SDRScraper, ScraperStatus},
    Tuning,
};

use super::{message::KiwiServerMessage, KiwiSDR};

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

        log::debug!("starting scraper for {}", self.settings.name.green());

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
                log::debug!("spawned event thread for {}", settings.name.green());
                loop {
                    let mut sdr = sdr.lock().await;
                    if let Some(event) = sdr.read_event().await {
                        match event {
                            KiwiEvent::Close(reason) => {
                                match reason {
                                    KiwiCloseReason::ServerClosed => {
                                        log::error!(
                                            "{}: server closed connection",
                                            settings.name.red()
                                        );
                                    }
                                    KiwiCloseReason::AuthenticationFailed => {
                                        log::error!(
                                            "{}: authentication failed",
                                            settings.name.red()
                                        );
                                    }
                                }

                                let new_sdr = KiwiSDR::connect(
                                    settings.name.clone(),
                                    settings.endpoint.clone(),
                                    settings.password.clone(),
                                );

                                match new_sdr.await {
                                    Ok(new_sdr) => {
                                        *sdr = Box::new(new_sdr);
                                        log::info!("reconnected to {}", settings.name.green());
                                    }
                                    Err(e) => {
                                        log::error!("railed to reconnect to SDR: {}", e);
                                    }
                                }
                                break;
                            }
                            KiwiEvent::Message(msg) => {
                                log::debug!(
                                    "{}: {}",
                                    settings.name.blue(),
                                    if msg.len() > 100 {
                                        format!("{:.100}...", msg)
                                    } else {
                                        msg
                                    }
                                );
                            }
                            _ => {}
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
