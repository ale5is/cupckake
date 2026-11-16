use std::sync::Mutex;
use std::time::Instant;
use svc_client::protocol;
pub use svc_client::runtime::{state, Glue};

pub const ID: &str = "simple_voice_chat";
pub const NAME: &str = "Simple Voice Chat";
pub const VERSION: &str = "1.0.3";
pub const DESCRIPTION: &str = "Proximity voice chat, push-to-talk, player mute controls and a live voice HUD.";
static OUT: Mutex<Vec<(String, Vec<u8>)>> = Mutex::new(Vec::new());
fn locked() -> std::sync::MutexGuard<'static, Glue> { state().lock().unwrap_or_else(|e| e.into_inner()) }
pub fn queue_outbound(channel: String, body: Vec<u8>) { OUT.lock().unwrap_or_else(|e| e.into_inner()).push((channel, body)); }
pub fn drain_outbound() -> Vec<(String, Vec<u8>)> { std::mem::take(&mut *OUT.lock().unwrap_or_else(|e| e.into_inner())) }
fn parse_muted_players(value: Option<String>) -> std::collections::HashSet<u128> {
    value.unwrap_or_default().split(',').filter_map(|v| v.trim().parse().ok()).collect()
}
fn save_preferences(game: &crate::game::Game<'_>, g: &Glue) {
    game.native_set_setting(ID, "muted", if g.cfg.muted { "true" } else { "false" });
    game.native_set_setting(ID, "mute_all", if g.mute_all { "true" } else { "false" });
    game.native_set_setting(ID, "show_hud", if g.show_hud { "true" } else { "false" });
    let mut ids: Vec<String> = g.muted_players.iter().map(u128::to_string).collect(); ids.sort();
    game.native_set_setting(ID, "muted_players", &ids.join(","));
}
pub fn load_preferences(game: &crate::game::Game<'_>) {
    let muted = game.native_setting(ID, "muted").as_deref() == Some("true");
    let mute_all = game.native_setting(ID, "mute_all").as_deref() == Some("true");
    let show_hud = game.native_setting(ID, "show_hud").as_deref() != Some("false");
    let muted_players = parse_muted_players(game.native_setting(ID, "muted_players"));
    let mut g = locked(); g.cfg.muted = muted; g.mute_all = mute_all; g.show_hud = show_hud; g.muted_players = muted_players;
    if let Some(session) = &mut g.session { session.set_muted(muted); }
}
pub fn world_open_ready(game: &mut crate::game::Game<'_>) {
    load_preferences(game); let mut g = locked(); g.world_open();
    let (channel, body) = Glue::handshake(); queue_outbound(channel, body);
    let notes = g.start_audio(); g.notes.extend(notes);
}
pub fn world_closed() { locked().world_close(); }
pub fn world_lifecycle(game: &mut crate::game::Game<'_>, phase: &crate::game::native_lifecycle::WorldPhase) {
    use crate::game::native_lifecycle::WorldPhase::{WorldCloseFailed, WorldCloseFinal, WorldOpenReady};
    match phase { WorldOpenReady => world_open_ready(game), WorldCloseFinal | WorldCloseFailed => world_closed(), _ => {} }
}
pub fn on_net(channel: &str, data: &[u8]) { locked().on_net(channel, data, None, Instant::now()); }
pub fn on_player_submit(_player: &crate::game::native_mob_draw::NativeMobRender<'_>, _who: &crate::game::native_mob_draw::NativePlayerRender<'_>) {}
pub fn on_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    let ptt = game.native_mod_key(ID, "ptt", "key.keyboard.v"); let mute = game.native_mod_key(ID, "mute", "key.keyboard.m");
    let mut g = locked(); let before = g.cfg.muted; let feedback = g.on_key(key, pressed, &ptt, &mute); let after = g.cfg.muted;
    if before != after { queue_outbound(protocol::UPDATE_STATE.to_string(), protocol::update_state(after)); save_preferences(game, &g); }
    drop(g); if let Some(line) = feedback { game.native_chat(&format!("[voice] {line}")); }
}
pub fn on_command(command: &str, ran: &mut bool) {
    let head = command.trim_start_matches('/').split_whitespace().next().unwrap_or(""); if head != "svc" && head != "vcgroup" { return; }
    let (feedback, outbound) = locked().on_command(command.trim_start_matches('/'));
    for (channel, body) in outbound { queue_outbound(channel, body); } if !feedback.is_empty() { eprintln!("[voice] {feedback}"); } *ran = true;
}
pub fn on_frame(game: &mut crate::game::Game<'_>, _elapsed: std::time::Duration) { let _ = locked().on_frame(Instant::now()); let _ = game; }
pub fn listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    mods.push(crate::game::native_hooks::NativeMod { id: ID.into(), title: format!("{NAME} {VERSION}"), description: DESCRIPTION.into(), keys: vec![
        ["ptt".into(), "Push to talk".into(), "key.keyboard.v".into()], ["mute".into(), "Mute microphone".into(), "key.keyboard.m".into()],
    ] });
}
pub fn settings_page(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    if mod_id != ID { return; } load_preferences(game); let g = locked();
    buttons.push(format!("Microphone: {}", if g.cfg.muted { "MUTED" } else { "ON" }));
    buttons.push(format!("Mute everyone: {}", if g.mute_all { "ON" } else { "OFF" }));
    buttons.push(format!("Voice HUD: {}", if g.show_hud { "ON" } else { "OFF" }));
    for (name, _, muted, _) in g.player_rows(Instant::now()) { buttons.push(format!("{}: {}", name, if muted { "MUTED" } else { "AUDIBLE" })); }
    if g.uuid_by_name.is_empty() { buttons.push("No voice players detected yet".into()); }
}
pub fn settings_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    if mod_id != ID { return; } load_preferences(game); let mut g = locked();
    match button {
        0 => { g.cfg.muted = !g.cfg.muted; if let Some(s) = &mut g.session { s.set_muted(g.cfg.muted); } queue_outbound(protocol::UPDATE_STATE.to_string(), protocol::update_state(g.cfg.muted)); }
        1 => g.mute_all = !g.mute_all,
        2 => g.show_hud = !g.show_hud,
        n => { let rows = g.player_rows(Instant::now()); if let Some((_, uuid, _, _)) = rows.get(n - 3) { if !g.muted_players.remove(uuid) { g.muted_players.insert(*uuid); } } }
    }
    save_preferences(game, &g);
}
pub fn draw_hud(game: &mut crate::game::Game<'_>, batch: &mut horizon_client::hud::HudBatch, corners: &mut horizon_client::extras_hud::Corners) {
    use horizon_client::extras::Corner;
    if !game.native_on(ID) || !game.native_corner_shown(Corner::TopRight) { return; }
    let g = locked(); if !g.show_hud { return; }
    let mut lines = vec![format!("Voice: {}", g.connection_label()), if g.cfg.muted { "Microphone: MUTED".into() } else { "Microphone: READY".into() }];
    if g.ptt_held && !g.cfg.muted { lines.push("TRANSMITTING".into()); } if g.mute_all { lines.push("All players muted".into()); }
    for (name, _, _, speaking) in g.player_rows(Instant::now()) { if speaking { lines.push(format!("Speaking: {name}")); } }
    drop(g); let width = lines.iter().map(|line| game.native_text_width(line)).fold(0.0, f32::max) + 8.0; let height = lines.len() as f32 * 10.0 + 6.0;
    let (x, y) = corners.place(Corner::TopRight, width, height); game.native_fill(batch, [x, y, width, height], 0x78000000);
    for (i, line) in lines.iter().enumerate() { game.native_text(batch, line, [x + 4.0, y + 3.0 + i as f32 * 10.0], 0xFFFFFF, true); }
}
