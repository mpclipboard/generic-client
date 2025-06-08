use anyhow::anyhow;
use mpclipboard_common::{AuthResponse, Clip};
use tokio_websockets::Message;

pub(crate) enum WebsocketMessage {
    Ping,
    Pong,
    Clip(Clip),
    AuthResponse(AuthResponse),
}

impl TryFrom<&Message> for WebsocketMessage {
    type Error = anyhow::Error;

    fn try_from(message: &Message) -> Result<Self, Self::Error> {
        if message.is_ping() {
            Ok(WebsocketMessage::Ping)
        } else if message.is_pong() {
            Ok(WebsocketMessage::Pong)
        } else if let Ok(clip) = Clip::try_from(message) {
            Ok(WebsocketMessage::Clip(clip))
        } else if let Ok(auth_response) = AuthResponse::try_from(message) {
            Ok(WebsocketMessage::AuthResponse(auth_response))
        } else {
            Err(anyhow!("unknown message type: {message:?}"))
        }
    }
}
