use super::message::KiwiServerMessage;

#[derive(Debug)]
pub enum KiwiCloseReason {
    ServerClosed,
    AuthenticationFailed,
}

#[derive(Debug)]
pub enum KiwiEvent {
    Close(KiwiCloseReason),
    Message(String),
    SoundData(Vec<u8>),
    Ping,
}
