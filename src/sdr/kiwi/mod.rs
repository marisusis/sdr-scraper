pub mod event;
mod message;
mod scraper;

use std::time::Duration;

use byteorder::{BigEndian, ByteOrder, LittleEndian};
use futures_util::{SinkExt, StreamExt};

use rand::Rng;
pub use scraper::{KiwiSDRScraper, KiwiSDRScraperSettings, KiwiScraperStats};

use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;
use url::Url;

use crate::sdr::kiwi::{event::KiwiCloseReason, message::KiwiServerMessage};

pub use self::{event::KiwiEvent, message::KiwiClientMessage};

#[derive(Deserialize, Serialize)]
pub struct VerResponse {
    #[serde(rename = "maj")]
    pub major: i32,
    #[serde(rename = "min")]
    pub minor: i32,
    #[serde(rename = "ts")]
    pub code: Option<i128>,
}

pub struct KiwiSDR {
    cancellation_token: CancellationToken,
    event_channel_rx: Option<tokio::sync::mpsc::Receiver<KiwiEvent>>,
    message_channel_tx: Option<tokio::sync::mpsc::Sender<KiwiClientMessage>>,
    endpoint: Url,
}

impl KiwiSDR {
    pub fn new(endpoint: Url) -> Self {
        Self {
            cancellation_token: CancellationToken::new(),
            event_channel_rx: None,
            message_channel_tx: None,
            endpoint,
        }
    }

    pub async fn connect(&mut self, password: Option<String>) -> anyhow::Result<()> {
        log::debug!("Connecting to KiwiSDR at {}", self.endpoint.clone());

        let mut url = self.endpoint.clone();
        url.set_scheme("http").unwrap();
        url = url.join("VER").unwrap();

        log::info!("getting version from {}", url);
        let response = reqwest::get(url).await?;
        let version = response.json::<VerResponse>().await?;
        log::info!("KiwiSDR version: {}.{}", version.major, version.minor);

        // reqwest::get
        let number = if let Some(code) = version.code {
            code
        } else {
            let mut rng = rand::thread_rng();
            rng.gen_range(0..1000)
        };

        // Connect and login
        let connect = tokio_tungstenite::connect_async(format!(
            "{}/kiwi/{}/SND",
            self.endpoint.clone(),
            number
        ));

        let (ws_socket, _) = tokio::time::timeout(Duration::from_secs(2), connect)
            .await
            .map_err(|_| anyhow::anyhow!("Connection timeout"))??;

        let (mut write, read) = ws_socket.split();
        write
            .send(KiwiClientMessage::Login(password).into())
            .await?;

        // Create event channels
        let (event_tx, event_rx) = tokio::sync::mpsc::channel::<KiwiEvent>(100);
        let (msg_tx, mut msg_rx) = tokio::sync::mpsc::channel::<KiwiClientMessage>(100);
        self.event_channel_rx = Some(event_rx);
        self.message_channel_tx = Some(msg_tx);

        self.cancellation_token = CancellationToken::new();
        let token_clone = self.cancellation_token.clone();
        let endpoint = self.endpoint.clone();
        tokio::spawn(async move {
            log::debug!("starting event loop for KiwiSDR at {}", endpoint);
            let token = token_clone;
            let read_loop = read.for_each(|msg| async {
                let msg = match msg {
                    Ok(msg) => msg,
                    Err(e) => {
                        log::error!("Error reading message: {:?}", e);
                        return;
                    }
                };

                match msg {
                    Message::Text(text) => {
                        let message: KiwiServerMessage = text.into();
                        if let KiwiServerMessage::Unknown(msg) = message {
                            event_tx.send(KiwiEvent::Message(msg)).await.unwrap();
                        }
                    }
                    Message::Binary(bin) => {
                        let code = String::from_utf8(bin[..3].to_vec()).unwrap();
                        match code.as_str() {
                            "SND" => {
                                let data = bin[3..].to_vec();
                                let _flags = data[0];
                                let _seq = LittleEndian::read_u32(&data[1..5]);
                                let smeter = BigEndian::read_u16(&data[5..7]);

                                let rssi = 0.1 * smeter as f64 - 127.0;
                                log::debug!("RSSI: {}", rssi);

                                let data = data[7..].to_vec();
                                event_tx
                                    .send(KiwiEvent::SoundData { data, rssi })
                                    .await
                                    .unwrap();
                            }
                            "MSG" => {
                                let str = match String::from_utf8(bin[4..].to_vec()) {
                                    Ok(str) => str,
                                    Err(e) => {
                                        log::error!("Error decoding binary message: {:?}", e);
                                        return;
                                    }
                                };

                                match KiwiServerMessage::from(str) {
                                    KiwiServerMessage::Unknown(msg) => {
                                        event_tx.send(KiwiEvent::Message(msg)).await.unwrap();
                                    }
                                    KiwiServerMessage::AuthenticationResult(result) => {
                                        if !result {
                                            token.cancel();
                                            event_tx
                                                .send(KiwiEvent::Close(
                                                    KiwiCloseReason::AuthenticationFailed,
                                                ))
                                                .await
                                                .unwrap();
                                        }
                                    }
                                    KiwiServerMessage::AudioInit(rate) => {
                                        event_tx.send(KiwiEvent::Ready(rate)).await.unwrap();
                                    }
                                    _ => {}
                                }
                            }
                            _ => {
                                let _str = match String::from_utf8(bin[4..].to_vec()) {
                                    Ok(str) => str,
                                    Err(e) => {
                                        log::error!("Error decoding binary message: {:?}", e);
                                        return;
                                    }
                                };
                            }
                        }
                    }
                    Message::Close(_close) => {
                        token.cancel();
                        event_tx
                            .send(KiwiEvent::Close(KiwiCloseReason::ServerClosed))
                            .await
                            .unwrap();
                        return;
                    }
                    Message::Ping(_ping) => {
                        event_tx.send(KiwiEvent::Ping).await.unwrap();
                    }
                    Message::Pong(pong) => {
                        log::debug!("Received pong message: {:?}", pong);
                    }
                    _ => {}
                }
            });

            tokio::select! {
                _ = token.cancelled() => {

                }
                _ = read_loop => {
                    panic!("Read loop ended");
                }
            };
        });

        let token_clone = self.cancellation_token.clone();
        let endpoint = self.endpoint.clone();
        tokio::spawn(async move {
            log::debug!("starting message loop for KiwiSDR at {}", endpoint);
            let token = token_clone;
            let write_loop = async {
                loop {
                    if let Some(msg) = msg_rx.recv().await {
                        let msg: Message = msg.into();
                        log::debug!("Sending message: {:?}", msg);
                        log::debug!("Message: {:?}", Message::from(msg.clone()));
                        write.send(msg).await.unwrap();
                    }
                }
            };

            tokio::select! {
                _ = token.cancelled() => {
                }
                _ = write_loop => {
                    panic!("Write loop ended");
                }
            };
        });

        Ok(())
    }

    pub async fn read_event(&mut self, timeout: Duration) -> Option<KiwiEvent> {
        if self.event_channel_rx.is_none() {
            return None;
        }

        match tokio::time::timeout(timeout, self.event_channel_rx.as_mut().unwrap().recv()).await {
            Ok(event) => event,
            Err(_) => None,
        }
    }

    pub async fn send_message(&self, message: KiwiClientMessage) -> anyhow::Result<()> {
        log::debug!("Sending message: {:?}", message);
        self.message_channel_tx
            .as_ref()
            .unwrap()
            .send(message)
            .await?;
        Ok(())
    }

    fn shutdown(&self) -> anyhow::Result<()> {
        log::debug!("Shutting down KiwiSDR");
        self.cancellation_token.cancel();
        Ok(())
    }
}

// pub struct KiwiSDRInner {
//     endpoint: Url,
//     password: Option<String>,
// }

// impl KiwiSDRInner {
//     fn connect(&self) -> Result<()> {
//         Ok(())
//     }

//     fn send_message(message: impl Into<Message>) -> Result<()> {
//         Ok(())
//     }
// }
