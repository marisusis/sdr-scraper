mod message;
mod scraper;

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
pub use message::AgcMessage;
pub use message::KiwiMessage;
pub use message::LoginMessage;
pub use message::SetCompressionMessage;
pub use message::SetIdentityMessage;
pub use message::SetLocationMessage;
pub use message::TuneMessage;
use rand::Rng;
pub use scraper::{KiwiSDRScraper, KiwiSDRScraperSettings};
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;
use url::Url;

use super::SDRError;

pub struct KiwiSDR {
    cancellation_token: CancellationToken,
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

        let (ws_socket, _) =
            tokio_tungstenite::connect_async(format!("{}/kiwi/{}/SND", endpoint.clone(), number))
                .await?;
        let (mut write, mut read) = ws_socket.split();

        let message: Message = LoginMessage::new(password).into();
        log::debug!("Sending message: {:?}", message);
        write.send(message).await?;

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
                            log::debug!("Received message: {:?}", text);
                        }
                        Message::Binary(bin) => {
                            let code = String::from_utf8(bin[..3].to_vec()).unwrap();
                            match code.as_str() {
                                "SND" => {
                                    log::debug!("{} Received SND message", name);
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
                            log::debug!("Received close message: {:?}", close);
                            token.cancel();
                        }
                        Message::Ping(ping) => {
                            log::debug!("Received ping message: {:?}", ping);
                        }
                        Message::Pong(pong) => {
                            log::debug!("Received pong message: {:?}", pong);
                        }
                        _ => {
                            log::debug!("Received message: {:?}", msg);
                        }
                    }
                }).await
                } => {}
            };
        });

        Ok(Self {
            cancellation_token: token,
        })
    }

    fn connected(&self) -> bool {
        !self.cancellation_token.is_cancelled()
    }

    fn read_message(&self) -> Option<Message> {
        None
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
