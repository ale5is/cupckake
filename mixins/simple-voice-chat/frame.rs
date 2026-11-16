fn frame(game: &mut crate::game::Game<'_>, elapsed: std::time::Duration) {
    crate::game::simple_voice_chat::on_frame(game, elapsed);
    for (channel, data) in crate::game::simple_voice_chat::drain_outbound() {
        let result = game.native_net_send_to_server(&channel, &data);
        crate::game::simple_voice_chat::on_net_send_result(result);
    }
}
