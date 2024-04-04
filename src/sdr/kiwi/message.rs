use crate::sdr::Tuning;
use percent_encoding::{percent_encode, NON_ALPHANUMERIC};

use tokio_tungstenite::tungstenite::Message;

pub struct LoginMessage {
    pub pass: Option<String>,
}

impl LoginMessage {
    pub fn new(pass: Option<String>) -> Self {
        LoginMessage { pass }
    }
}

impl From<LoginMessage> for Message {
    fn from(msg: LoginMessage) -> Message {
        if let Some(pass) = msg.pass {
            Message::Text(format!("SET auth t=kiwi p={}", pass))
        } else {
            Message::Text("SET auth t=kiwi p=#".to_string())
        }
    }
}

#[derive(Debug)]
pub enum KiwiClientMessage {
    AROk {
        input_rate: i64,
        output_rate: i64,
    },
    Login(Option<String>),
    KeepAlive,
    SetCompression(bool),
    SetIdentity(String),
    SetLocation(String),
    SetAgc {
        enabled: bool,
        decay: i64,
        hang: bool,
        slope: i64,
        thresh: i64,
        gain: i64,
    },
    Tune(Tuning),
    Unknown(String),
}

impl From<KiwiClientMessage> for Message {
    fn from(msg: KiwiClientMessage) -> Message {
        match msg {
            KiwiClientMessage::KeepAlive => Message::Text("SET keepalive".to_string()),
            KiwiClientMessage::Login(password) => {
                if let Some(pass) = password {
                    Message::Text(format!("SET auth t=kiwi p={}", pass))
                } else {
                    Message::Text("SET auth t=kiwi p=#".to_string())
                }
            }
            KiwiClientMessage::AROk {
                input_rate,
                output_rate,
            } => Message::Text(format!("SET AR OK in={} out={}", input_rate, output_rate)),
            KiwiClientMessage::Tune(tuning) => match tuning {
                Tuning::AM {
                    bandwidth,
                    frequency,
                } => Message::Text(format!(
                    "SET mod=am low_cut={} high_cut={} freq={}",
                    -(bandwidth / 2) as i32,
                    (bandwidth / 2) as i32,
                    frequency / 1000.0
                )),
                Tuning::FM {
                    low_cut,
                    high_cut,
                    frequency,
                } => Message::Text(format!(
                    "SET mod=fm low_cut={} high_cut={} freq={}",
                    low_cut,
                    high_cut,
                    frequency / 1000.0
                )),
                Tuning::LSB {
                    low_cut,
                    high_cut,
                    frequency,
                } => Message::Text(format!(
                    "SET mod=lsb low_cut={} high_cut={} freq={}",
                    low_cut,
                    high_cut,
                    frequency / 1000.0
                )),
                Tuning::USB {
                    low_cut,
                    high_cut,
                    frequency,
                } => Message::Text(format!(
                    "SET mod=usb low_cut={} high_cut={} freq={}",
                    low_cut,
                    high_cut,
                    frequency / 1000.0
                )),
                _ => {
                    todo!();
                }
            },
            KiwiClientMessage::SetCompression(enabled) => {
                Message::Text(format!("SET compression={}", if enabled { 1 } else { 0 }))
            }
            KiwiClientMessage::SetIdentity(identity) => Message::Text(format!(
                "SET ident_user={}",
                percent_encode(identity.as_bytes(), NON_ALPHANUMERIC)
            )),
            KiwiClientMessage::SetLocation(location) => Message::Text(format!(
                "SET geoloc={}",
                percent_encode(location.as_bytes(), NON_ALPHANUMERIC)
            )),
            KiwiClientMessage::SetAgc {
                enabled,
                decay,
                hang,
                slope,
                thresh,
                gain,
            } => Message::Text(format!(
                "SET agc={} hang={} thresh={} slope={} decay={}  manGain={}",
                if enabled { 1 } else { 0 },
                if hang { 1 } else { 0 },
                thresh,
                slope,
                decay,
                gain
            )),
            KiwiClientMessage::Unknown(msg) => Message::Text(msg),
        }
    }
}

#[derive(Debug)]
pub enum KiwiServerMessage {
    AuthenticationResult(bool),
    Unknown(String),
    AudioInit(u32),
    SoundData(Vec<u8>),
}

impl From<String> for KiwiServerMessage {
    fn from(msg: String) -> KiwiServerMessage {
        if msg.contains("badp") {
            if msg.eq("badp=1") {
                KiwiServerMessage::AuthenticationResult(false)
            } else {
                KiwiServerMessage::AuthenticationResult(true)
            }
        } else if msg.contains("audio_init") {
            let parts = msg.split_whitespace().collect::<Vec<&str>>();
            let parts = parts[1].split('=').collect::<Vec<&str>>();
            let rate = parts[1].parse::<u32>().unwrap();

            KiwiServerMessage::AudioInit(rate)
        } else {
            KiwiServerMessage::Unknown(msg)
        }
    }
}
