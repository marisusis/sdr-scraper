use std::sync::Arc;

use colored::Colorize;
use hound::{WavSpec, WavWriter};
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
    sdr: Arc<Mutex<Box<KiwiSDR>>>,
    status: ScraperStatus,
    token: CancellationToken,
    writer: Arc<Mutex<Writer>>,
}

impl KiwiSDRScraper {
    pub fn new(settings: KiwiSDRScraperSettings) -> KiwiSDRScraper {
        KiwiSDRScraper {
            settings: settings.clone(),
            sdr: Arc::new(Mutex::new(Box::new(KiwiSDR::new(settings.endpoint)))),
            status: ScraperStatus::Stopped,
            token: CancellationToken::new(),
            writer: Arc::new(Mutex::new(Writer::new(
                std::fs::File::create(format!("{}.wav", settings.name)).unwrap(),
            ))),
        }
    }
}

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
        let writer_clone = self.writer.clone();
        tokio::spawn(async move {
            let writer = writer_clone;
            let event_loop = async {
                log::debug!("spawned event thread for {}", settings.name.green());
                loop {
                    if let Some(event) = {
                        // Make sure SDR instance is dropped immediately after fetching the latest message
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

                                let reconnect_timeout = std::time::Duration::from_secs(1);

                                log::info!(
                                    "{}: reconnecting in {}",
                                    settings.name.yellow(),
                                    reconnect_timeout.as_secs()
                                );

                                tokio::time::sleep(reconnect_timeout).await;

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
                            KiwiEvent::Ready => {
                                log::debug!("{} is ready", settings.name.green());

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

                                sdr.send_message(KiwiClientMessage::Tune(settings.station.clone()))
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
                            KiwiEvent::SoundData(data) => {
                                log::debug!(
                                    "{}: received {} samples",
                                    settings.name.blue(),
                                    data.len()
                                );
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
}
