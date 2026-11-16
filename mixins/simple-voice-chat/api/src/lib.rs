#[path = "../../crates/svc_client/src/audio.rs"]
pub mod audio;
#[path = "../../crates/svc_client/src/audio_io.rs"]
pub mod audio_io;
#[path = "../../crates/svc_client/src/opus_ffi.rs"]
pub mod opus_ffi;
#[path = "../../crates/svc_client/src/protocol.rs"]
pub mod protocol;
#[path = "../../crates/svc_client/src/runtime.rs"]
pub mod runtime;
#[path = "../../crates/svc_client/src/session.rs"]
pub mod session;

use horizon_client::mod_api::{
    ClientPacketEvent, ClientboundPlayPacket, Hooks, Host, KeyEvent, ServerboundPlayPacket, Text, TickEvent,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

static IN_WORLD: AtomicBool = AtomicBool::new(false);

struct SimpleVoiceChat;

impl Hooks for SimpleVoiceChat {
    fn client_packet(&self, host: &mut dyn Host, event: &mut ClientPacketEvent<'_>) {
        if let ClientboundPlayPacket::CustomPayload(payload) = event.packet {
            if !payload.channel.starts_with("voicechat:") {
                return;
            }
            let address = host.client_state().and_then(|state| state.server_address);
            let hint = address.as_deref().and_then(server_host);
            if let Ok(mut state) = runtime::state().lock() {
                state.on_net(
                    &payload.channel,
                    &payload.data,
                    hint.as_deref(),
                    Instant::now(),
                );
            }
        }
    }

    fn key(&self, host: &mut dyn Host, event: &mut KeyEvent<'_>) {
        if host.client_state().is_none() {
            return;
        }
        if let Ok(mut state) = runtime::state().lock() {
            if let Some(note) = state.on_key(event.key, event.pressed) {
                host.client_action_bar(Text::literal(format!("[voice] {note}")), true);
            }
        }
    }

    fn tick(&self, host: &mut dyn Host, _event: &mut TickEvent) {
        let Some(client) = host.client_state() else {
            return;
        };
        let playing = client.logged_in && client.world.is_some();
        let was_playing = IN_WORLD.swap(playing, Ordering::AcqRel);

        if playing && !was_playing {
            if let Ok(mut state) = runtime::state().lock() {
                state.world_open();
                let audio_notes = state.start_audio();
                state.notes.extend(audio_notes);
                let (channel, data) = runtime::Glue::handshake();
                state.pending_sends.push((channel, data));
            }
        } else if !playing && was_playing {
            if let Ok(mut state) = runtime::state().lock() {
                state.world_close();
            }
        }

        if !playing {
            return;
        }

        let (sends, notes) = if let Ok(mut state) = runtime::state().lock() {
            let _ = state.on_frame(Instant::now());
            if state.reconnect_pending {
                state.reconnect_pending = false;
                if let Some(mut session) = state.session.take() {
                    session.close();
                }
                let (channel, data) = runtime::Glue::handshake();
                state.pending_sends.push((channel, data));
            }
            (
                std::mem::take(&mut state.pending_sends),
                std::mem::take(&mut state.notes),
            )
        } else {
            return;
        };

        for (channel, data) in sends {
            if let Err(error) = send_custom_payload(host, &channel, &data) {
                host.client_action_bar(
                    Text::literal(format!("[voice] no se pudo enviar {channel}: {error}")),
                    true,
                );
            }
        }

        if !notes.is_empty() {
            host.client_action_bar(Text::literal(notes.join(" · ")), true);
        }
    }
}

fn send_custom_payload(
    host: &mut dyn Host,
    channel: &str,
    data: &[u8],
) -> Result<(), String> {
    const SERVERBOUND_CUSTOM_PAYLOAD_ID: i32 = 22;
    let mut body = Vec::with_capacity(channel.len() + data.len() + 5);
    write_varint(channel.len() as u32, &mut body);
    body.extend_from_slice(channel.as_bytes());
    body.extend_from_slice(data);
    let packet = ServerboundPlayPacket::decode(SERVERBOUND_CUSTOM_PAYLOAD_ID, &body)
        .map_err(|error| format!("{error:?}"))?;
    host.send_packet(packet)
}

fn write_varint(mut value: u32, output: &mut Vec<u8>) {
    loop {
        if value & !0x7f == 0 {
            output.push(value as u8);
            return;
        }
        output.push(((value & 0x7f) | 0x80) as u8);
        value >>= 7;
    }
}

fn server_host(address: &str) -> Option<String> {
    if let Ok(socket) = address.parse::<std::net::SocketAddr>() {
        return Some(socket.ip().to_string());
    }
    let address = address.strip_prefix('[').unwrap_or(address);
    let address = address.split(']').next().unwrap_or(address);
    let host = address.rsplit_once(':').map(|(host, _)| host).unwrap_or(address);
    if host.is_empty() {
        None
    } else {
        Some(host.to_string())
    }
}

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(SimpleVoiceChat)
}
