mod voice_move_action;
use enum_display::EnumDisplay;
pub use voice_move_action::VoiceMoveAction;

mod voice_channel_manager;
pub use voice_channel_manager::VoiceChannelManager;

mod broadcaster;
pub use broadcaster::*;

mod prepared_message;
pub use prepared_message::PreparedMessage;

mod broadcaster_creation_error;
pub use broadcaster_creation_error::BroadcasterCreationError;

mod sending_message_error;
pub use sending_message_error::SendingMessageError;

mod voice_channel_manager_creation_error;
pub use voice_channel_manager_creation_error::VoiceChannelManagerCreationError;

#[derive(Hash, PartialEq, Eq, Clone, Debug, EnumDisplay)]
pub enum MessageKind {
    CallOpened,
}

impl MessageLabel for MessageKind {
}
