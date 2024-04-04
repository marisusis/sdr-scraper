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
    SoundData { data: Vec<u8>, rssi: f64 },
    Ping,
}
