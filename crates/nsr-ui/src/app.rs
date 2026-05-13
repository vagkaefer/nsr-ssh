use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};
use egui::{Key, Modifiers, Pos2, Rect, RichText, Stroke, Vec2};
use egui::epaint::CornerRadius;
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

use nsr_core::{SessionEvent, SessionManager};
use nsr_theme::{Theme, load_user_themes};
use nsr_vault::{Host, VaultStore};

#[derive(Debug, Clone, PartialEq)]
enum PaneState {
    Connecting,
    Connected,
    Disconnected { error: Option<String> },
}

use crate::connect_dialog::ConnectDialog;
use crate::design::Ds;
use crate::pane::{PaneTree, Tab};
use crate::settings::SettingsPanel;
use crate::tab_bar::{TabBar, TabBarAction};
use crate::terminal_widget::{TerminalBuffer, TerminalWidgetResult};
use crate::vault_panel::{VaultAction, VaultPanel};

#[derive(Default)]
struct PaneResult {
    new_active: Option<Uuid>,
    copy_text: Option<String>,
    paste_requested: Option<Uuid>,
    save_content: Option<Uuid>,
    toggle_recording: Option<Uuid>,
    reconnect: Option<Uuid>,
    close_pane: Option<Uuid>,
    split_h: Option<Uuid>,
    split_v: Option<Uuid>,
}

pub struct NsrApp {
    tabs: Vec<Tab>,
    active_tab: Option<Uuid>,
    hosts: Vec<Host>,
    theme: Theme,

    terminal_buffers: HashMap<Uuid, Arc<Mutex<TerminalBuffer>>>,
    output_receivers: HashMap<Uuid, mpsc::Receiver<Vec<u8>>>,
    pane_states: HashMap<Uuid, PaneState>,
    session_connected_at: HashMap<Uuid, Instant>,

    session_manager: Arc<SessionManager>,
    event_rx: broadcast::Receiver<SessionEvent>,
    vault_store: Option<VaultStore>,

    vault_panel: VaultPanel,
    connect_dialog: ConnectDialog,
    settings: SettingsPanel,

    rt: Arc<tokio::runtime::Runtime>,
    show_vault: bool,
    welcome_search: String,
    ssh_config_mtime: Option<SystemTime>,
    dragging_tab: Option<Uuid>,      // tab_id sendo arrastado
    status_message: String,
    status_ok: bool,
}

impl NsrApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let rt = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("Falha ao criar runtime Tokio"),
        );

        let (session_manager, event_rx) = rt.block_on(async { SessionManager::new() });
        let session_manager = Arc::new(session_manager);

        let vault_store = VaultStore::new().ok();
        let mut hosts = vault_store
            .as_ref()
            .and_then(|s| s.load_hosts().ok())
            .unwrap_or_default();

        // Merge hosts do ~/.ssh/config que ainda não estão no vault
        if let Some(ref store) = vault_store {
            if let Ok(new_from_config) = store.new_hosts_from_ssh_config(&hosts) {
                if !new_from_config.is_empty() {
                    hosts.extend(new_from_config);
                    let _ = store.save_hosts(&hosts);
                }
            }
        }
        // Lê mtime APÓS o possível save inicial para não disparar check no primeiro frame
        let initial_mtime = vault_store.as_ref().and_then(|s| s.ssh_config_mtime());

        let themes = load_user_themes();
        let theme = themes.into_iter().next().unwrap_or_else(nsr_theme::builtin::dracula);
        let theme_name = theme.name.clone();

        Self {
            tabs: Vec::new(),
            active_tab: None,
            hosts,
            theme: theme.clone(),
            terminal_buffers: HashMap::new(),
            output_receivers: HashMap::new(),
            pane_states: HashMap::new(),
            session_connected_at: HashMap::new(),
            session_manager,
            event_rx,
            ssh_config_mtime: initial_mtime,
            vault_store,
            vault_panel: VaultPanel::new(),
            connect_dialog: ConnectDialog::new(),
            settings: SettingsPanel::new(&theme_name),
            rt,
            show_vault: true,
            welcome_search: String::new(),
            dragging_tab: None,
            status_message: "Pronto".into(),
            status_ok: true,
        }
    }

    fn connect_to_host(&mut self, host: Host) {
        self.connect_to_host_with_password(host, None);
    }

    fn connect_to_host_with_password(&mut self, host: Host, password: Option<String>) {
        let sm = self.session_manager.clone();
        let host_clone = host.clone();
        let rt = self.rt.clone();

        let scrollback = self.settings.scrollback_lines;
        match rt.block_on(async { sm.connect(host_clone, password).await }) {
            Ok((session_id, output_rx)) => {
                self.terminal_buffers.insert(
                    session_id,
                    Arc::new(Mutex::new(TerminalBuffer::new(session_id, 220, 50, scrollback))),
                );
                self.output_receivers.insert(session_id, output_rx);
                self.pane_states.insert(session_id, PaneState::Connecting);
                let tab = Tab::new(host.alias.clone(), &host.alias, session_id);
                let tab_id = tab.id;
                self.tabs.push(tab);
                self.active_tab = Some(tab_id);
                self.status_message = format!("Conectando em {}...", host.alias);
                self.status_ok = true;
            }
            Err(e) => {
                self.status_message = format!("Erro: {}", e);
                self.status_ok = false;
            }
        }
    }

    fn close_tab(&mut self, tab_id: Uuid) {
        if let Some(pos) = self.tabs.iter().position(|t| t.id == tab_id) {
            let tab = self.tabs.remove(pos);
            for sid in tab.pane_tree.sessions() {
                self.terminal_buffers.remove(&sid);
                self.output_receivers.remove(&sid);
                let sm = self.session_manager.clone();
                let rt = self.rt.clone();
                rt.block_on(async { sm.disconnect(sid).await });
            }
            self.active_tab = self.tabs.last().map(|t| t.id);
        }
    }

    fn duplicate_tab(&mut self, tab_id: Uuid) {
        let host_alias = self.tabs.iter()
            .find(|t| t.id == tab_id)
            .map(|t| t.host_alias.clone());
        if let Some(alias) = host_alias {
            if let Some(host) = self.hosts.iter().find(|h| h.alias == alias).cloned() {
                self.connect_to_host(host);
            }
        }
    }

    fn split_active_h(&mut self) {
        if let Some(tab_id) = self.active_tab {
            if let Some(idx) = self.tabs.iter().position(|t| t.id == tab_id) {
                let active_pane = self.tabs[idx].active_pane;
                // Clona a sessão atual para o novo pane
                let host_alias = self.tabs[idx].host_alias.clone();
                if let Some(host) = self.hosts.iter().find(|h| h.alias == host_alias).cloned() {
                    let sm = self.session_manager.clone();
                    let rt = self.rt.clone();
                    let scrollback = self.settings.scrollback_lines;
                    if let Ok((new_sid, rx)) = rt.block_on(async { sm.connect(host, None).await }) {
                        self.terminal_buffers.insert(
                            new_sid,
                            Arc::new(Mutex::new(TerminalBuffer::new(new_sid, 110, 50, scrollback))),
                        );
                        self.output_receivers.insert(new_sid, rx);
                        let tree = std::mem::replace(
                            &mut self.tabs[idx].pane_tree,
                            PaneTree::Terminal(active_pane),
                        );
                        self.tabs[idx].pane_tree = tree.split_h_at(active_pane, new_sid);
                        self.tabs[idx].active_pane = new_sid;
                    }
                }
            }
        }
    }

    fn split_active_v(&mut self) {
        if let Some(tab_id) = self.active_tab {
            if let Some(idx) = self.tabs.iter().position(|t| t.id == tab_id) {
                let active_pane = self.tabs[idx].active_pane;
                let host_alias = self.tabs[idx].host_alias.clone();
                if let Some(host) = self.hosts.iter().find(|h| h.alias == host_alias).cloned() {
                    let sm = self.session_manager.clone();
                    let rt = self.rt.clone();
                    let scrollback = self.settings.scrollback_lines;
                    if let Ok((new_sid, rx)) = rt.block_on(async { sm.connect(host, None).await }) {
                        self.terminal_buffers.insert(
                            new_sid,
                            Arc::new(Mutex::new(TerminalBuffer::new(new_sid, 220, 25, scrollback))),
                        );
                        self.output_receivers.insert(new_sid, rx);
                        let tree = std::mem::replace(
                            &mut self.tabs[idx].pane_tree,
                            PaneTree::Terminal(active_pane),
                        );
                        self.tabs[idx].pane_tree = tree.split_v_at(active_pane, new_sid);
                        self.tabs[idx].active_pane = new_sid;
                    }
                }
            }
        }
    }

    fn close_active_pane(&mut self) {
        if let Some(tab_id) = self.active_tab {
            if let Some(idx) = self.tabs.iter().position(|t| t.id == tab_id) {
                let active_pane = self.tabs[idx].active_pane;
                let sessions_before = self.tabs[idx].pane_tree.sessions();

                // Se só tem um pane, fecha a aba inteira
                if sessions_before.len() == 1 {
                    self.close_tab(tab_id);
                    return;
                }

                let tree = std::mem::replace(
                    &mut self.tabs[idx].pane_tree,
                    PaneTree::Terminal(active_pane),
                );
                if let Some(new_tree) = tree.close_pane(active_pane) {
                    self.tabs[idx].pane_tree = new_tree;
                    // Ativa o primeiro pane restante
                    let remaining = self.tabs[idx].pane_tree.sessions();
                    if let Some(&first) = remaining.first() {
                        self.tabs[idx].active_pane = first;
                    }
                } else {
                    self.tabs[idx].pane_tree = PaneTree::Terminal(active_pane);
                    self.close_tab(tab_id);
                    return;
                }

                // Desconecta a sessão fechada
                self.terminal_buffers.remove(&active_pane);
                self.output_receivers.remove(&active_pane);
                let sm = self.session_manager.clone();
                let rt = self.rt.clone();
                rt.block_on(async { sm.disconnect(active_pane).await });
            }
        }
    }

    fn check_ssh_config_changes(&mut self) {
        let Some(ref store) = self.vault_store else { return };
        let current_mtime = store.ssh_config_mtime();
        if current_mtime == self.ssh_config_mtime {
            return;
        }
        self.ssh_config_mtime = current_mtime;

        match store.new_hosts_from_ssh_config(&self.hosts) {
            Ok(new_hosts) if !new_hosts.is_empty() => {
                let count = new_hosts.len();
                self.hosts.extend(new_hosts);
                self.persist_hosts();
                self.status_message = format!("{} novo(s) host(s) importado(s) do ~/.ssh/config", count);
                self.status_ok = true;
            }
            Ok(_) => {
                // arquivo mudou mas sem novos hosts (edição de entradas existentes)
                // reimporta atualizações de hosts existentes pelo alias
                if let Ok(imported) = store.import_from_ssh_config() {
                    for imported_host in imported {
                        if let Some(existing) = self.hosts.iter_mut().find(|h| h.alias == imported_host.alias) {
                            existing.hostname = imported_host.hostname;
                            existing.user = imported_host.user;
                            existing.port = imported_host.port;
                            if imported_host.identity_file.is_some() {
                                existing.identity_file = imported_host.identity_file;
                            }
                        }
                    }
                    self.persist_hosts();
                }
            }
            Err(_) => {}
        }
    }

    fn drain_output_receivers(&mut self) {
        let ids: Vec<Uuid> = self.output_receivers.keys().copied().collect();
        for sid in ids {
            if let Some(rx) = self.output_receivers.get_mut(&sid) {
                loop {
                    match rx.try_recv() {
                        Ok(data) => {
                            if let Some(buf) = self.terminal_buffers.get(&sid) {
                                if let Ok(mut b) = buf.lock() { b.process(&data); }
                            }
                        }
                        Err(mpsc::error::TryRecvError::Empty) => break,
                        Err(mpsc::error::TryRecvError::Disconnected) => break,
                    }
                }
            }
        }
    }

    fn process_session_events(&mut self) {
        loop {
            match self.event_rx.try_recv() {
                Ok(SessionEvent::Connected { session_id }) => {
                    self.pane_states.insert(session_id, PaneState::Connected);
                    self.session_connected_at.insert(session_id, Instant::now());
                    self.status_message = "Conectado".into();
                    self.status_ok = true;
                }
                Ok(SessionEvent::Disconnected { session_id }) => {
                    let entry = self.pane_states.entry(session_id).or_insert(PaneState::Connecting);
                    if matches!(entry, PaneState::Connected | PaneState::Connecting) {
                        *entry = PaneState::Disconnected { error: None };
                    }
                    self.session_connected_at.remove(&session_id);
                    self.status_message = "Desconectado".into();
                    self.status_ok = false;
                }
                Ok(SessionEvent::Error { session_id, message }) => {
                    self.pane_states.insert(session_id, PaneState::Disconnected { error: Some(message.clone()) });
                    self.status_message = message;
                    self.status_ok = false;
                }
                Ok(SessionEvent::Output { session_id, data }) => {
                    if let Some(buf) = self.terminal_buffers.get(&session_id) {
                        if let Ok(mut b) = buf.lock() { b.process(&data); }
                    }
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
    }

    fn render_pane_tree(
        ui: &mut egui::Ui,
        pane: &mut PaneTree,
        active_pane: Uuid,
        terminal_buffers: &mut HashMap<Uuid, Arc<Mutex<TerminalBuffer>>>,
        pane_states: &HashMap<Uuid, PaneState>,
        font_size: f32,
        session_manager: &Arc<SessionManager>,
        rt: &Arc<tokio::runtime::Runtime>,
    ) -> PaneResult {
        const SEP: f32 = 4.0;

        match pane {
            PaneTree::Terminal(sid) => {
                let sid = *sid;
                let mut result = PaneResult::default();
                let state = pane_states.get(&sid).cloned().unwrap_or(PaneState::Connecting);

                match state {
                    PaneState::Disconnected { error } => {
                        // Tela de desconexão
                        Self::render_disconnected(ui, sid, error.as_deref(), &mut result);
                    }
                    PaneState::Connecting => {
                        // Tela de aguardando conexão
                        Self::render_connecting(ui);
                    }
                    PaneState::Connected => {
                        // Terminal normal
                        if let Some(buf_arc) = terminal_buffers.get(&sid) {
                            if let Ok(mut buf) = buf_arc.lock() {
                                let term_theme = nsr_theme::builtin::dracula();
                                let is_active = sid == active_pane;
                                let mut widget = crate::terminal_widget::TerminalWidget::new(
                                    &mut buf,
                                    &term_theme,
                                )
                                .font_size(font_size)
                                .active(is_active);

                                let TerminalWidgetResult {
                                    response: resp,
                                    resize,
                                    copy_text,
                                    paste_requested,
                                    save_content_requested,
                                    toggle_recording,
                                } = widget.show(ui);

                                if resp.clicked() { result.new_active = Some(sid); }
                                if let Some(text) = copy_text { result.copy_text = Some(text); }
                                if paste_requested { result.paste_requested = Some(sid); }
                                if save_content_requested { result.save_content = Some(sid); }
                                if toggle_recording { result.toggle_recording = Some(sid); }


                                if let Some((cols, rows)) = resize {
                                    let sm = session_manager.clone();
                                    rt.block_on(async { sm.resize(sid, cols, rows).await });
                                }

                                if is_active || resp.has_focus() {
                                    let mut inputs: Vec<Vec<u8>> = Vec::new();
                                    let modifiers_held = ui.input(|i| i.modifiers);
                                    ui.input(|i| {
                                        for event in &i.events {
                                            let data: Vec<u8> = match event {
                                                egui::Event::Text(text) => {
                                                    if modifiers_held.ctrl { vec![] }
                                                    else { text.as_bytes().to_vec() }
                                                }
                                                egui::Event::Key { key, pressed: true, modifiers, .. } => {
                                                    key_to_bytes(*key, *modifiers)
                                                }
                                                _ => vec![],
                                            };
                                            if !data.is_empty() { inputs.push(data); }
                                        }
                                    });
                                    for data in inputs {
                                        let sm = session_manager.clone();
                                        rt.block_on(async { sm.send_input(sid, data).await });
                                    }
                                }
                            }
                        }
                    }
                }

                result
            }

            PaneTree::HSplit { ratio, left, right } => {
                let avail = ui.available_size();
                let left_w = (avail.x - SEP) * *ratio;
                let right_w = avail.x - SEP - left_w;

                let mut result = PaneResult::default();

                // Layout horizontal explícito para side-by-side
                ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                    ui.spacing_mut().item_spacing = Vec2::ZERO;

                    let left_resp = ui.allocate_ui(Vec2::new(left_w, avail.y), |ui| {
                        Self::render_pane_tree(ui, left, active_pane, terminal_buffers, pane_states, font_size, session_manager, rt)
                    });
                    merge_pane_result(&mut result, left_resp.inner);

                    let (sep_rect, sep_resp) = ui.allocate_exact_size(Vec2::new(SEP, avail.y), egui::Sense::drag());
                    let sep_color = if sep_resp.hovered() || sep_resp.dragged() { Ds::ACCENT } else { Ds::BORDER };
                    ui.painter().rect_filled(sep_rect, CornerRadius::ZERO, sep_color);
                    if sep_resp.hovered() || sep_resp.dragged() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                    }
                    if sep_resp.dragged() {
                        *ratio = (*ratio + sep_resp.drag_delta().x / avail.x).clamp(0.1, 0.9);
                    }

                    let right_resp = ui.allocate_ui(Vec2::new(right_w, avail.y), |ui| {
                        Self::render_pane_tree(ui, right, active_pane, terminal_buffers, pane_states, font_size, session_manager, rt)
                    });
                    merge_pane_result(&mut result, right_resp.inner);
                });

                result
            }

            PaneTree::VSplit { ratio, top, bottom } => {
                let avail = ui.available_size();
                let top_h = (avail.y - SEP) * *ratio;
                let bot_h = avail.y - SEP - top_h;

                let mut result = PaneResult::default();

                let top_resp = ui.allocate_ui(Vec2::new(avail.x, top_h), |ui| {
                    Self::render_pane_tree(ui, top, active_pane, terminal_buffers, pane_states, font_size, session_manager, rt)
                });
                merge_pane_result(&mut result, top_resp.inner);

                let (sep_rect, sep_resp) = ui.allocate_exact_size(Vec2::new(avail.x, SEP), egui::Sense::drag());
                let sep_color = if sep_resp.hovered() || sep_resp.dragged() { Ds::ACCENT } else { Ds::BORDER };
                ui.painter().rect_filled(sep_rect, CornerRadius::ZERO, sep_color);
                if sep_resp.hovered() || sep_resp.dragged() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                }
                if sep_resp.dragged() {
                    *ratio = (*ratio + sep_resp.drag_delta().y / avail.y).clamp(0.1, 0.9);
                }

                let bot_resp = ui.allocate_ui(Vec2::new(avail.x, bot_h), |ui| {
                    Self::render_pane_tree(ui, bottom, active_pane, terminal_buffers, pane_states, font_size, session_manager, rt)
                });
                merge_pane_result(&mut result, bot_resp.inner);

                result
            }
        }
    }

    fn handle_vault_action(&mut self, action: VaultAction) {
        match action {
            VaultAction::Connect(host) => {
                if host.hostname.is_empty() {
                    self.status_message = format!("Host '{}' sem HostName configurado", host.alias);
                    self.status_ok = false;
                    return;
                }
                self.connect_to_host(host);
            }
            VaultAction::Save(host) => {
                if let Some(e) = self.hosts.iter_mut().find(|h| h.id == host.id) {
                    *e = host;
                } else {
                    self.hosts.push(host);
                }
                self.persist_hosts();
            }
            VaultAction::Delete(id) => {
                self.hosts.retain(|h| h.id != id);
                self.persist_hosts();
            }
            VaultAction::Duplicate(host) => {
                let mut h = host.clone();
                h.id = Uuid::new_v4();
                h.alias = format!("{}-copy", host.alias);
                self.hosts.push(h);
                self.persist_hosts();
            }
            VaultAction::NewBlank => {
                self.connect_dialog.open_blank();
            }
            VaultAction::Edit(host) => {
                self.connect_dialog.open_with_host(&host);
            }
            VaultAction::Export | VaultAction::Import => {}
        }
    }

    fn persist_hosts(&mut self) {
        if let Some(ref s) = self.vault_store {
            match s.save_hosts(&self.hosts) {
                Ok(()) => {
                    // Atualiza mtime para não detectar a própria escrita como mudança externa
                    self.ssh_config_mtime = s.ssh_config_mtime();
                    self.status_message = format!("{} hosts salvos", self.hosts.len());
                    self.status_ok = true;
                }
                Err(e) => {
                    self.status_message = format!("Erro ao salvar vault: {}", e);
                    self.status_ok = false;
                }
            }
        }
    }

    fn render_connecting(ui: &mut egui::Ui) {
        let rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(rect, egui::epaint::CornerRadius::ZERO, Ds::BG_BASE);
        ui.centered_and_justified(|ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(rect.height() * 0.3);
                // Spinner visual simples — círculo pulsando via sin do tempo
                let t = ui.input(|i| i.time) as f32;
                let radius = 18.0 + (t * 2.0).sin() * 3.0;
                let center = ui.cursor().center_top() + egui::Vec2::new(0.0, 30.0);
                ui.painter().circle_stroke(
                    center,
                    radius,
                    egui::Stroke::new(2.5, Ds::ACCENT),
                );
                ui.painter().circle_filled(
                    center + egui::Vec2::new((t * 2.5).cos() * radius, (t * 2.5).sin() * radius),
                    4.0,
                    Ds::ACCENT,
                );
                ui.add_space(60.0);
                ui.label(RichText::new("Conectando...").color(Ds::TEXT_PRIMARY).size(18.0).strong());
                ui.add_space(Ds::SPACE_SM);
                ui.label(RichText::new("Estabelecendo sessão SSH").color(Ds::TEXT_MUTED).size(Ds::FONT_MD));
                ui.ctx().request_repaint();
            });
        });
    }

    fn render_disconnected(ui: &mut egui::Ui, sid: Uuid, error: Option<&str>, result: &mut PaneResult) {
        let rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(rect, egui::epaint::CornerRadius::ZERO, Ds::BG_BASE);

        ui.centered_and_justified(|ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(rect.height() * 0.25);

                // Ícone
                let icon_rect = egui::Rect::from_center_size(
                    egui::Pos2::new(ui.available_rect_before_wrap().center().x, ui.cursor().top() + 36.0),
                    egui::Vec2::splat(64.0),
                );
                ui.painter().rect_filled(icon_rect, egui::epaint::CornerRadius::same(16), Ds::BG_SURFACE);
                ui.painter().rect_stroke(
                    icon_rect,
                    egui::epaint::CornerRadius::same(16),
                    egui::Stroke::new(1.5, Ds::RED),
                    egui::StrokeKind::Inside,
                );
                ui.painter().text(
                    icon_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "⚡",
                    egui::FontId::proportional(28.0),
                    Ds::RED,
                );
                ui.add_space(76.0);

                // Título
                let title = if error.is_some() { "Falha na Conexão" } else { "Sessão Encerrada" };
                ui.label(RichText::new(title).color(Ds::TEXT_PRIMARY).size(22.0).strong());
                ui.add_space(Ds::SPACE_XS);

                // Mensagem de erro ou descrição genérica
                let msg = error.unwrap_or("A sessão SSH foi encerrada pelo servidor.");
                ui.label(RichText::new(msg).color(Ds::TEXT_MUTED).size(Ds::FONT_MD));
                ui.add_space(Ds::SPACE_XL);

                // Botões
                ui.horizontal(|ui| {
                    let btn_w = 140.0;
                    let btn_h = 36.0;
                    let gap = Ds::SPACE_MD;
                    let total = btn_w * 2.0 + gap;
                    ui.add_space((ui.available_width() - total) * 0.5);

                    // Botão Reconectar
                    let reconnect_btn = egui::Button::new(
                        RichText::new("Reconectar").color(Ds::BG_BASE).size(Ds::FONT_MD).strong()
                    )
                    .fill(Ds::ACCENT)
                    .stroke(egui::Stroke::NONE)
                    .corner_radius(Ds::R_MD)
                    .min_size(egui::Vec2::new(btn_w, btn_h));

                    if ui.add(reconnect_btn).clicked() {
                        result.reconnect = Some(sid);
                    }

                    ui.add_space(gap);

                    // Botão Fechar
                    let close_btn = egui::Button::new(
                        RichText::new("Fechar Aba").color(Ds::TEXT_PRIMARY).size(Ds::FONT_MD)
                    )
                    .fill(Ds::BG_SURFACE)
                    .stroke(egui::Stroke::new(1.0, Ds::BORDER))
                    .corner_radius(Ds::R_MD)
                    .min_size(egui::Vec2::new(btn_w, btn_h));

                    if ui.add(close_btn).clicked() {
                        result.close_pane = Some(sid);
                    }
                });
            });
        });
    }

    fn detach_active_pane(&mut self) {
        let Some(tab_id) = self.active_tab else { return };
        let Some(tab_idx) = self.tabs.iter().position(|t| t.id == tab_id) else { return };
        let active_pane = self.tabs[tab_idx].active_pane;

        // Só faz sentido se o tab tem split (mais de um pane)
        let sessions = self.tabs[tab_idx].pane_tree.sessions();
        if sessions.len() <= 1 { return; }

        // Remove o pane da árvore atual
        let old_tree = std::mem::replace(
            &mut self.tabs[tab_idx].pane_tree,
            PaneTree::Terminal(active_pane),
        );
        if let Some(new_tree) = old_tree.close_pane(active_pane) {
            self.tabs[tab_idx].pane_tree = new_tree;
            let remaining = self.tabs[tab_idx].pane_tree.sessions();
            if let Some(&first) = remaining.first() {
                self.tabs[tab_idx].active_pane = first;
            }
        }

        // Cria nova aba com esse pane
        let host_alias = self.hosts.iter()
            .find(|h| {
                if let Some(buf) = self.terminal_buffers.get(&active_pane) {
                    let _ = buf; true
                } else { false }
            })
            .map(|h| h.alias.clone())
            .unwrap_or_else(|| format!("pane-{}", &active_pane.to_string()[..4]));

        let new_tab = crate::pane::Tab::new(host_alias.clone(), &host_alias, active_pane);
        let new_tab_id = new_tab.id;
        self.tabs.push(new_tab);
        self.active_tab = Some(new_tab_id);
    }

    fn reconnect_session(&mut self, old_sid: Uuid, tab_idx: usize) {
        // Descobre o host pelo alias da aba
        let host_alias = self.tabs[tab_idx].host_alias.clone();
        let host = match self.hosts.iter().find(|h| h.alias == host_alias).cloned() {
            Some(h) => h,
            None => {
                self.status_message = format!("Host '{}' não encontrado no vault", host_alias);
                self.status_ok = false;
                return;
            }
        };

        // Desconecta a sessão antiga e limpa o estado
        let sm = self.session_manager.clone();
        let rt = self.rt.clone();
        rt.block_on(async { sm.disconnect(old_sid).await });
        self.output_receivers.remove(&old_sid);
        self.pane_states.remove(&old_sid);

        // Cria nova sessão
        let scrollback = self.settings.scrollback_lines;
        match rt.block_on(async { sm.connect(host.clone(), None).await }) {
            Ok((new_sid, output_rx)) => {
                // Reutiliza o buffer existente (ou cria novo)
                let buf = Arc::new(Mutex::new(TerminalBuffer::new(new_sid, 220, 50, scrollback)));
                self.terminal_buffers.remove(&old_sid);
                self.terminal_buffers.insert(new_sid, buf);
                self.output_receivers.insert(new_sid, output_rx);
                self.pane_states.insert(new_sid, PaneState::Connecting);

                // Atualiza a pane_tree para apontar para o novo session_id
                let old_tree = std::mem::replace(
                    &mut self.tabs[tab_idx].pane_tree,
                    crate::pane::PaneTree::Terminal(new_sid),
                );
                self.tabs[tab_idx].pane_tree = replace_session_id(old_tree, old_sid, new_sid);
                self.tabs[tab_idx].active_pane = new_sid;

                self.status_message = format!("Reconectando em {}...", host.alias);
                self.status_ok = true;
            }
            Err(e) => {
                self.pane_states.insert(old_sid, PaneState::Disconnected {
                    error: Some(format!("Reconexão falhou: {}", e)),
                });
                self.status_message = format!("Erro ao reconectar: {}", e);
                self.status_ok = false;
            }
        }
    }

    fn paste_from_clipboard(&mut self, session_id: Uuid) {
        match arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
            Ok(text) if !text.is_empty() => {
                let sm = self.session_manager.clone();
                let rt = self.rt.clone();
                rt.block_on(async { sm.send_input(session_id, text.into_bytes()).await });
            }
            _ => {
                self.status_message = "Área de transferência vazia".into();
                self.status_ok = false;
            }
        }
    }

    fn save_terminal_content(&mut self, session_id: Uuid) {
        if let Some(buf_arc) = self.terminal_buffers.get(&session_id) {
            if let Ok(buf) = buf_arc.lock() {
                let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
                let filename = format!("nsr_terminal_{}.txt", timestamp);
                let path = dirs::home_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("."))
                    .join("Downloads")
                    .join(&filename);
                match buf.save_content(&path) {
                    Ok(()) => {
                        self.status_message = format!("Salvo em {}", path.display());
                        self.status_ok = true;
                    }
                    Err(e) => {
                        self.status_message = format!("Erro ao salvar: {}", e);
                        self.status_ok = false;
                    }
                }
            }
        }
    }

    fn toggle_terminal_recording(&mut self, session_id: Uuid) {
        if let Some(buf_arc) = self.terminal_buffers.get(&session_id) {
            if let Ok(mut buf) = buf_arc.lock() {
                if buf.recording {
                    buf.stop_recording();
                    self.status_message = "Gravação parada".into();
                    self.status_ok = true;
                } else {
                    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
                    let filename = format!("nsr_record_{}.log", timestamp);
                    let path = dirs::home_dir()
                        .unwrap_or_else(|| std::path::PathBuf::from("."))
                        .join("Downloads")
                        .join(&filename);
                    match buf.start_recording(path.clone()) {
                        Ok(()) => {
                            self.status_message = format!("Gravando em {}", path.display());
                            self.status_ok = true;
                        }
                        Err(e) => {
                            self.status_message = format!("Erro ao iniciar gravação: {}", e);
                            self.status_ok = false;
                        }
                    }
                }
            }
        }
    }

    fn show_welcome(&mut self, ui: &mut egui::Ui) {
        let avail = ui.available_rect_before_wrap();

        // ── Header ────────────────────────────────────────────────────────
        ui.vertical_centered(|ui| {
            ui.add_space(32.0);

            // Logo
            let logo_rect = Rect::from_center_size(
                Pos2::new(avail.center().x, ui.cursor().top() + 32.0),
                Vec2::new(56.0, 56.0),
            );
            ui.painter().rect_filled(logo_rect, CornerRadius::same(14), Ds::ACCENT_DIM);
            ui.painter().text(
                logo_rect.center(),
                egui::Align2::CENTER_CENTER,
                ">_",
                egui::FontId::monospace(20.0),
                Ds::ACCENT,
            );
            ui.add_space(64.0);

            ui.label(RichText::new("NSR-SSH").size(28.0).color(Ds::TEXT_PRIMARY).strong());
            ui.add_space(4.0);
            ui.label(RichText::new("No Subscription Required").size(Ds::FONT_MD).color(Ds::TEXT_MUTED));
            ui.add_space(Ds::SPACE_LG);

            // ── Quick actions ─────────────────────────────────────────────
            ui.horizontal(|ui| {
                // centralizar manualmente
                let card_w = 120.0;
                let n = 3;
                let gap = Ds::SPACE_MD;
                let total = card_w * n as f32 + gap * (n - 1) as f32;
                ui.add_space((avail.width() - total) * 0.5);

                if quick_action_card(ui, "⚡", "Nova Conexão", "Ctrl+T", card_w).clicked() {
                    self.connect_dialog.open_blank();
                }
                ui.add_space(gap);
                if quick_action_card(ui, "☰", "Mostrar Vault", "Ctrl+B", card_w).clicked() {
                    self.show_vault = !self.show_vault;
                }
                ui.add_space(gap);
                if quick_action_card(ui, "⚙", "Configurações", "Ctrl+,", card_w).clicked() {
                    self.settings.open = true;
                }
            });

            ui.add_space(Ds::SPACE_XL);
        });

        // ── Lista de hosts recentes ───────────────────────────────────────
        if !self.hosts.is_empty() {
            ui.separator();
            ui.add_space(Ds::SPACE_SM);

            ui.horizontal(|ui| {
                ui.add_space(Ds::SPACE_LG);
                ui.label(RichText::new("HOSTS SALVOS").color(Ds::TEXT_MUTED).size(Ds::FONT_XS).strong());
            });
            ui.add_space(Ds::SPACE_XS);

            // Campo de pesquisa
            ui.horizontal(|ui| {
                ui.add_space(Ds::SPACE_LG);
                ui.add(
                    egui::TextEdit::singleline(&mut self.welcome_search)
                        .hint_text("🔍  buscar hosts...")
                        .font(egui::FontId::proportional(Ds::FONT_SM))
                        .desired_width(320.0)
                        .margin(egui::Margin::symmetric(Ds::SPACE_SM as i8, Ds::SPACE_XS as i8)),
                );
            });
            ui.add_space(Ds::SPACE_SM);

            let query = self.welcome_search.to_lowercase();
            let filtered: Vec<_> = self.hosts.iter()
                .filter(|h| query.is_empty()
                    || h.alias.to_lowercase().contains(&query)
                    || h.hostname.to_lowercase().contains(&query))
                .cloned()
                .collect();

            egui::ScrollArea::vertical()
                .max_height(avail.height() * 0.4)
                .show(ui, |ui| {
                    for host in filtered {
                        ui.add_space(Ds::SPACE_XS);
                        ui.horizontal(|ui| {
                            ui.add_space(Ds::SPACE_LG);

                            // Aloca área com sense para capturar cliques
                            let desired = Vec2::new(ui.available_width() - Ds::SPACE_LG, 36.0);
                            let (rect, resp) = ui.allocate_exact_size(desired, egui::Sense::click());

                            if ui.is_rect_visible(rect) {
                                let hovered = resp.hovered();
                                let fill = if hovered { Ds::BG_ACTIVE } else { Ds::BG_SURFACE };
                                let border = if hovered { Ds::ACCENT } else { Ds::BORDER };
                                ui.painter().rect_filled(rect, Ds::R_SM, fill);
                                ui.painter().rect_stroke(rect, Ds::R_SM, Stroke::new(1.0, border), egui::StrokeKind::Inside);

                                // ">_" ícone
                                ui.painter().text(
                                    Pos2::new(rect.left() + 10.0, rect.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    ">_",
                                    egui::FontId::monospace(Ds::FONT_SM),
                                    Ds::ACCENT,
                                );
                                // alias
                                ui.painter().text(
                                    Pos2::new(rect.left() + 32.0, rect.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    &host.alias,
                                    egui::FontId::proportional(Ds::FONT_MD),
                                    Ds::TEXT_PRIMARY,
                                );
                                // user@host:port
                                let conn_str = format!("{}@{}:{}", host.user, host.hostname, host.port);
                                ui.painter().text(
                                    Pos2::new(rect.right() - 8.0, rect.center().y),
                                    egui::Align2::RIGHT_CENTER,
                                    &conn_str,
                                    egui::FontId::monospace(Ds::FONT_SM),
                                    Ds::TEXT_MUTED,
                                );
                            }

                            if resp.clicked() {
                                self.connect_to_host(host.clone());
                            }
                        });
                    }
                });
        }
    }
}

impl eframe::App for NsrApp {
    #[allow(deprecated)]
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        self.process_session_events();
        self.drain_output_receivers();
        self.check_ssh_config_changes();

        Ds::apply_global_visuals(&ctx);

        // Atalhos globais
        let (close_pane, open_new, toggle_vault, toggle_settings, split_h, split_v, next_tab, prev_tab) =
            ctx.input_mut(|i| (
                i.consume_key(Modifiers::CTRL, Key::W),
                i.consume_key(Modifiers::CTRL, Key::T),
                i.consume_key(Modifiers::CTRL, Key::B),
                i.consume_key(Modifiers::CTRL, Key::Comma),
                i.consume_key(Modifiers::CTRL | Modifiers::SHIFT, Key::Backslash),
                i.consume_key(Modifiers::CTRL | Modifiers::SHIFT, Key::Minus),
                i.consume_key(Modifiers::CTRL, Key::Tab),
                i.consume_key(Modifiers::CTRL | Modifiers::SHIFT, Key::Tab),
            ));

        if close_pane { self.close_active_pane(); }
        if open_new { self.connect_dialog.open_blank(); }
        if toggle_vault { self.show_vault = !self.show_vault; }
        if toggle_settings { self.settings.open = !self.settings.open; }
        if split_h { self.split_active_h(); }
        if split_v { self.split_active_v(); }
        if next_tab {
            if let Some(id) = self.active_tab {
                if let Some(pos) = self.tabs.iter().position(|t| t.id == id) {
                    let next = (pos + 1) % self.tabs.len();
                    self.active_tab = Some(self.tabs[next].id);
                }
            }
        }
        if prev_tab {
            if let Some(id) = self.active_tab {
                if let Some(pos) = self.tabs.iter().position(|t| t.id == id) {
                    let prev = if pos == 0 { self.tabs.len() - 1 } else { pos - 1 };
                    self.active_tab = Some(self.tabs[prev].id);
                }
            }
        }

        // ── Tab bar ──────────────────────────────────────────────────────────
        egui::Panel::top("tab_bar")
            .exact_size(Ds::TAB_H)
            .show(&ctx, |ui| {
                if let Some(action) = TabBar::show(ui, &self.tabs, self.active_tab, &self.theme) {
                    match action {
                        TabBarAction::Activate(id) => self.active_tab = Some(id),
                        TabBarAction::Close(id) => self.close_tab(id),
                        TabBarAction::New => self.connect_dialog.open_blank(),
                        TabBarAction::Duplicate(id) => self.duplicate_tab(id),
                        TabBarAction::OpenSettings => self.settings.open = true,
                        TabBarAction::SplitH(_) => self.split_active_h(),
                        TabBarAction::SplitV(_) => self.split_active_v(),
                        TabBarAction::StartDrag(id) => self.dragging_tab = Some(id),
                        TabBarAction::EndDrag => { /* handled by CentralPanel drop logic */ }
                        TabBarAction::DetachPane(_) => self.detach_active_pane(),
                    }
                }
            });

        // ── Status bar ───────────────────────────────────────────────────────
        egui::Panel::bottom("status_bar")
            .exact_size(Ds::STATUS_H)
            .show(&ctx, |ui| {
                let rect = ui.available_rect_before_wrap();
                ui.painter().rect_filled(rect, CornerRadius::ZERO, Ds::BG_PANEL);
                ui.painter().line_segment(
                    [rect.left_top(), rect.right_top()],
                    Stroke::new(1.0, Ds::BORDER),
                );
                ui.horizontal(|ui| {
                    ui.add_space(Ds::SPACE_SM);
                    let dot_color = if self.status_ok { Ds::GREEN } else { Ds::RED };
                    ui.painter().circle_filled(
                        Pos2::new(ui.cursor().left() + 5.0, rect.center().y),
                        4.0,
                        dot_color,
                    );
                    ui.add_space(12.0);
                    ui.label(RichText::new(&self.status_message).size(Ds::FONT_SM).color(Ds::TEXT_SECONDARY));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(Ds::SPACE_SM);
                        ui.label(RichText::new(format!("{} sessões  •  {} hosts", self.tabs.len(), self.hosts.len())).size(Ds::FONT_SM).color(Ds::TEXT_MUTED));
                        // Stats da sessão ativa
                        if let Some(active_id) = self.active_tab {
                            if let Some(tab) = self.tabs.iter().find(|t| t.id == active_id) {
                                let active_pane = tab.active_pane;
                                // ID abreviado da sessão
                                let short_id = &active_pane.to_string()[..8];
                                ui.add_space(Ds::SPACE_SM);
                                ui.label(RichText::new(format!("ID:{}", short_id)).size(Ds::FONT_SM).color(Ds::TEXT_MUTED).family(egui::FontFamily::Monospace));
                                // Tempo online
                                if let Some(connected_at) = self.session_connected_at.get(&active_pane) {
                                    let uptime = format_duration(connected_at.elapsed());
                                    ui.add_space(Ds::SPACE_SM);
                                    ui.label(RichText::new(format!("⏱ {}", uptime)).size(Ds::FONT_SM).color(Ds::TEXT_MUTED));
                                }
                            }
                        }
                        ui.add_space(Ds::SPACE_SM);
                    });
                });
            });

        // ── Vault sidebar ────────────────────────────────────────────────────
        if self.show_vault {
            egui::Panel::left("vault_panel")
                .resizable(true)
                .default_size(Ds::SIDEBAR_W)
                .size_range(160.0..=380.0)
                .show(&ctx, |ui| {
                    if let Some(action) = self.vault_panel.show(ui, &mut self.hosts, &self.theme) {
                        self.handle_vault_action(action);
                    }
                });
        }

        // ── Dialogs ──────────────────────────────────────────────────────────
        if let Some(req) = self.connect_dialog.show_window(&ctx) {
            if req.save_to_vault {
                self.handle_vault_action(VaultAction::Save(req.host.clone()));
            }
            if req.connect {
                self.connect_to_host_with_password(req.host, req.password);
            }
        }
        if self.settings.open {
            self.settings.show_window(&ctx, &mut self.theme);
        }

        // ── Central panel ────────────────────────────────────────────────────
        egui::CentralPanel::default()
            .frame(egui::Frame::new())
            .show(&ctx, |ui| {
            let bg = ui.available_rect_before_wrap();
            // Captura o rect ANTES de renderizar qualquer coisa (sem margens do Frame padrão)
            let central_rect = bg;
            ui.painter().rect_filled(bg, CornerRadius::ZERO, Ds::BG_BASE);

            if self.tabs.is_empty() {
                self.show_welcome(ui);
            } else if let Some(active_id) = self.active_tab {
                if let Some(tab_idx) = self.tabs.iter().position(|t| t.id == active_id) {
                    let active_pane = self.tabs[tab_idx].active_pane;
                    let font_size = self.settings.font_size;

                    let result = Self::render_pane_tree(
                        ui, &mut self.tabs[tab_idx].pane_tree, active_pane,
                        &mut self.terminal_buffers, &self.pane_states, font_size,
                        &self.session_manager, &self.rt,
                    );
                    if let Some(na) = result.new_active {
                        self.tabs[tab_idx].active_pane = na;
                    }
                    if let Some(text) = result.copy_text {
                        ui.ctx().copy_text(text);
                        self.status_message = "Copiado para área de transferência".into();
                        self.status_ok = true;
                    }
                    if let Some(sid) = result.paste_requested {
                        self.paste_from_clipboard(sid);
                    }
                    if let Some(sid) = result.save_content {
                        self.save_terminal_content(sid);
                    }
                    if let Some(sid) = result.toggle_recording {
                        self.toggle_terminal_recording(sid);
                    }
                    if let Some(sid) = result.reconnect {
                        self.reconnect_session(sid, tab_idx);
                    }
                    if let Some(_sid) = result.close_pane {
                        self.close_active_pane();
                    }
                    if result.split_h.is_some() {
                        self.split_active_h();
                    }
                    if result.split_v.is_some() {
                        self.split_active_v();
                    }

                    // ── Zonas de drop quando há aba sendo arrastada ──────────
                    if let Some(drag_id) = self.dragging_tab {
                        // Mostra zonas mesmo arrastando a aba ativa, mas só se há mais de 1 tab OU
                        // o tab ativo tem split (para mover pane para nova posição na mesma tab)
                        let has_multiple_tabs = self.tabs.len() > 1;
                        let is_different_tab = drag_id != active_id;
                        if is_different_tab || has_multiple_tabs {
                            let r = central_rect;
                            let pointer_pos = ctx.input(|i| i.pointer.hover_pos());
                            let mouse_released = ctx.input(|i| i.pointer.any_released());
                            let painter = ctx.layer_painter(egui::LayerId::new(
                                egui::Order::Foreground,
                                egui::Id::new("drop_zones"),
                            ));

                            // Determina zona pelo quadrante sem sobreposição:
                            // divide o rect em 4 triângulos pelas diagonais
                            let hovered_side = pointer_pos.and_then(|p| {
                                if !r.contains(p) { return None; }
                                let dx = p.x - r.center().x;  // + = direita
                                let dy = p.y - r.center().y;  // + = baixo
                                // Escala para comparação normalizada
                                let nx = dx / (r.width() * 0.5);
                                let ny = dy / (r.height() * 0.5);
                                if nx.abs() > ny.abs() {
                                    if nx > 0.0 { Some("right") } else { Some("left") }
                                } else {
                                    if ny > 0.0 { Some("bottom") } else { Some("top") }
                                }
                            });

                            let zone_depth = 0.35; // 35% do lado como preview
                            let zones: &[(&str, Rect, &str)] = &[
                                ("↑  Cima",    Rect::from_min_max(r.min, Pos2::new(r.max.x, r.min.y + r.height() * zone_depth)), "top"),
                                ("↓  Baixo",   Rect::from_min_max(Pos2::new(r.min.x, r.max.y - r.height() * zone_depth), r.max), "bottom"),
                                ("←  Esquerda",Rect::from_min_max(r.min, Pos2::new(r.min.x + r.width() * zone_depth, r.max.y)), "left"),
                                ("→  Direita", Rect::from_min_max(Pos2::new(r.max.x - r.width() * zone_depth, r.min.y), r.max), "right"),
                            ];

                            for (label, zone_rect, side) in zones {
                                let active = hovered_side == Some(side);
                                let fill = if active {
                                    egui::Color32::from_rgba_premultiplied(
                                        Ds::ACCENT.r(), Ds::ACCENT.g(), Ds::ACCENT.b(), 110,
                                    )
                                } else {
                                    egui::Color32::from_rgba_premultiplied(80, 80, 120, 40)
                                };
                                painter.rect_filled(*zone_rect, egui::epaint::CornerRadius::ZERO, fill);
                                if active {
                                    painter.rect_stroke(
                                        *zone_rect,
                                        egui::epaint::CornerRadius::ZERO,
                                        egui::Stroke::new(2.0, Ds::ACCENT),
                                        egui::StrokeKind::Inside,
                                    );
                                }
                                painter.text(
                                    zone_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    label,
                                    egui::FontId::proportional(if active { 16.0 } else { 13.0 }),
                                    if active { egui::Color32::WHITE } else { egui::Color32::from_white_alpha(100) },
                                );

                                if active && mouse_released {
                                    // Não pode arrastar aba para si mesma sem split
                                    if drag_id != active_id {
                                        if let Some(drag_tab_idx) = self.tabs.iter().position(|t| t.id == drag_id) {
                                            // Captura tudo por valor antes de qualquer mutação
                                            let drag_session = self.tabs[drag_tab_idx].active_pane;
                                            let dest_id = active_id; // ID estável, não índice

                                            // Remove aba arrastada primeiro
                                            self.tabs.remove(drag_tab_idx);

                                            // Reindexar pelo ID, que é estável
                                            if let Some(dest_idx) = self.tabs.iter().position(|t| t.id == dest_id) {
                                                let dest_session = self.tabs[dest_idx].active_pane;
                                                let old_tree = std::mem::replace(
                                                    &mut self.tabs[dest_idx].pane_tree,
                                                    PaneTree::Terminal(dest_session),
                                                );
                                                self.tabs[dest_idx].pane_tree = match *side {
                                                    "left"   => PaneTree::HSplit { ratio: 0.5, left:  Box::new(PaneTree::Terminal(drag_session)), right: Box::new(old_tree) },
                                                    "right"  => PaneTree::HSplit { ratio: 0.5, left:  Box::new(old_tree), right: Box::new(PaneTree::Terminal(drag_session)) },
                                                    "top"    => PaneTree::VSplit { ratio: 0.5, top:   Box::new(PaneTree::Terminal(drag_session)), bottom: Box::new(old_tree) },
                                                    _        => PaneTree::VSplit { ratio: 0.5, top:   Box::new(old_tree), bottom: Box::new(PaneTree::Terminal(drag_session)) },
                                                };
                                                self.tabs[dest_idx].active_pane = drag_session;
                                                self.active_tab = Some(dest_id);

                                                // Envia resize imediato para as duas sessões com ~metade das colunas/linhas
                                                // para que o shell redesenhe antes do próximo frame (minimiza "flash" preto)
                                                let font_size = self.settings.font_size;
                                                let char_w = font_size * 0.601;
                                                let char_h = font_size * 1.25;
                                                let half_cols = ((central_rect.width() * 0.5 / char_w) as u16).max(10);
                                                let half_rows = ((central_rect.height() / char_h) as u16).max(4);
                                                let sm = self.session_manager.clone();
                                                let rt = self.rt.clone();
                                                let ds = drag_session;
                                                let dest_s = dest_session;
                                                rt.block_on(async {
                                                    sm.resize(ds, half_cols, half_rows).await;
                                                    sm.resize(dest_s, half_cols, half_rows).await;
                                                });
                                                // Atualiza buffers locais para não disparar resize duplo no próximo frame
                                                if let Some(buf) = self.terminal_buffers.get(&ds) {
                                                    if let Ok(mut b) = buf.lock() { b.resize(half_cols as usize, half_rows as usize); }
                                                }
                                                if let Some(buf) = self.terminal_buffers.get(&dest_s) {
                                                    if let Ok(mut b) = buf.lock() { b.resize(half_cols as usize, half_rows as usize); }
                                                }
                                            }
                                        }
                                    }
                                    self.dragging_tab = None;
                                }
                            }

                            // Cancela drag se soltar fora de qualquer zona
                            if mouse_released && hovered_side.is_none() {
                                self.dragging_tab = None;
                            }
                        }
                    }
                }
            }
        });

        // Fallback: limpa drag se mouse foi solto fora de qualquer zona de drop
        if self.dragging_tab.is_some() {
            let mouse_down = ctx.input(|i| i.pointer.primary_down());
            if !mouse_down {
                self.dragging_tab = None;
            }
        }

        ctx.request_repaint();
    }
}

fn replace_session_id(tree: crate::pane::PaneTree, old: Uuid, new: Uuid) -> crate::pane::PaneTree {
    match tree {
        crate::pane::PaneTree::Terminal(id) if id == old => crate::pane::PaneTree::Terminal(new),
        crate::pane::PaneTree::Terminal(_) => tree,
        crate::pane::PaneTree::HSplit { ratio, left, right } => crate::pane::PaneTree::HSplit {
            ratio,
            left: Box::new(replace_session_id(*left, old, new)),
            right: Box::new(replace_session_id(*right, old, new)),
        },
        crate::pane::PaneTree::VSplit { ratio, top, bottom } => crate::pane::PaneTree::VSplit {
            ratio,
            top: Box::new(replace_session_id(*top, old, new)),
            bottom: Box::new(replace_session_id(*bottom, old, new)),
        },
    }
}

fn merge_pane_result(dst: &mut PaneResult, src: PaneResult) {
    if src.new_active.is_some() { dst.new_active = src.new_active; }
    if src.copy_text.is_some() { dst.copy_text = src.copy_text; }
    if src.paste_requested.is_some() { dst.paste_requested = src.paste_requested; }
    if src.save_content.is_some() { dst.save_content = src.save_content; }
    if src.toggle_recording.is_some() { dst.toggle_recording = src.toggle_recording; }
    if src.reconnect.is_some() { dst.reconnect = src.reconnect; }
    if src.close_pane.is_some() { dst.close_pane = src.close_pane; }
    if src.split_h.is_some() { dst.split_h = src.split_h; }
    if src.split_v.is_some() { dst.split_v = src.split_v; }
}

fn quick_action_card(ui: &mut egui::Ui, icon: &str, label: &str, shortcut: &str, width: f32) -> egui::Response {
    let card_h = 76.0;
    let sense = egui::Sense::click();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, card_h), sense);

    if ui.is_rect_visible(rect) {
        let hovered = resp.hovered();
        let fill = if hovered { Ds::BG_ACTIVE } else { Ds::BG_SURFACE };
        let border = if hovered { Ds::ACCENT } else { Ds::BORDER };
        ui.painter().rect_filled(rect, Ds::R_MD, fill);
        ui.painter().rect_stroke(rect, Ds::R_MD, Stroke::new(1.0, border), egui::StrokeKind::Inside);

        // ícone
        ui.painter().text(
            Pos2::new(rect.center().x, rect.top() + 20.0),
            egui::Align2::CENTER_TOP,
            icon,
            egui::FontId::proportional(18.0),
            if hovered { Ds::ACCENT } else { Ds::TEXT_SECONDARY },
        );
        // label
        ui.painter().text(
            Pos2::new(rect.center().x, rect.top() + 42.0),
            egui::Align2::CENTER_TOP,
            label,
            egui::FontId::proportional(Ds::FONT_SM),
            Ds::TEXT_PRIMARY,
        );
        // shortcut
        ui.painter().text(
            Pos2::new(rect.center().x, rect.top() + 58.0),
            egui::Align2::CENTER_TOP,
            shortcut,
            egui::FontId::monospace(Ds::FONT_XS),
            Ds::TEXT_MUTED,
        );
    }

    resp
}

fn key_to_bytes(key: Key, modifiers: Modifiers) -> Vec<u8> {
    // Ctrl+letra → ASCII control code (Ctrl+A = 0x01 ... Ctrl+Z = 0x1A)
    if modifiers.ctrl && !modifiers.shift && !modifiers.alt {
        let ctrl_byte: Option<u8> = match key {
            Key::A => Some(0x01), Key::B => Some(0x02), Key::C => Some(0x03),
            Key::D => Some(0x04), Key::E => Some(0x05), Key::F => Some(0x06),
            Key::G => Some(0x07), Key::H => Some(0x08), Key::I => Some(0x09),
            Key::J => Some(0x0A), Key::K => Some(0x0B), Key::L => Some(0x0C),
            Key::M => Some(0x0D), Key::N => Some(0x0E), Key::O => Some(0x0F),
            Key::P => Some(0x10), Key::Q => Some(0x11), Key::R => Some(0x12),
            Key::S => Some(0x13), Key::T => Some(0x14), Key::U => Some(0x15),
            Key::V => Some(0x16), Key::W => Some(0x17), Key::X => Some(0x18),
            Key::Y => Some(0x19), Key::Z => Some(0x1A),
            Key::OpenBracket => Some(0x1B),  // Ctrl+[ = ESC
            Key::Backslash => Some(0x1C),
            Key::CloseBracket => Some(0x1D),
            _ => None,
        };
        if let Some(b) = ctrl_byte {
            return vec![b];
        }
    }

    match key {
        Key::Enter => vec![b'\r'],
        Key::Backspace => vec![0x7f],
        Key::Tab => vec![b'\t'],
        Key::Escape => vec![0x1b],
        Key::ArrowUp => {
            if modifiers.shift { vec![0x1b, b'[', b'1', b';', b'2', b'A'] }
            else { vec![0x1b, b'[', b'A'] }
        }
        Key::ArrowDown => {
            if modifiers.shift { vec![0x1b, b'[', b'1', b';', b'2', b'B'] }
            else { vec![0x1b, b'[', b'B'] }
        }
        Key::ArrowRight => {
            if modifiers.ctrl { vec![0x1b, b'[', b'1', b';', b'5', b'C'] }
            else { vec![0x1b, b'[', b'C'] }
        }
        Key::ArrowLeft => {
            if modifiers.ctrl { vec![0x1b, b'[', b'1', b';', b'5', b'D'] }
            else { vec![0x1b, b'[', b'D'] }
        }
        Key::Home => vec![0x1b, b'[', b'H'],
        Key::End => vec![0x1b, b'[', b'F'],
        Key::Delete => vec![0x1b, b'[', b'3', b'~'],
        Key::PageUp => vec![0x1b, b'[', b'5', b'~'],
        Key::PageDown => vec![0x1b, b'[', b'6', b'~'],
        Key::F1 => vec![0x1b, b'O', b'P'],
        Key::F2 => vec![0x1b, b'O', b'Q'],
        Key::F3 => vec![0x1b, b'O', b'R'],
        Key::F4 => vec![0x1b, b'O', b'S'],
        Key::F5 => vec![0x1b, b'[', b'1', b'5', b'~'],
        Key::F6 => vec![0x1b, b'[', b'1', b'7', b'~'],
        Key::F7 => vec![0x1b, b'[', b'1', b'8', b'~'],
        Key::F8 => vec![0x1b, b'[', b'1', b'9', b'~'],
        Key::F9 => vec![0x1b, b'[', b'2', b'0', b'~'],
        Key::F10 => vec![0x1b, b'[', b'2', b'1', b'~'],
        Key::F11 => vec![0x1b, b'[', b'2', b'3', b'~'],
        Key::F12 => vec![0x1b, b'[', b'2', b'4', b'~'],
        _ => vec![],
    }
}

fn format_duration(d: Duration) -> String {
    let s = d.as_secs();
    if s < 60 {
        format!("{:02}s", s)
    } else if s < 3600 {
        format!("{:02}:{:02}", s / 60, s % 60)
    } else {
        format!("{:02}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60)
    }
}
