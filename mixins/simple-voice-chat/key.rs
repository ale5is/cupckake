fn simple_voice_chat_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    crate::game::simple_voice_chat::on_key(game, key, pressed);
}
