use mod_api::{
    self, ClientHost, ClientPacketEvent, Hooks, Identifier, InputKeyEvent, KeyAction,
    ServerboundPlayPacket, Text,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

static IN_WORLD: AtomicBool = AtomicBool::new(false);

#[derive(Default)]
struct SimpleVoiceChat;

#[mod_api::horizon_mod]
impl Hooks for SimpleVoiceChat {
    fn client_packet(&self, host: &mut dyn ClientHost, event: &mut ClientPacketEvent<'_>) {
        let Some(payload) = &event.packet.custom_payload else {
            return;
        };
        if !payload.channel.starts_with("voicechat:") {
            return;
        }

        let address = host
            .client_session()
            .client_server()
            .and_then(|server| server.address);
        let hint = address.as_deref().and_then(server_host);

        if let Ok(mut state) = svc_client::runtime::state().lock() {
            state.on_net(&payload.channel, &payload.data, hint.as_deref(), Instant::now());
        }
    }

    fn input_key(&self, _host: &mut dyn ClientHost, event: &mut InputKeyEvent<'_>) {
        let pressed = matches!(event.action, KeyAction::Press | KeyAction::Repeat);
        if let Ok(mut state) = svc_client::runtime::state().lock() {
            let note = state.on_key(event.key, pressed);
            if let Some(note) = note {
                state.notes.push(format!("[voice] {note}"));
            }
        }
    }

    fn client_tick_post(&self, host: &mut dyn ClientHost) {
        let playing = host.playing();
        let was_playing = IN_WORLD.swap(playing, Ordering::AcqRel);

        if playing && !was_playing {
            if let Ok(mut state) = svc_client::runtime::state().lock() {
                state.world_open();
                let (channel, data) = svc_client::runtime::Glue::handshake();
                state.pending_sends.push((channel, data));
                let audio_notes = state.start_audio();
                state.notes.extend(audio_notes);
            }
        } else if !playing && was_playing {
            if let Ok(mut state) = svc_client::runtime::state().lock() {
                state.world_close();
            }
        }

        if !playing {
            return;
        }

        let (sends, notes) = if let Ok(mut state) = svc_client::runtime::state().lock() {
            let _ = state.on_frame(Instant::now());
            (
                std::mem::take(&mut state.pending_sends),
                std::mem::take(&mut state.notes),
            )
        } else {
            return;
        };

        for (channel, data) in sends {
            if let Err(error) = send_custom_payload(host, &channel, &data) {
                let _ = host.chat_line(Text::literal(format!(
                    "[voice] no se pudo enviar {channel}: {error:?}"
                )));
            }
        }

        for note in notes {
            let _ = host.chat_line(Text::literal(note));
        }
    }
}

fn send_custom_payload(
    host: &mut dyn ClientHost,
    channel: &str,
    data: &[u8],
) -> Result<(), String> {
    let _ = Identifier::parse(channel).map_err(|error| format!("{error:?}"))?;
    let mut body = Vec::with_capacity(channel.len() + data.len() + 5);
    write_varint(channel.len() as u32, &mut body);
    body.extend_from_slice(channel.as_bytes());
    body.extend_from_slice(data);

    let packet = ServerboundPlayPacket::new(
        Identifier::parse("minecraft:custom_payload").expect("valid packet identifier"),
        body,
    );
    host.send_packet(packet).map_err(|error| format!("{error:?}"))?;
    Ok(())
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
    if host.is_empty() { None } else { Some(host.to_string()) }
}
