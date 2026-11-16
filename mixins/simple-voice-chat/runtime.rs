use std::sync::Mutex;
use std::time::Instant;
use svc_client::protocol;
pub use svc_client::runtime::{state, Glue};

pub const ID: &str = "simple_voice_chat";
pub const NAME: &str = "Simple Voice Chat";
pub const VERSION: &str = "1.0.4";
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
    game.native_set_setting(ID, "show_debug", if g.show_debug { "true" } else { "false" });
    game.native_set_setting(ID, "hud_corner", &g.hud_corner.to_string());
    let mut ids: Vec<String> = g.muted_players.iter().map(u128::to_string).collect(); ids.sort();
    game.native_set_setting(ID, "muted_players", &ids.join(","));
}
pub fn load_preferences(game: &crate::game::Game<'_>) {
    let muted = game.native_setting(ID, "muted").as_deref() == Some("true");
    let mute_all = game.native_setting(ID, "mute_all").as_deref() == Some("true");
    let show_hud = game.native_setting(ID, "show_hud").as_deref() != Some("false");
    let show_debug = game.native_setting(ID, "show_debug").as_deref() == Some("true");
    let hud_corner = game.native_setting(ID, "hud_corner").and_then(|v| v.parse::<u8>().ok()).filter(|v| *v <= 3).unwrap_or(1);
    let muted_players = parse_muted_players(game.native_setting(ID, "muted_players"));
    let mut g = locked(); g.cfg.muted = muted; g.mute_all = mute_all; g.show_hud = show_hud; g.show_debug = show_debug; g.hud_corner = hud_corner; g.muted_players = muted_players;
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
pub fn on_net(channel: &str, data: &[u8]) {
    let mut g = locked();
    g.on_net(channel, data, None, Instant::now());
    if channel == protocol::SECRET {
        let muted = g.cfg.muted;
        if let Some(session) = &mut g.session { session.set_muted(muted); }
        let muted = g.cfg.muted;
        drop(g);
        queue_outbound(protocol::UPDATE_STATE.to_string(), protocol::update_state(muted));
    }
}
pub fn on_player_submit(_player: &crate::game::native_mob_draw::NativeMobRender<'_>, _who: &crate::game::native_mob_draw::NativePlayerRender<'_>) {}
pub fn on_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    let ptt = game.native_mod_key(ID, "ptt", "key.keyboard.v");
    let mute = game.native_mod_key(ID, "mute", "key.keyboard.m");
    let debug = game.native_mod_key(ID, "debug", "key.keyboard.f9");
    let mut g = locked();
    if pressed && key.eq_ignore_ascii_case(&debug) {
        g.show_debug = !g.show_debug;
        save_preferences(game, &g);
    }
    let before = g.cfg.muted;
    let feedback = g.on_key(key, pressed, &ptt, &mute);
    let after = g.cfg.muted;
    if before != after {
        queue_outbound(protocol::UPDATE_STATE.to_string(), protocol::update_state(after));
        save_preferences(game, &g);
    }
    drop(g);
    if let Some(line) = feedback { game.native_chat(&format!("[voice] {line}")); }
}
pub fn on_command(command: &str, ran: &mut bool) {
    let head = command.trim_start_matches('/').split_whitespace().next().unwrap_or(""); if head != "svc" && head != "vcgroup" { return; }
    let (feedback, outbound) = locked().on_command(command.trim_start_matches('/'));
    for (channel, body) in outbound { queue_outbound(channel, body); } if !feedback.is_empty() { eprintln!("[voice] {feedback}"); } *ran = true;
}
pub fn on_frame(game: &mut crate::game::Game<'_>, _elapsed: std::time::Duration) {
    let mut g = locked();
    let _ = g.on_frame(Instant::now());
    if g.reconnect_pending {
        g.reconnect_pending = false;
        g.session = None;
        g.connection_requested = true;
        let (channel, body) = Glue::handshake();
        drop(g);
        queue_outbound(channel, body);
    } else {
        drop(g);
    }
    let _ = game;
}
pub fn listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    mods.push(crate::game::native_hooks::NativeMod { id: ID.into(), title: format!("{NAME} {VERSION}"), description: DESCRIPTION.into(), keys: vec![
        ["ptt".into(), "Push to talk".into(), "key.keyboard.v".into()], ["mute".into(), "Mute microphone".into(), "key.keyboard.m".into()], ["debug".into(), "Toggle voice debug overlay".into(), "key.keyboard.f9".into()],
    ] });
}
fn tr(key: &str) -> &'static str {
    let code = horizon_client::lang::code();
    let es = code.starts_with("es");
    let pt = code.starts_with("pt");
    let fr = code.starts_with("fr");
    let de = code.starts_with("de");
    let it = code.starts_with("it");
    let ru = code.starts_with("ru");
    let zh = code.starts_with("zh");
    let ja = code.starts_with("ja");
    match key {
        "microphone" => if es { "Micrófono" } else if pt { "Microfone" } else if fr { "Micro" } else if de { "Mikrofon" } else if it { "Microfono" } else if ru { "Микрофон" } else if zh { "麦克风" } else if ja { "マイク" } else { "Microphone" },
        "mute_all" => if es { "Silenciar a todos" } else if pt { "Silenciar todos" } else if fr { "Couper tous les joueurs" } else if de { "Alle stummschalten" } else if it { "Silenzia tutti" } else if ru { "Отключить всех" } else if zh { "静音所有玩家" } else if ja { "全員をミュート" } else { "Mute everyone" },
        "hud" => if es { "HUD de voz" } else if pt { "HUD de voz" } else if fr { "HUD vocal" } else if de { "Sprach-HUD" } else if it { "HUD vocale" } else if ru { "Голосовой HUD" } else if zh { "语音 HUD" } else if ja { "ボイスHUD" } else { "Voice HUD" },
        "debug" => if es { "Diagnóstico de conexión" } else if pt { "Diagnóstico da conexão" } else if fr { "Diagnostic de connexion" } else if de { "Verbindungsdiagnose" } else if it { "Diagnostica connessione" } else if ru { "Диагностика соединения" } else if zh { "连接诊断" } else if ja { "接続診断" } else { "Connection debug" },
        "position" => if es { "Posición del HUD" } else if pt { "Posição do HUD" } else if fr { "Position du HUD" } else if de { "HUD-Position" } else if it { "Posizione HUD" } else if ru { "Положение HUD" } else if zh { "HUD位置" } else if ja { "HUDの位置" } else { "HUD position" },
        "audible" => if es { "SE OYE" } else if pt { "AUDÍVEL" } else if fr { "AUDIBLE" } else if de { "HÖRBAR" } else if it { "UDIBILE" } else if ru { "СЛЫШНО" } else if zh { "可听见" } else if ja { "聞こえる" } else { "AUDIBLE" },
        "players" => if es { "Jugadores (clic para silenciar)" } else if pt { "Jogadores (clique para silenciar)" } else if fr { "Joueurs (cliquer pour couper)" } else if de { "Spieler (zum Stummschalten klicken)" } else if it { "Giocatori (clicca per silenziare)" } else if ru { "Игроки (нажмите, чтобы заглушить)" } else if zh { "玩家（点击静音）" } else if ja { "プレイヤー（クリックでミュート）" } else { "Players (click to mute)" },
        "no_players" => if es { "No hay jugadores de voz detectados" } else if pt { "Nenhum jogador de voz detectado" } else if fr { "Aucun joueur vocal détecté" } else if de { "Keine Sprachspieler erkannt" } else if it { "Nessun giocatore vocale rilevato" } else if ru { "Голосовые игроки не обнаружены" } else if zh { "未检测到语音玩家" } else if ja { "ボイスプレイヤーが見つかりません" } else { "No voice players detected" },
        "cannot_block" => if es { "Bloquear mi voz por jugador requiere soporte del servidor" } else if pt { "Bloquear minha voz por jogador exige suporte do servidor" } else if fr { "Bloquer ma voix par joueur nécessite le serveur" } else if de { "Mein Mikrofon pro Spieler blockieren erfordert Serverunterstützung" } else if it { "Bloccare la mia voce per giocatore richiede il server" } else if ru { "Блокировка моего голоса для игрока требует поддержки сервера" } else if zh { "按玩家屏蔽我的语音需要服务器支持" } else if ja { "プレイヤー別の送信ミュートにはサーバー対応が必要です" } else { "Per-player outgoing mute requires server support" },
        "voice" => if es { "Voz" } else if pt { "Voz" } else if fr { "Voix" } else if de { "Stimme" } else if it { "Voce" } else if ru { "Голос" } else if zh { "语音" } else if ja { "ボイス" } else { "Voice" },
        "ready" => if es { "LISTO" } else if pt { "PRONTO" } else if fr { "PRÊT" } else if de { "BEREIT" } else if it { "PRONTO" } else if ru { "ГОТОВО" } else if zh { "就绪" } else if ja { "準備完了" } else { "READY" },
        "muted" => if es { "SILENCIADO" } else if pt { "MUTADO" } else if fr { "MUET" } else if de { "STUMM" } else if it { "MUTO" } else if ru { "ЗАГЛУШЕНО" } else if zh { "已静音" } else if ja { "ミュート中" } else { "MUTED" },
        "all_muted" => if es { "Todos silenciados" } else if pt { "Todos silenciados" } else if fr { "Tous en sourdine" } else if de { "Alle stumm" } else if it { "Tutti silenziati" } else if ru { "Все отключены" } else if zh { "所有人已静音" } else if ja { "全員ミュート中" } else { "All players muted" },
        "speaking" => if es { "Hablando" } else if pt { "Falando" } else if fr { "Parle" } else if de { "Spricht" } else if it { "Sta parlando" } else if ru { "Говорит" } else if zh { "正在说话" } else if ja { "発話中" } else { "Speaking" },
        "press" => if es { "Pulsa" } else if pt { "Pressione" } else if fr { "Appuyez sur" } else if de { "Drücke" } else if it { "Premi" } else if ru { "Нажмите" } else if zh { "按下" } else if ja { "押して" } else { "Press" },
        "to_talk" => if es { "para hablar" } else if pt { "para falar" } else if fr { "pour parler" } else if de { "zum Sprechen" } else if it { "per parlare" } else if ru { "чтобы говорить" } else if zh { "开始说话" } else if ja { "で話す" } else { "to talk" },
        "transmitting" => if es { "TRANSMITIENDO" } else if pt { "TRANSMITINDO" } else if fr { "ÉMISSION" } else if de { "SENDEN" } else if it { "TRASMISSIONE" } else if ru { "ПЕРЕДАЧА" } else if zh { "正在传输" } else if ja { "送信中" } else { "TRANSMITTING" },
        "connected" => if es { "CONECTADO" } else if pt { "CONECTADO" } else if fr { "CONNECTÉ" } else if de { "VERBUNDEN" } else if it { "CONNESSO" } else if ru { "ПОДКЛЮЧЕНО" } else if zh { "已连接" } else if ja { "接続済み" } else { "CONNECTED" },
        "connecting" => if es { "CONECTANDO" } else if pt { "CONECTANDO" } else if fr { "CONNEXION" } else if de { "VERBINDEN" } else if it { "CONNESSIONE" } else if ru { "ПОДКЛЮЧЕНИЕ" } else if zh { "正在连接" } else if ja { "接続中" } else { "CONNECTING" },
        "disconnected" => if es { "DESCONECTADO" } else if pt { "DESCONECTADO" } else if fr { "DÉCONNECTÉ" } else if de { "GETRENNT" } else if it { "DISCONNESSO" } else if ru { "НЕ ПОДКЛЮЧЕНО" } else if zh { "未连接" } else if ja { "未接続" } else { "DISCONNECTED" },
        "reconnecting" => if es { "RECONECTANDO" } else if pt { "RECONECTANDO" } else if fr { "RECONNEXION" } else if de { "NEU VERBINDEN" } else if it { "RICONNESSIONE" } else if ru { "ПЕРЕПОДКЛЮЧЕНИЕ" } else if zh { "正在重新连接" } else if ja { "再接続中" } else { "RECONNECTING" },
        "udp" => "UDP",
        "waiting_secret" => if es { "Esperando respuesta SECRET" } else if pt { "Aguardando resposta SECRET" } else if fr { "En attente de SECRET" } else if de { "Warte auf SECRET-Antwort" } else if it { "In attesa della risposta SECRET" } else if ru { "Ожидание ответа SECRET" } else if zh { "等待 SECRET 响应" } else if ja { "SECRET応答待ち" } else { "Waiting for SECRET reply" },
        "auth_pending" => if es { "Autenticación UDP pendiente" } else if pt { "Autenticação UDP pendente" } else if fr { "Authentification UDP en attente" } else if de { "UDP-Authentifizierung ausstehend" } else if it { "Autenticazione UDP in attesa" } else if ru { "Ожидание UDP-аутентификации" } else if zh { "等待 UDP 身份验证" } else if ja { "UDP認証待ち" } else { "UDP authentication pending" },
        "authenticated" => if es { "UDP autenticado" } else if pt { "UDP autenticado" } else if fr { "UDP authentifié" } else if de { "UDP authentifiziert" } else if it { "UDP autenticato" } else if ru { "UDP аутентифицирован" } else if zh { "UDP 已验证" } else if ja { "UDP認証済み" } else { "UDP authenticated" },
        "no_session" => if es { "Sin sesión" } else if pt { "Sem sessão" } else if fr { "Aucune session" } else if de { "Keine Sitzung" } else if it { "Nessuna sessione" } else if ru { "Нет сеанса" } else if zh { "无会话" } else if ja { "セッションなし" } else { "No session" },
        "server" => if es { "Servidor" } else if pt { "Servidor" } else if fr { "Serveur" } else if de { "Server" } else if it { "Server" } else if ru { "Сервер" } else if zh { "服务器" } else if ja { "サーバー" } else { "Server" },
        "mic_input" => if es { "Entrada de micrófono" } else if pt { "Entrada do microfone" } else if fr { "Entrée micro" } else if de { "Mikrofoneingang" } else if it { "Ingresso microfono" } else if ru { "Вход микрофона" } else if zh { "麦克风输入" } else if ja { "マイク入力" } else { "Microphone input" },
        "audio_output" => if es { "Salida de audio" } else if pt { "Saída de áudio" } else if fr { "Sortie audio" } else if de { "Audioausgabe" } else if it { "Uscita audio" } else if ru { "Аудиовыход" } else if zh { "音频输出" } else if ja { "音声出力" } else { "Audio output" },
        "codec" => if es { "Códec Opus" } else if pt { "Codec Opus" } else if fr { "Codec Opus" } else if de { "Opus-Codec" } else if it { "Codec Opus" } else if ru { "Кодек Opus" } else if zh { "Opus 编码器" } else if ja { "Opusコーデック" } else { "Opus codec" },
        "on" => if es { "ACTIVADO" } else if pt { "ATIVADO" } else if fr { "ACTIVÉ" } else if de { "EIN" } else if it { "ATTIVO" } else if ru { "ВКЛ" } else if zh { "开启" } else if ja { "オン" } else { "ON" },
        "off" => if es { "DESACTIVADO" } else if pt { "DESATIVADO" } else if fr { "DÉSACTIVÉ" } else if de { "AUS" } else if it { "DISATTIVO" } else if ru { "ВЫКЛ" } else if zh { "关闭" } else if ja { "オフ" } else { "OFF" },
        "top_left" => if es { "Superior izquierda" } else if pt { "Superior esquerda" } else if fr { "Haut gauche" } else if de { "Oben links" } else if it { "In alto a sinistra" } else if ru { "Сверху слева" } else if zh { "左上" } else if ja { "左上" } else { "Top left" },
        "top_right" => if es { "Superior derecha" } else if pt { "Superior direita" } else if fr { "Haut droite" } else if de { "Oben rechts" } else if it { "In alto a destra" } else if ru { "Сверху справа" } else if zh { "右上" } else if ja { "右上" } else { "Top right" },
        "bottom_left" => if es { "Inferior izquierda" } else if pt { "Inferior esquerda" } else if fr { "Bas gauche" } else if de { "Unten links" } else if it { "In basso a sinistra" } else if ru { "Снизу слева" } else if zh { "左下" } else if ja { "左下" } else { "Bottom left" },
        "bottom_right" => if es { "Inferior derecha" } else if pt { "Inferior direita" } else if fr { "Bas droite" } else if de { "Unten rechts" } else if it { "In basso a destra" } else if ru { "Снизу справа" } else if zh { "右下" } else if ja { "右下" } else { "Bottom right" },
        _ => "Unknown",
    }
}

fn connection_label(g: &Glue) -> &'static str {
    match g.session.as_ref().map(|session| session.state()) {
        None if g.connection_requested => tr("connecting"),
        None => tr("disconnected"),
        Some(svc_client::session::State::Idle) if g.reconnect_pending => tr("reconnecting"),
        Some(svc_client::session::State::Idle) => tr("disconnected"),
        Some(svc_client::session::State::RequestingSecret) | Some(svc_client::session::State::Connecting) => tr("connecting"),
        Some(svc_client::session::State::Connected) => tr("connected"),
    }
}

fn key_label(key: &str) -> String {
    let value = key.strip_prefix("key.keyboard.").unwrap_or(key);
    let value = value.strip_prefix("Key").unwrap_or(value);
    value.to_uppercase()
}

fn hud_corner(value: u8) -> horizon_client::extras::Corner {
    use horizon_client::extras::Corner;
    match value {
        0 => Corner::TopLeft,
        1 => Corner::TopRight,
        2 => Corner::BottomLeft,
        _ => Corner::BottomRight,
    }
}

pub fn settings_page(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    if mod_id != ID { return; }
    load_preferences(game);
    let g = locked();
    buttons.push(format!("{}: {}", tr("microphone"), if g.cfg.muted { tr("muted") } else { tr("ready") }));
    buttons.push(format!("{}: {}", tr("mute_all"), if g.mute_all { tr("on") } else { tr("off") }));
    buttons.push(format!("{}: {}", tr("hud"), if g.show_hud { tr("on") } else { tr("off") }));
    buttons.push(format!("{}: {}", tr("debug"), if g.show_debug { tr("on") } else { tr("off") }));
    let corner = match g.hud_corner { 0 => tr("top_left"), 1 => tr("top_right"), 2 => tr("bottom_left"), _ => tr("bottom_right") };
    buttons.push(format!("{}: {}", tr("position"), corner));
    buttons.push(tr("players").into());
    let rows = g.player_rows(Instant::now());
    for (name, _, muted, _) in &rows {
        buttons.push(format!("{}: {}", name, if *muted { tr("muted") } else { tr("audible") }));
    }
    if rows.is_empty() { buttons.push(tr("no_players").into()); }
    buttons.push(tr("cannot_block").into());
}

pub fn settings_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    if mod_id != ID { return; }
    load_preferences(game);
    let mut g = locked();
    match button {
        0 => { g.cfg.muted = !g.cfg.muted; let muted = g.cfg.muted; if let Some(s) = &mut g.session { s.set_muted(muted); } queue_outbound(protocol::UPDATE_STATE.to_string(), protocol::update_state(muted)); }
        1 => g.mute_all = !g.mute_all,
        2 => g.show_hud = !g.show_hud,
        3 => g.show_debug = !g.show_debug,
        4 => g.hud_corner = (g.hud_corner + 1) % 4,
        5 => {}
        n => {
            let rows = g.player_rows(Instant::now());
            if let Some((_, uuid, _, _)) = rows.get(n - 6) {
                if !g.muted_players.remove(uuid) { g.muted_players.insert(*uuid); }
            }
        }
    }
    save_preferences(game, &g);
}

pub fn draw_hud(game: &mut crate::game::Game<'_>, batch: &mut horizon_client::hud::HudBatch, corners: &mut horizon_client::extras_hud::Corners) {
    if !game.native_on(ID) { return; }
    let ptt = game.native_mod_key(ID, "ptt", "key.keyboard.v");
    let mut g = locked();
    if !g.show_hud && !g.show_debug { return; }
    let corner = hud_corner(g.hud_corner);
    if !game.native_corner_shown(corner) { return; }
    let mut lines = Vec::new();
    if g.show_hud {
        lines.push(format!("{}: {}", tr("voice"), connection_label(&g)));
        lines.push(format!("{}: {}", tr("microphone"), if g.cfg.muted { tr("muted") } else { tr("ready") }));
        if g.ptt_held && !g.cfg.muted { lines.push(tr("transmitting").into()); }
        else if !g.cfg.muted { lines.push(format!("{} [{}] {}", tr("press"), key_label(&ptt), tr("to_talk"))); }
        if g.mute_all { lines.push(tr("all_muted").into()); }
        for (name, _, _, speaking) in g.player_rows(Instant::now()) {
            if speaking { lines.push(format!("{}: {name}", tr("speaking"))); }
        }
    }
    if g.show_debug {
        lines.push(format!("{} — {}", tr("debug"), connection_label(&g)));
        let udp = match g.session.as_ref().map(|session| session.state()) {
            None if g.connection_requested => tr("waiting_secret"),
            None => tr("no_session"),
            Some(svc_client::session::State::Idle) if g.reconnect_pending => tr("reconnecting"),
            Some(svc_client::session::State::Idle) => tr("disconnected"),
            Some(svc_client::session::State::RequestingSecret) => tr("waiting_secret"),
            Some(svc_client::session::State::Connecting) => tr("auth_pending"),
            Some(svc_client::session::State::Connected) => tr("authenticated"),
        };
        lines.push(format!("{}: {}", tr("udp"), udp));
        lines.push(format!("{}: {}", tr("server"), g.server_addr.map(|addr| addr.to_string()).unwrap_or_else(|| "—".into())));
        lines.push(format!("{}: {}", tr("mic_input"), if g.mic.is_some() { tr("ready") } else { tr("off") }));
        lines.push(format!("{}: {}", tr("audio_output"), if g.speaker.is_some() { tr("ready") } else { tr("off") }));
        lines.push(format!("{}: {}", tr("codec"), if g.codec.is_some() { tr("ready") } else { tr("off") }));
    }
    drop(g);
    let width = lines.iter().map(|line| game.native_text_width(line)).fold(0.0, f32::max) + 8.0;
    let height = lines.len() as f32 * 10.0 + 6.0;
    let (x, y) = corners.place(corner, width, height);
    game.native_fill(batch, [x, y, width, height], 0x78000000);
    for (i, line) in lines.iter().enumerate() { game.native_text(batch, line, [x + 4.0, y + 3.0 + i as f32 * 10.0], 0xFFFFFF, true); }
}
