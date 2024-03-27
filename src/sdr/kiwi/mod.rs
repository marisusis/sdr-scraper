pub mod event;
mod message;
mod scraper;

use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};

pub use message::LoginMessage;

use colored::Colorize;
use rand::Rng;
pub use scraper::{KiwiSDRScraper, KiwiSDRScraperSettings};
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;
use url::Url;

use crate::sdr::kiwi::{event::KiwiCloseReason, message::KiwiServerMessage};

use self::event::KiwiEvent;

pub struct KiwiSDR {
    cancellation_token: CancellationToken,
    event_channel_rx: tokio::sync::mpsc::Receiver<KiwiEvent>,
    // message_channel_tx: tokio::sync::mpsc::Sender<KiwiEvent>,
}

impl KiwiSDR {
    pub async fn connect(
        name: String,
        endpoint: Url,
        password: Option<String>,
    ) -> anyhow::Result<Self> {
        log::debug!(
            "Connecting to KiwiSDR at {} with password {:?}",
            endpoint,
            password
        );

        let number = {
            let mut rng = rand::thread_rng();
            rng.gen_range(0..1000)
        };

        // Connect and login
        let (ws_socket, _) =
            tokio_tungstenite::connect_async(format!("{}/kiwi/{}/SND", endpoint.clone(), number))
                .await?;
        let (mut write, read) = ws_socket.split();
        let message: Message = LoginMessage::new(password).into();
        write.send(message).await?;
        let write = Arc::new(Mutex::new(write));

        // Create event channel
        let (tx, rx) = tokio::sync::mpsc::channel::<KiwiEvent>(100);

        let token = CancellationToken::new();
        let token_clone = token.clone();
        tokio::spawn(async move {
            let token = token_clone;
            tokio::select! {
                _ = token.cancelled() => {
                    log::debug!("Connection cancelled");
                }
                _ = async {
                    read.for_each(|msg| async {
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
                                tx.send(KiwiEvent::Message(msg)).await.unwrap();
                            }
                        }
                        Message::Binary(bin) => {
                            let code = String::from_utf8(bin[..3].to_vec()).unwrap();
                            match code.as_str() {
                                "SND" => {
                                    let data = bin[4..].to_vec();
                                    tx.send(KiwiEvent::SoundData(data)).await.unwrap();
                                }
                                "MSG" => {
                                    let str = match String::from_utf8(bin[4..].to_vec()) {
                                        Ok(str) => str,
                                        Err(e) => {
                                            log::error!("Error decoding binary message: {:?}", e);
                                            return;
                                        }
                                    };

                                    let message: KiwiServerMessage = str.into();

                                    if let KiwiServerMessage::AuthenticationResult(result) = message {
                                        if !result {
                                            tx.send(KiwiEvent::Close(KiwiCloseReason::AuthenticationFailed)).await.unwrap();
                                            token.cancel();
                                            return;
                                        }
                                    }

                                    if let KiwiServerMessage::Unknown(msg) = message {
                                        tx.send(KiwiEvent::Message(msg)).await.unwrap();
                                    }


                                }
                                _ => {
                                    let str = match String::from_utf8(bin[4..].to_vec()) {
                                        Ok(str) => str,
                                        Err(e) => {
                                            log::error!("Error decoding binary message: {:?}", e);
                                            return;
                                        }
                                    };
                                    log::debug!("{} Received binary message: {:?}", name, if str.len() > 30 { &str[..30] } else { &str });
                                }
                            }
                        }
                        Message::Close(close) => {
                            token.cancel();
                            tx.send(KiwiEvent::Close(KiwiCloseReason::ServerClosed)).await.unwrap();
                            return;
                        }
                        Message::Ping(ping) => {
                            tx.send(KiwiEvent::Ping).await.unwrap();
                        }
                        Message::Pong(pong) => {
                            log::debug!("Received pong message: {:?}", pong);
                        }
                        _ => {}
                    }
                }).await
                } => {}
            };
        });

        Ok(Self {
            cancellation_token: token,
            event_channel_rx: rx,
        })
    }

    fn connected(&self) -> bool {
        !self.cancellation_token.is_cancelled()
    }

    pub async fn read_event(&mut self) -> Option<KiwiEvent> {
        self.event_channel_rx.recv().await
    }

    fn send_message(&self, message: impl Into<Message>) -> anyhow::Result<()> {
        let msg: Message = message.into();
        log::debug!("Sending message: {:?}", msg);
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
