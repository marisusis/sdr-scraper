#[derive(Debug)]
pub enum KiwiCloseReason {
    ServerClosed,
    AuthenticationFailed,
}

#[derive(Debug)]
pub enum KiwiEvent {
    Close(KiwiCloseReason),
    Message(String),
    Ready(u32),
    SoundData(Vec<u8>),
    Ping,
}
