use mod_api::{self, Hooks};

#[derive(Default)]
struct SimpleVoiceChat;

#[mod_api::horizon_mod]
impl Hooks for SimpleVoiceChat {}
