use std::convert::Into;
use tokio_tungstenite::tungstenite::Message;

use crate::sdr::Tuning;

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

pub struct TuneMessage {
    pub station: Tuning,
}

impl TuneMessage {
    pub fn new(station: Tuning) -> Self {
        TuneMessage { station }
    }
}

impl Into<Message> for TuneMessage {
    fn into(self) -> Message {
        match self.station {
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
                frequency: freq,
            } => Message::Text(format!(
                "SET mod=fm low_cut={} high_cut={} freq={}",
                low_cut, high_cut, freq
            )),
            Tuning::LSB {
                low_cut,
                high_cut,
                frequency: freq,
            } => Message::Text(format!(
                "SET mod=lsb low_cut={} high_cut={} freq={}",
                low_cut, high_cut, freq
            )),
            Tuning::USB {
                low_cut,
                high_cut,
                frequency: freq,
            } => Message::Text(format!(
                "SET mod=usb low_cut={} high_cut={} freq={}",
                low_cut, high_cut, freq
            )),
            _ => {
                todo!();
            }
        }
    }
}

pub struct AgcMessage {
    pub enabled: bool,
    pub hang: bool,
    pub thresh: i64,
    pub slope: i64,
    pub decay: i64,
    pub gain: i64,
}

impl Into<Message> for AgcMessage {
    fn into(self) -> Message {
        Message::Text(format!(
            "SET agc={} hang={} thresh={} slope={} decay={} manGain={}",
            if self.enabled { 1 } else { 0 },
            if self.hang { 1 } else { 0 },
            self.thresh,
            self.slope,
            self.decay,
            self.gain
        ))
    }
}
pub struct SetCompressionMessage {
    pub enabled: bool,
}

impl Into<Message> for SetCompressionMessage {
    fn into(self) -> Message {
        Message::Text(format!(
            "SET compression={}",
            if self.enabled { 1 } else { 0 }
        ))
    }
}

pub struct SetIdentityMessage {
    identity: String,
}

impl SetIdentityMessage {
    pub fn new(identity: String) -> Self {
        SetIdentityMessage { identity }
    }
}

impl Into<Message> for SetIdentityMessage {
    fn into(self) -> Message {
        Message::Text(format!("SET ident_user={}", self.identity))
    }
}

pub struct SetLocationMessage {
    location: String,
}

impl SetLocationMessage {
    pub fn new(location: String) -> Self {
        SetLocationMessage { location }
    }
}

impl Into<Message> for SetLocationMessage {
    fn into(self) -> Message {
        Message::Text(format!("SET geoloc={}", self.location))
    }
}

pub enum KiwiMessage {
    KeepAlive,
}

impl Into<Message> for KiwiMessage {
    fn into(self) -> Message {
        match self {
            KiwiMessage::KeepAlive => Message::Text("SET keepalive".to_string()),
        }
    }
}

#[derive(Debug)]
pub enum KiwiServerMessage {
    AuthenticationResult(bool),
    Unknown(String),
    AudioInit,
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
            KiwiServerMessage::AudioInit
        } else {
            KiwiServerMessage::Unknown(msg)
        }
    }
}
