use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use egui::{Color32, Context, Key, Modifiers, RichText};
use tokio::sync::broadcast;
use uuid::Uuid;

use nsr_core::{SessionEvent, SessionManager};
use nsr_theme::{Theme, load_user_themes};
use nsr_vault::{Host, VaultStore};

use crate::connect_dialog::ConnectDialog;
use crate::pane::{PaneTree, Tab};
use crate::settings::SettingsPanel;
use crate::tab_bar::{TabBar, TabBarAction};
use crate::terminal_widget::TerminalBuffer;
use crate::vault_panel::{VaultAction, VaultPanel};

pub struct NsrApp {
    tabs: Vec<Tab>,
    active_tab: Option<Uuid>,
    hosts: Vec<Host>,
    theme: Theme,

    terminal_buffers: HashMap<Uuid, Arc<Mutex<TerminalBuffer>>>,
    output_receivers: HashMap<Uuid, broadcast::Receiver<Vec<u8>>>,

    session_manager: Arc<SessionManager>,
    event_rx: broadcast::Receiver<SessionEvent>,
    vault_store: Option<VaultStore>,

    vault_panel: VaultPanel,
    connect_dialog: ConnectDialog,
    settings: SettingsPanel,

    rt: Arc<tokio::runtime::Runtime>,

    show_vault: bool,
    status_message: String,
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
        let hosts = vault_store
            .as_ref()
            .and_then(|s| s.load_hosts().ok())
            .unwrap_or_default();

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
            session_manager,
            event_rx,
            vault_store,
            vault_panel: VaultPanel::new(),
            connect_dialog: ConnectDialog::new(),
            settings: SettingsPanel::new(&theme_name),
            rt,
            show_vault: true,
            status_message: "Pronto".into(),
        }
    }

    fn connect_to_host(&mut self, host: Host) {
        let sm = self.session_manager.clone();
        let host_clone = host.clone();
        let rt = self.rt.clone();

        match rt.block_on(async { sm.connect(host_clone).await }) {
            Ok((session_id, output_rx)) => {
                self.terminal_buffers.insert(
                    session_id,
                    Arc::new(Mutex::new(TerminalBuffer::new(session_id, 220, 50))),
                );
                self.output_receivers.insert(session_id, output_rx);

                let tab = Tab::new(host.alias.clone(), &host.alias, session_id);
                let tab_id = tab.id;
                self.tabs.push(tab);
                self.active_tab = Some(tab_id);
                self.status_message = format!("Conectando em {}...", host.alias);
            }
            Err(e) => {
                self.status_message = format!("Erro ao conectar: {}", e);
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

    fn drain_output_receivers(&mut self) {
        let session_ids: Vec<Uuid> = self.output_receivers.keys().copied().collect();
        for sid in session_ids {
            if let Some(rx) = self.output_receivers.get_mut(&sid) {
                loop {
                    match rx.try_recv() {
                        Ok(data) => {
                            if let Some(buf) = self.terminal_buffers.get(&sid) {
                                if let Ok(mut b) = buf.lock() {
                                    b.process(&data);
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
        }
    }

    fn process_session_events(&mut self) {
        loop {
            match self.event_rx.try_recv() {
                Ok(SessionEvent::Connected { .. }) => {
                    self.status_message = "Conectado".into();
                }
                Ok(SessionEvent::Disconnected { .. }) => {
                    self.status_message = "Desconectado".into();
                }
                Ok(SessionEvent::Error { message, .. }) => {
                    self.status_message = format!("Erro: {}", message);
                }
                Ok(SessionEvent::Output { session_id, data }) => {
                    if let Some(buf) = self.terminal_buffers.get(&session_id) {
                        if let Ok(mut b) = buf.lock() {
                            b.process(&data);
                        }
                    }
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
    }

    fn render_pane_tree(
        ui: &mut egui::Ui,
        pane: &PaneTree,
        active_pane: Uuid,
        terminal_buffers: &mut HashMap<Uuid, Arc<Mutex<TerminalBuffer>>>,
        theme: &Theme,
        font_size: f32,
        session_manager: &Arc<SessionManager>,
        rt: &Arc<tokio::runtime::Runtime>,
    ) -> Option<Uuid> {
        match pane {
            PaneTree::Terminal(sid) => {
                let sid = *sid;
                let mut clicked_pane = None;

                if let Some(buf_arc) = terminal_buffers.get(&sid) {
                    if let Ok(mut buf) = buf_arc.lock() {
                        let mut widget = crate::terminal_widget::TerminalWidget::new(&mut buf, theme)
                            .font_size(font_size)
                            .active(sid == active_pane);
                        let resp = widget.show(ui);

                        if resp.clicked() {
                            clicked_pane = Some(sid);
                        }

                        if sid == active_pane {
                            ui.input(|i| {
                                for event in &i.events {
                                    let data = match event {
                                        egui::Event::Text(text) => {
                                            text.as_bytes().to_vec()
                                        }
                                        egui::Event::Key { key, pressed: true, modifiers, .. } => {
                                            key_to_bytes(*key, *modifiers)
                                        }
                                        _ => vec![],
                                    };
                                    if !data.is_empty() {
                                        let sm = session_manager.clone();
                                        rt.block_on(async { sm.send_input(sid, data).await });
                                    }
                                }
                            });
                        }
                    }
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label(RichText::new("Conectando...").color(Color32::GRAY));
                    });
                }

                clicked_pane
            }
            PaneTree::HSplit { ratio: _, left, right } => {
                let mut new_active = None;
                ui.columns(2, |cols| {
                    if let Some(r) = Self::render_pane_tree(
                        &mut cols[0], left, active_pane,
                        terminal_buffers, theme, font_size, session_manager, rt,
                    ) { new_active = Some(r); }
                    if let Some(r) = Self::render_pane_tree(
                        &mut cols[1], right, active_pane,
                        terminal_buffers, theme, font_size, session_manager, rt,
                    ) { new_active = Some(r); }
                });
                new_active
            }
            PaneTree::VSplit { ratio, top, bottom } => {
                let top_h = ui.available_height() * ratio;
                let mut new_active = None;
                ui.vertical(|ui| {
                    ui.set_max_height(top_h);
                    if let Some(r) = Self::render_pane_tree(
                        ui, top, active_pane,
                        terminal_buffers, theme, font_size, session_manager, rt,
                    ) { new_active = Some(r); }
                });
                ui.separator();
                ui.vertical(|ui| {
                    if let Some(r) = Self::render_pane_tree(
                        ui, bottom, active_pane,
                        terminal_buffers, theme, font_size, session_manager, rt,
                    ) { new_active = Some(r); }
                });
                new_active
            }
        }
    }

    fn handle_vault_action(&mut self, action: VaultAction) {
        match action {
            VaultAction::Connect(host) => self.connect_to_host(host),
            VaultAction::Save(host) => {
                if let Some(existing) = self.hosts.iter_mut().find(|h| h.id == host.id) {
                    *existing = host;
                } else {
                    self.hosts.push(host);
                }
                if let Some(ref store) = self.vault_store {
                    let _ = store.save_hosts(&self.hosts);
                }
            }
            VaultAction::Delete(id) => {
                self.hosts.retain(|h| h.id != id);
                if let Some(ref store) = self.vault_store {
                    let _ = store.save_hosts(&self.hosts);
                }
            }
            VaultAction::Duplicate(host) => {
                let mut new_host = host.clone();
                new_host.id = Uuid::new_v4();
                new_host.alias = format!("{}-copy", host.alias);
                self.hosts.push(new_host);
            }
            VaultAction::Edit(host) => {
                self.vault_panel.editing_host = Some(host);
                self.vault_panel.show_add_dialog = true;
            }
            VaultAction::Export | VaultAction::Import => {}
        }
    }

    fn show_welcome(&self, ui: &mut egui::Ui) {
        let accent = Color32::from_rgb(self.theme.ui_accent.0, self.theme.ui_accent.1, self.theme.ui_accent.2);
        let dim = Color32::from_rgb(self.theme.ui_text_dim.0, self.theme.ui_text_dim.1, self.theme.ui_text_dim.2);

        ui.vertical_centered(|ui| {
            ui.add_space(80.0);
            ui.heading(RichText::new("NSR-SSH").size(42.0).color(accent));
            ui.add_space(8.0);
            ui.label(RichText::new("No Subscription Required").size(16.0).color(dim));
            ui.add_space(32.0);
            ui.label(RichText::new("Pressione Ctrl+T para nova conexão").color(dim).size(13.0));
            ui.add_space(6.0);
            ui.label(RichText::new("Ou selecione um host no Vault (Ctrl+B)").color(dim).size(12.0));
        });
    }
}

impl eframe::App for NsrApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        self.process_session_events();
        self.drain_output_receivers();

        // Atalhos globais
        let (close_active, open_new, toggle_vault, toggle_settings) = ctx.input_mut(|i| (
            i.consume_key(Modifiers::CTRL, Key::W),
            i.consume_key(Modifiers::CTRL, Key::T),
            i.consume_key(Modifiers::CTRL, Key::B),
            i.consume_key(Modifiers::CTRL, Key::Comma),
        ));
        if close_active {
            if let Some(id) = self.active_tab { self.close_tab(id); }
        }
        if open_new { self.connect_dialog.open = true; }
        if toggle_vault { self.show_vault = !self.show_vault; }
        if toggle_settings { self.settings.open = !self.settings.open; }

        apply_theme(&ctx, &self.theme);

        // Menu bar
        egui::Panel::top("menu_bar").show(&ctx, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("Arquivo", |ui| {
                    if ui.button("Nova conexão  Ctrl+T").clicked() {
                        self.connect_dialog.open = true;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Sair").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Vault", |ui| {
                    if ui.button("Mostrar/ocultar  Ctrl+B").clicked() {
                        self.show_vault = !self.show_vault;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Exportar hosts").clicked() { ui.close(); }
                    if ui.button("Importar hosts").clicked() { ui.close(); }
                });
                ui.menu_button("Ajuda", |ui| {
                    ui.label("NSR-SSH v0.1.0");
                    ui.label("No Subscription Required");
                });
            });
        });

        // Tab bar
        egui::Panel::top("tab_bar").show(&ctx, |ui| {
            if let Some(action) = TabBar::show(ui, &self.tabs, self.active_tab, &self.theme) {
                match action {
                    TabBarAction::Activate(id) => self.active_tab = Some(id),
                    TabBarAction::Close(id) => self.close_tab(id),
                    TabBarAction::New => self.connect_dialog.open = true,
                    TabBarAction::Duplicate(id) => self.duplicate_tab(id),
                    TabBarAction::SplitH(_) | TabBarAction::SplitV(_) => {}
                }
            }
        });

        // Status bar
        egui::Panel::bottom("status_bar").show(&ctx, |ui| {
            let dim = Color32::from_rgb(
                self.theme.ui_text_dim.0, self.theme.ui_text_dim.1, self.theme.ui_text_dim.2,
            );
            ui.horizontal(|ui| {
                ui.label(RichText::new(&self.status_message).size(11.0).color(dim));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("Tema: {} | Sessões: {}", self.theme.name, self.tabs.len()))
                            .size(11.0).color(dim),
                    );
                });
            });
        });

        // Vault sidebar
        if self.show_vault {
            egui::Panel::left("vault_panel")
                .resizable(true)
                .default_size(200.0)
                .size_range(150.0..=400.0)
                .show(&ctx, |ui| {
                    if let Some(action) = self.vault_panel.show(ui, &mut self.hosts, &self.theme) {
                        self.handle_vault_action(action);
                    }
                });
        }

        // Connect dialog
        if let Some(req) = self.connect_dialog.show_window(&ctx) {
            self.connect_to_host(req.host);
        }

        // Settings
        if self.settings.open {
            self.settings.show_window(&ctx, &mut self.theme);
        }

        // Central panel
        egui::CentralPanel::default().show_inside(ui, |ui| {
            if self.tabs.is_empty() {
                self.show_welcome(ui);
            } else if let Some(active_id) = self.active_tab {
                if let Some(tab_idx) = self.tabs.iter().position(|t| t.id == active_id) {
                    let active_pane = self.tabs[tab_idx].active_pane;
                    let pane_tree = self.tabs[tab_idx].pane_tree.clone();
                    let font_size = self.settings.font_size;

                    let new_active = Self::render_pane_tree(
                        ui, &pane_tree, active_pane,
                        &mut self.terminal_buffers, &self.theme, font_size,
                        &self.session_manager, &self.rt,
                    );

                    if let Some(na) = new_active {
                        self.tabs[tab_idx].active_pane = na;
                    }
                }
            }
        });

        ctx.request_repaint();
    }
}

fn apply_theme(ctx: &Context, theme: &Theme) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = Color32::from_rgb(theme.ui_background.0, theme.ui_background.1, theme.ui_background.2);
    visuals.window_fill = Color32::from_rgb(theme.ui_surface.0, theme.ui_surface.1, theme.ui_surface.2);
    visuals.extreme_bg_color = Color32::from_rgb(theme.ui_background.0, theme.ui_background.1, theme.ui_background.2);
    visuals.override_text_color = Some(Color32::from_rgb(theme.ui_text.0, theme.ui_text.1, theme.ui_text.2));
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(
        1.0,
        Color32::from_rgb(theme.ui_border.0, theme.ui_border.1, theme.ui_border.2),
    );
    ctx.set_visuals(visuals);
}

fn key_to_bytes(key: Key, _modifiers: Modifiers) -> Vec<u8> {
    match key {
        Key::Enter => vec![b'\r'],
        Key::Backspace => vec![0x7f],
        Key::Tab => vec![b'\t'],
        Key::Escape => vec![0x1b],
        Key::ArrowUp => vec![0x1b, b'[', b'A'],
        Key::ArrowDown => vec![0x1b, b'[', b'B'],
        Key::ArrowRight => vec![0x1b, b'[', b'C'],
        Key::ArrowLeft => vec![0x1b, b'[', b'D'],
        Key::Home => vec![0x1b, b'[', b'H'],
        Key::End => vec![0x1b, b'[', b'F'],
        Key::Delete => vec![0x1b, b'[', b'3', b'~'],
        Key::PageUp => vec![0x1b, b'[', b'5', b'~'],
        Key::PageDown => vec![0x1b, b'[', b'6', b'~'],
        _ => vec![],
    }
}
