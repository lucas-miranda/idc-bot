
/// An user made some action related to a voice channel.
pub enum VoiceMoveAction {
    /// Other state change has happened, such as mute or deaf.
    Undefined,

    /// Entering a voice channel
    Enter,

    /// Leaving a voice channel.
    Leave,

    /// Switching a voice channel to another voice channel.
    Moving,
}
