use super::message::KiwiServerMessage;

pub enum KiwiEvent {
    Close,
    Message(KiwiServerMessage),
    Ping,
}
