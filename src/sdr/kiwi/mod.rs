mod message;

use anyhow::Result;
pub use message::AgcMessage;
pub use message::LoginMessage;
pub use message::SetCompressionMessage;
pub use message::SetIdentityMessage;
pub use message::SetLocationMessage;
pub use message::TuneMessage;
use std::sync::{Arc, Mutex};
use tokio_tungstenite::tungstenite::Message;
use url::Url;

use super::SDRError;

pub struct KiwiSDR {
    inner: Arc<Mutex<KiwiSDRInner>>,
}

impl Clone for KiwiSDR {
    fn clone(&self) -> Self {
        KiwiSDR {
            inner: self.inner.clone(),
        }
    }
}

impl KiwiSDR {
    pub fn new(endpoint: Url, password: Option<String>) -> Self {
        KiwiSDR {
            inner: Arc::new(Mutex::new(KiwiSDRInner { endpoint, password })),
        }
    }

    fn send_message(&self, message: impl Into<Message>) -> Result<()> {
        Ok(())
    }

    async fn connect(&self) -> Result<(), SDRError> {
        if self.is_connected() {
            log::warn!("Attempted to connect to an already connected SDR");
        }

        Ok(())
    }

    fn is_connected(&self) -> bool {
        true
    }

    fn is_ready(&self) -> bool {
        true
    }

    fn disconnect(&self) -> Result<(), SDRError> {
        Ok(())
    }
}

pub struct KiwiSDRInner {
    endpoint: Url,
    password: Option<String>,
}

impl KiwiSDRInner {
    fn connect(&self) -> Result<()> {
        Ok(())
    }

    fn send_message(message: impl Into<Message>) -> Result<()> {
        Ok(())
    }
}
