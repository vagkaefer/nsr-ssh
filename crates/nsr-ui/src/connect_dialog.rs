use egui::{Color32, Context, RichText, Stroke, Ui, Vec2};
use nsr_vault::Host;
use crate::design::Ds;
use crate::vault_panel::ghost_button;

#[derive(PartialEq, Clone, Copy)]
pub enum AuthTab {
    Key,
    Password,
    Agent,
}

pub struct ConnectDialog {
    pub open: bool,
    host: Host,
    password: String,
    auth_tab: AuthTab,
    port_str: String,
    tags_str: String,
    id_file_str: String,
    desc_str: String,
    ssh_options_str: String,
    show_advanced: bool,
}

pub struct ConnectRequest {
    pub host: Host,
    pub password: Option<String>,
    pub save_to_vault: bool,
    pub connect: bool,
}

impl ConnectDialog {
    pub fn new() -> Self {
        let default_user = std::env::var("USER").unwrap_or_else(|_| "root".into());
        Self {
            open: false,
            host: Host {
                id: uuid::Uuid::new_v4(),
                alias: String::new(),
                hostname: String::new(),
                user: default_user,
                port: 22,
                identity_file: Some("~/.ssh/id_rsa".into()),
                tags: vec![],
                description: None,
                ssh_options: None,
            },
            password: String::new(),
            auth_tab: AuthTab::Key,
            port_str: "22".into(),
            tags_str: String::new(),
            id_file_str: "~/.ssh/id_rsa".into(),
            desc_str: String::new(),
            ssh_options_str: String::new(),
            show_advanced: false,
        }
    }

    pub fn open_blank(&mut self) {
        let default_user = std::env::var("USER").unwrap_or_else(|_| "root".into());
        self.host = Host {
            id: uuid::Uuid::new_v4(),
            alias: String::new(),
            hostname: String::new(),
            user: default_user,
            port: 22,
            identity_file: Some("~/.ssh/id_rsa".into()),
            tags: vec![],
            description: None,
            ssh_options: None,
        };
        self.password = String::new();
        self.auth_tab = AuthTab::Key;
        self.port_str = "22".into();
        self.tags_str = String::new();
        self.id_file_str = "~/.ssh/id_rsa".into();
        self.desc_str = String::new();
        self.ssh_options_str = String::new();
        self.show_advanced = false;
        self.open = true;
    }

    pub fn open_with_host(&mut self, host: &Host) {
        self.host = host.clone();
        self.password = String::new();
        self.auth_tab = if host.identity_file.is_some() { AuthTab::Key } else { AuthTab::Password };
        self.port_str = host.port.to_string();
        self.tags_str = host.tags.join(", ");
        self.id_file_str = host.identity_file.clone().unwrap_or_else(|| "~/.ssh/id_rsa".into());
        self.desc_str = host.description.clone().unwrap_or_default();
        self.ssh_options_str = host.ssh_options.clone().unwrap_or_default();
        self.show_advanced = host.ssh_options.is_some();
        self.open = true;
    }

    pub fn show_window(&mut self, ctx: &Context) -> Option<ConnectRequest> {
        if !self.open {
            return None;
        }

        let mut result = None;
        let mut should_close = false;

        // Overlay escuro
        egui::Area::new(egui::Id::new("connect_overlay"))
            .fixed_pos(egui::pos2(0.0, 0.0))
            .order(egui::Order::Background)
            .show(ctx, |ui| {
                let screen = ctx.content_rect();
                ui.painter().rect_filled(
                    screen,
                    egui::epaint::CornerRadius::ZERO,
                    Color32::from_black_alpha(160),
                );
            });

        egui::Window::new("##connect_dialog")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .fixed_size(Vec2::new(480.0, 0.0))
            .frame(
                egui::Frame::window(&ctx.global_style())
                    .fill(Ds::BG_SURFACE)
                    .stroke(Stroke::new(1.0, Ds::BORDER))
                    .corner_radius(Ds::R_LG)
                    .inner_margin(egui::Margin::same(Ds::SPACE_LG as i8)),
            )
            .show(ctx, |ui| {
                // ── Cabeçalho ────────────────────────────────────────────────
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Nova Conexão SSH")
                            .size(Ds::FONT_LG)
                            .color(Ds::TEXT_PRIMARY)
                            .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(
                            egui::Button::new(RichText::new("×").size(20.0).color(Ds::TEXT_MUTED))
                                .fill(Color32::TRANSPARENT)
                                .stroke(Stroke::NONE),
                        ).clicked() {
                            should_close = true;
                        }
                    });
                });
                ui.add_space(Ds::SPACE_SM);
                ui.add(egui::Separator::default().spacing(Ds::SPACE_SM));
                ui.add_space(Ds::SPACE_SM);

                // ── Nome / Alias ─────────────────────────────────────────────
                field_label(ui, "Nome / Alias");
                ui.add(
                    egui::TextEdit::singleline(&mut self.host.alias)
                        .hint_text("prod-web  (deixe vazio para usar o hostname)")
                        .desired_width(f32::INFINITY)
                        .font(egui::FontId::monospace(Ds::FONT_MD)),
                );
                ui.add_space(Ds::SPACE_SM);

                // ── user @ host : porta ──────────────────────────────────────
                field_label(ui, "Endereço");
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.host.user)
                            .hint_text("ubuntu")
                            .desired_width(110.0)
                            .font(egui::FontId::monospace(Ds::FONT_MD)),
                    );
                    ui.label(RichText::new("@").color(Ds::TEXT_MUTED).size(16.0));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.host.hostname)
                            .hint_text("192.168.1.1 ou host.exemplo.com")
                            .desired_width(220.0)
                            .font(egui::FontId::monospace(Ds::FONT_MD)),
                    );
                    ui.label(RichText::new(":").color(Ds::TEXT_MUTED).size(16.0));
                    if ui.add(
                        egui::TextEdit::singleline(&mut self.port_str)
                            .hint_text("22")
                            .desired_width(50.0)
                            .font(egui::FontId::monospace(Ds::FONT_MD)),
                    ).changed() {
                        self.host.port = self.port_str.parse().unwrap_or(22);
                    }
                });
                ui.add_space(Ds::SPACE_SM);

                // ── Autenticação ─────────────────────────────────────────────
                field_label(ui, "Autenticação");
                ui.horizontal(|ui| {
                    auth_tab_btn(ui, "Chave SSH", self.auth_tab == AuthTab::Key, || {
                        self.auth_tab = AuthTab::Key;
                    });
                    ui.add_space(4.0);
                    auth_tab_btn(ui, "Senha", self.auth_tab == AuthTab::Password, || {
                        self.auth_tab = AuthTab::Password;
                    });
                    ui.add_space(4.0);
                    auth_tab_btn(ui, "SSH Agent", self.auth_tab == AuthTab::Agent, || {
                        self.auth_tab = AuthTab::Agent;
                    });
                });
                ui.add_space(Ds::SPACE_XS);

                match self.auth_tab {
                    AuthTab::Key => {
                        ui.horizontal(|ui| {
                            let changed = ui.add(
                                egui::TextEdit::singleline(&mut self.id_file_str)
                                    .hint_text("~/.ssh/id_rsa")
                                    .desired_width(ui.available_width() - 36.0)
                                    .font(egui::FontId::monospace(Ds::FONT_SM)),
                            ).changed();
                            let browse = ui.add(
                                egui::Button::new(RichText::new("…").size(Ds::FONT_MD).color(Ds::TEXT_SECONDARY))
                                    .fill(Ds::BG_HOVER)
                                    .stroke(Stroke::new(1.0, Ds::BORDER))
                                    .min_size(Vec2::new(28.0, 0.0)),
                            );
                            if browse.clicked() {
                                if let Some(path) = rfd::FileDialog::new()
                                    .set_title("Selecionar chave SSH")
                                    .set_directory(dirs::home_dir().unwrap_or_default().join(".ssh"))
                                    .pick_file()
                                {
                                    self.id_file_str = path.to_string_lossy().into_owned();
                                }
                            }
                            if changed || browse.clicked() {
                                self.host.identity_file = if self.id_file_str.is_empty() {
                                    None
                                } else {
                                    Some(self.id_file_str.clone())
                                };
                            }
                        });
                        self.host.identity_file = if self.id_file_str.is_empty() {
                            None
                        } else {
                            Some(self.id_file_str.clone())
                        };
                    }
                    AuthTab::Password => {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.password)
                                .password(true)
                                .hint_text("senha")
                                .desired_width(f32::INFINITY),
                        );
                        self.host.identity_file = None;
                    }
                    AuthTab::Agent => {
                        ui.label(
                            RichText::new("Usando SSH Agent ($SSH_AUTH_SOCK)")
                                .color(Ds::TEXT_MUTED)
                                .size(Ds::FONT_SM),
                        );
                        self.host.identity_file = None;
                    }
                }
                ui.add_space(Ds::SPACE_SM);

                // ── Tags ─────────────────────────────────────────────────────
                field_label(ui, "Tags  (separadas por vírgula, opcional)");
                if ui.add(
                    egui::TextEdit::singleline(&mut self.tags_str)
                        .hint_text("producao, web, aws")
                        .desired_width(f32::INFINITY),
                ).changed() {
                    self.host.tags = self.tags_str
                        .split([',', ' '])
                        .map(|t| t.trim().to_string())
                        .filter(|t| !t.is_empty())
                        .collect();
                }
                ui.add_space(Ds::SPACE_SM);

                // ── Descrição ────────────────────────────────────────────────
                field_label(ui, "Descrição  (opcional)");
                if ui.add(
                    egui::TextEdit::singleline(&mut self.desc_str)
                        .hint_text("Servidor de produção principal")
                        .desired_width(f32::INFINITY),
                ).changed() {
                    self.host.description = if self.desc_str.is_empty() {
                        None
                    } else {
                        Some(self.desc_str.clone())
                    };
                }
                ui.add_space(Ds::SPACE_SM);

                // ── Opções SSH avançadas ──────────────────────────────────────
                let adv_label = if self.show_advanced { "▾ Opções SSH avançadas" } else { "▸ Opções SSH avançadas" };
                if ui.add(
                    egui::Button::new(RichText::new(adv_label).color(Ds::TEXT_MUTED).size(Ds::FONT_SM))
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::NONE),
                ).clicked() {
                    self.show_advanced = !self.show_advanced;
                }

                if self.show_advanced {
                    ui.add_space(2.0);
                    field_label(ui, "Opções extras  (-o Chave=Valor, uma por linha)");
                    if ui.add(
                        egui::TextEdit::multiline(&mut self.ssh_options_str)
                            .hint_text("StrictHostKeyChecking=no\nUserKnownHostsFile=/dev/null\nRequiredRSASize=256")
                            .desired_width(f32::INFINITY)
                            .desired_rows(3)
                            .font(egui::FontId::monospace(Ds::FONT_SM)),
                    ).changed() {
                        self.host.ssh_options = if self.ssh_options_str.trim().is_empty() {
                            None
                        } else {
                            Some(self.ssh_options_str.clone())
                        };
                    }
                }

                ui.add_space(Ds::SPACE_LG);
                ui.add(egui::Separator::default().spacing(Ds::SPACE_SM));
                ui.add_space(Ds::SPACE_SM);

                // ── Botões ───────────────────────────────────────────────────
                let can_connect = !self.host.hostname.trim().is_empty()
                    && !self.host.user.trim().is_empty();

                ui.horizontal(|ui| {
                    // ⚡ Conectar (sem salvar)
                    if ui.add_enabled(
                        can_connect,
                        egui::Button::new(
                            RichText::new("⚡  Conectar")
                                .color(Color32::WHITE)
                                .size(Ds::FONT_MD)
                                .strong(),
                        )
                        .fill(if can_connect { Ds::ACCENT } else { Ds::BG_ACTIVE })
                        .stroke(Stroke::NONE)
                        .corner_radius(Ds::R_SM)
                        .min_size(Vec2::new(110.0, 32.0)),
                    ).clicked() {
                        result = Some(self.build_request(false, true));
                        should_close = true;
                    }

                    ui.add_space(Ds::SPACE_XS);

                    // 💾 Salvar & Conectar
                    if ui.add_enabled(
                        can_connect,
                        egui::Button::new(
                            RichText::new("💾  Salvar & Conectar")
                                .color(if can_connect { Ds::ACCENT } else { Ds::TEXT_MUTED })
                                .size(Ds::FONT_MD),
                        )
                        .fill(if can_connect { Ds::ACCENT_DIM } else { Ds::BG_PANEL })
                        .stroke(Stroke::new(1.0, if can_connect { Ds::ACCENT } else { Ds::BORDER }))
                        .corner_radius(Ds::R_SM)
                        .min_size(Vec2::new(148.0, 32.0)),
                    ).clicked() {
                        result = Some(self.build_request(true, true));
                        should_close = true;
                    }

                    ui.add_space(Ds::SPACE_XS);

                    // 💾 Somente Salvar (sem conectar)
                    let can_save = !self.host.alias.trim().is_empty()
                        && !self.host.hostname.trim().is_empty();
                    if ui.add_enabled(
                        can_save,
                        egui::Button::new(
                            RichText::new("Salvar")
                                .color(if can_save { Ds::TEXT_PRIMARY } else { Ds::TEXT_MUTED })
                                .size(Ds::FONT_MD),
                        )
                        .fill(Ds::BG_PANEL)
                        .stroke(Stroke::new(1.0, if can_save { Ds::BORDER } else { Ds::BG_ACTIVE }))
                        .corner_radius(Ds::R_SM)
                        .min_size(Vec2::new(72.0, 32.0)),
                    ).clicked() {
                        result = Some(self.build_request(true, false));
                        should_close = true;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ghost_button(ui, "Cancelar").clicked() {
                            should_close = true;
                        }
                    });
                });

                if !can_connect && (!self.host.hostname.is_empty() || !self.host.user.is_empty()) {
                    ui.add_space(Ds::SPACE_XS);
                    ui.label(
                        RichText::new("Preencha usuário e host para conectar")
                            .color(Ds::RED)
                            .size(Ds::FONT_SM),
                    );
                }
            });

        if should_close {
            self.open = false;
        }
        result
    }

    fn build_request(&self, save_to_vault: bool, connect: bool) -> ConnectRequest {
        let mut host = self.host.clone();
        if host.alias.trim().is_empty() {
            host.alias = host.hostname.clone();
        }
        ConnectRequest {
            password: match self.auth_tab {
                AuthTab::Password => Some(self.password.clone()),
                _ => None,
            },
            host,
            save_to_vault,
            connect,
        }
    }
}

fn field_label(ui: &mut Ui, label: &str) {
    ui.label(RichText::new(label).color(Ds::TEXT_SECONDARY).size(Ds::FONT_SM));
    ui.add_space(2.0);
}

fn auth_tab_btn(ui: &mut Ui, label: &str, active: bool, mut on_click: impl FnMut()) {
    if ui.add(
        egui::Button::new(
            RichText::new(label)
                .color(if active { Ds::ACCENT } else { Ds::TEXT_SECONDARY })
                .size(Ds::FONT_SM),
        )
        .fill(if active { Ds::ACCENT_DIM } else { Ds::BG_HOVER })
        .stroke(if active {
            Stroke::new(1.0, Ds::ACCENT)
        } else {
            Stroke::new(1.0, Ds::BORDER)
        })
        .corner_radius(Ds::R_PILL)
        .min_size(Vec2::new(0.0, 24.0)),
    ).clicked() {
        on_click();
    }
}
