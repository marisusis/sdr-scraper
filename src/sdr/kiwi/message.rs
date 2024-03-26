use anyhow::Result;
use std::{convert::Into, str::FromStr};
use tokio_tungstenite::tungstenite::Message;

use crate::sdr::Station;

pub struct LoginMessage {
    pub pass: Option<String>,
}

impl LoginMessage {
    pub fn new(pass: Option<String>) -> Self {
        LoginMessage { pass }
    }
}

impl Into<Message> for LoginMessage {
    fn into(self) -> Message {
        if let Some(pass) = self.pass {
            Message::Text(format!("SET auth t=kiwi p={}", pass))
        } else {
            Message::Text("SET auth t=kiwi p=#".to_string())
        }
    }
}

pub struct TuneMessage {
    pub station: Station,
}

impl TuneMessage {
    pub fn new(station: Station) -> Self {
        TuneMessage { station }
    }
}

impl Into<Message> for TuneMessage {
    fn into(self) -> Message {
        match self.station {
            Station::AM(tuning) => Message::Text(format!(
                "SET mod=am low_cut={} high_cut={} freq={}",
                -(tuning.bandwidth / 2) as i32,
                (tuning.bandwidth / 2) as i32,
                tuning.freq
            )),
            _ => {
                todo!();
            }
        }
    }
}

pub struct AgcMessage {
    pub enabled: bool,
    pub hang: i64,
    pub thresh: i64,
    pub slope: i64,
    pub decay: i64,
    pub gain: i64,
}

impl AgcMessage {
    pub fn new(enabled: bool, hang: i64, thresh: i64, slope: i64, decay: i64, gain: i64) -> Self {
        AgcMessage {
            enabled,
            hang,
            thresh,
            slope,
            decay,
            gain,
        }
    }
}

impl Into<Message> for AgcMessage {
    fn into(self) -> Message {
        Message::Text(format!(
            "SET agc={} hang={} thresh={} slope={} decay={} manGain={}",
            if self.enabled { 1 } else { 0 },
            self.hang,
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
