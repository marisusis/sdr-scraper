use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use colored::Colorize;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use url::Url;

use crate::{
    audio::Writer,
    sdr::{
        kiwi::{
            event::{KiwiCloseReason, KiwiEvent},
            message::KiwiClientMessage,
        },
        scraper::{SDRScraper, ScraperStatus},
        Tuning,
    },
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

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct KiwiScraperStats {
    name: String,
    rssi: f64,
}

#[derive(Debug)]
pub struct AtomicF64 {
    storage: AtomicU64,
}
impl AtomicF64 {
    pub fn new(value: f64) -> Self {
        let as_u64 = value.to_bits();
        Self {
            storage: AtomicU64::new(as_u64),
        }
    }
    pub fn store(&self, value: f64, ordering: Ordering) {
        let as_u64 = value.to_bits();
        self.storage.store(as_u64, ordering)
    }
    pub fn load(&self, ordering: Ordering) -> f64 {
        let as_u64 = self.storage.load(ordering);
        f64::from_bits(as_u64)
    }
}

pub struct KiwiSDRScraper {
    settings: KiwiSDRScraperSettings,
    sdr: Arc<Mutex<Box<KiwiSDR>>>,
    status: ScraperStatus,
    token: CancellationToken,
    writer: Arc<Mutex<Writer>>,
    rssi: Arc<AtomicF64>,
}

impl KiwiSDRScraper {
    pub fn new(settings: KiwiSDRScraperSettings) -> KiwiSDRScraper {
        KiwiSDRScraper {
            settings: settings.clone(),
            sdr: Arc::new(Mutex::new(Box::new(KiwiSDR::new(settings.endpoint)))),
            status: ScraperStatus::Stopped,
            token: CancellationToken::new(),
            writer: Arc::new(Mutex::new(Writer::new(
                settings.name.clone(),
                std::path::Path::new("./RECORD"),
            ))),
            rssi: Arc::new(AtomicF64::new(0.0)),
        }
    }
}

impl KiwiSDRScraper {}

#[async_trait::async_trait]
impl SDRScraper for KiwiSDRScraper {
    async fn start(&mut self) -> anyhow::Result<()> {
        self.token = CancellationToken::new();

        match self.status {
            ScraperStatus::Running => {
                log::warn!("SDR for {} is already running", self.settings.name);
                return Ok(());
            }
            ScraperStatus::Stopped => {}
        }

        log::debug!("starting scraper for {}", self.settings.name.green());

        let sdr = self.sdr.clone();
        let password = self.settings.password.clone();
        tokio::spawn(async move {
            let mut sdr = sdr.lock().await;
            sdr.connect(password).await.unwrap();
        });

        let settings = self.settings.clone();
        let sdr = self.sdr.clone();
        let token = self.token.clone();
        let rssi_clone = self.rssi.clone();
        let writer_clone = self.writer.clone();
        tokio::spawn(async move {
            let writer = writer_clone;
            let rssi = rssi_clone;
            let event_loop = async {
                log::debug!("spawned event thread for {}", settings.name.green());
                loop {
                    if let Some(event) = {
                        // Make sure SDR instance lock is dropped immediately after fetching the latest message
                        let mut sdr = sdr.lock().await;
                        sdr.read_event(std::time::Duration::from_secs(1)).await
                    } {
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

                                writer.lock().await.close();

                                log::info!("{}: reconnecting in 4...", settings.name.yellow());
                                tokio::time::sleep(std::time::Duration::from_secs(4)).await;

                                match sdr.lock().await.connect(settings.password.clone()).await {
                                    Ok(_) => {
                                        log::info!("{}: reconnected", settings.name.green());
                                    }
                                    Err(e) => {
                                        log::error!(
                                            "{}: failed to reconnect: {}",
                                            settings.name.red(),
                                            e
                                        );
                                    }
                                };
                            }
                            KiwiEvent::Ready(rate) => {
                                log::info!("{} is ready at {} Hz", settings.name.green(), rate);
                                writer.lock().await.set_sample_rate(rate);

                                {
                                    let sdr = sdr.lock().await;

                                    sdr.send_message(KiwiClientMessage::AROk {
                                        input_rate: 12000,
                                        output_rate: 48000,
                                    })
                                    .await
                                    .unwrap();

                                    sdr.send_message(KiwiClientMessage::Unknown(
                                        "SERVER DE CLIENT openwebrx.js SND".to_string(),
                                    ))
                                    .await
                                    .unwrap();

                                    sdr.send_message(KiwiClientMessage::Unknown("SET browser=Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15".to_string()))
                                    .await
                                    .unwrap();

                                    sdr.send_message(KiwiClientMessage::Unknown(
                                        "SET squelch=0 param=0.00".to_string(),
                                    ))
                                    .await
                                    .unwrap();

                                    sdr.send_message(KiwiClientMessage::Tune(
                                        settings.station.clone(),
                                    ))
                                    .await
                                    .unwrap();

                                    sdr.send_message(KiwiClientMessage::SetIdentity(
                                        "W8EDU".to_string(),
                                    ))
                                    .await
                                    .unwrap();

                                    sdr.send_message(KiwiClientMessage::SetLocation(
                                        "Cleveland, OH".to_string(),
                                    ))
                                    .await
                                    .unwrap();

                                    sdr.send_message(KiwiClientMessage::SetAgc {
                                        enabled: true,
                                        decay: 1370,
                                        hang: false,
                                        slope: 6,
                                        thresh: -96,
                                        gain: 70,
                                    })
                                    .await
                                    .unwrap();

                                    sdr.send_message(KiwiClientMessage::SetCompression(true))
                                        .await
                                        .unwrap();
                                }

                                // Start keepalive loop
                                let sdr = sdr.clone();
                                let token = token.clone();
                                let settings = settings.clone();
                                tokio::spawn(async move {
                                    let keepalive_loop = async move {
                                        loop {
                                            tokio::time::sleep(std::time::Duration::from_secs(5))
                                                .await;
                                            sdr.lock()
                                                .await
                                                .send_message(KiwiClientMessage::KeepAlive)
                                                .await
                                                .unwrap();
                                        }
                                    };

                                    tokio::select! {
                                        _ = keepalive_loop => {}
                                        _ = token.cancelled() => {
                                            log::debug!("{}: keepalive loop cancelled", settings.name.yellow());
                                        }
                                    };
                                });
                            }
                            KiwiEvent::SoundData {
                                data,
                                rssi: the_rssi,
                            } => {
                                log::debug!(
                                    "{}: received {} samples",
                                    settings.name.blue(),
                                    data.len()
                                );
                                rssi.store(the_rssi, Ordering::Relaxed);
                                writer.lock().await.write_samples(&data)
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
            };

            tokio::select! {
                _ = event_loop => {}
                _ = token.cancelled() => {
                    log::debug!("{}: event loop cancelled", settings.name.yellow());
                }
            }
        });

        self.status = ScraperStatus::Running;

        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        log::debug!("Stopping scraper for {}", self.settings.name.green());

        self.token.cancel();

        let sdr = self.sdr.lock().await;

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

    fn get_stats(&self) -> KiwiScraperStats {
        KiwiScraperStats {
            rssi: self.rssi.load(Ordering::Relaxed) as f64,
            name: self.settings.name.clone(),
        }
    }
}
