use egui::{Color32, RichText, Stroke, Vec2};
use nsr_theme::{Theme, load_user_themes};
use crate::design::Ds;

pub struct SettingsPanel {
    pub open: bool,
    pub font_size: f32,
    pub scrollback_lines: usize,
    pub selected_theme_name: String,
    active_section: Section,
}

#[derive(PartialEq, Clone, Copy)]
enum Section {
    Appearance,
    Shortcuts,
    Ssh,
    Plugins,
}

impl SettingsPanel {
    pub fn new(theme_name: &str) -> Self {
        Self {
            open: false,
            font_size: 14.0,
            scrollback_lines: 5_000,
            selected_theme_name: theme_name.to_string(),
            active_section: Section::Appearance,
        }
    }

    /// Renderiza as configurações ocupando o ui inteiro (CentralPanel).
    pub fn show_as_page(&mut self, ui: &mut egui::Ui, current_theme: &mut Theme) {
        let rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(rect, egui::epaint::CornerRadius::ZERO, Ds::BG_BASE);

        // ── Topbar ────────────────────────────────────────────────────────────
        let topbar_h = 52.0;
        let topbar_rect = egui::Rect::from_min_size(rect.min, Vec2::new(rect.width(), topbar_h));
        ui.painter().rect_filled(topbar_rect, egui::epaint::CornerRadius::ZERO, Ds::BG_PANEL);
        ui.painter().line_segment(
            [topbar_rect.left_bottom(), topbar_rect.right_bottom()],
            Stroke::new(1.0, Ds::BORDER),
        );

        // Título e botão fechar na topbar
        let mut should_close = false;
        ui.scope_builder(egui::UiBuilder::new().max_rect(topbar_rect), |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.add_space(Ds::SPACE_LG);
                ui.label(
                    RichText::new("Configurações")
                        .size(18.0)
                        .color(Ds::TEXT_PRIMARY)
                        .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(Ds::SPACE_MD);
                    let close_btn = ui.add(
                        egui::Button::new(RichText::new("✕").size(14.0).color(Ds::TEXT_MUTED))
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::NONE)
                            .min_size(Vec2::splat(32.0)),
                    );
                    if close_btn.clicked() { should_close = true; }
                    if close_btn.hovered() {
                        ui.painter().rect_filled(close_btn.rect, Ds::R_SM, Ds::BG_HOVER);
                    }
                    ui.add_space(Ds::SPACE_SM);
                    ui.label(RichText::new("Esc para fechar").size(Ds::FONT_XS).color(Ds::TEXT_MUTED));
                });
            });
        });

        // ── Body: sidebar + content ──────────────────────────────────────────
        let body_rect = egui::Rect::from_min_max(
            egui::pos2(rect.min.x, rect.min.y + topbar_h),
            rect.max,
        );
        let sidebar_w = 200.0;
        let sidebar_rect = egui::Rect::from_min_size(body_rect.min, Vec2::new(sidebar_w, body_rect.height()));
        let content_rect = egui::Rect::from_min_max(
            egui::pos2(body_rect.min.x + sidebar_w, body_rect.min.y),
            body_rect.max,
        );

        // Sidebar
        ui.painter().rect_filled(sidebar_rect, egui::epaint::CornerRadius::ZERO, Ds::BG_PANEL);
        ui.painter().line_segment(
            [sidebar_rect.right_top(), sidebar_rect.right_bottom()],
            Stroke::new(1.0, Ds::BORDER),
        );

        ui.scope_builder(egui::UiBuilder::new().max_rect(sidebar_rect), |ui| {
            ui.add_space(Ds::SPACE_LG);
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.add_space(Ds::SPACE_SM);
                nav_item(ui, "Aparência",  Section::Appearance, &mut self.active_section, sidebar_w);
                nav_item(ui, "Atalhos",    Section::Shortcuts,  &mut self.active_section, sidebar_w);
                nav_item(ui, "SSH",        Section::Ssh,        &mut self.active_section, sidebar_w);
                nav_item(ui, "Plugins",    Section::Plugins,    &mut self.active_section, sidebar_w);
            });
        });

        // Content
        ui.scope_builder(egui::UiBuilder::new().max_rect(content_rect), |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add_space(Ds::SPACE_XL);
                    let inner_w = (content_rect.width() - Ds::SPACE_XL * 2.0).min(640.0);
                    ui.add_space(0.0); // força largura
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.set_width(content_rect.width());
                        // centraliza conteúdo
                        let margin = ((content_rect.width() - inner_w) * 0.5).max(Ds::SPACE_XL);
                        egui::Frame::new()
                            .inner_margin(egui::Margin::symmetric(margin as i8, 0))
                            .show(ui, |ui| {
                                match self.active_section {
                                    Section::Appearance => self.section_appearance(ui, current_theme),
                                    Section::Shortcuts  => section_shortcuts(ui),
                                    Section::Ssh        => section_ssh(ui),
                                    Section::Plugins    => section_plugins(ui),
                                }
                            });
                    });
                });
        });

        // Esc fecha
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) || should_close {
            self.open = false;
        }
    }

    fn section_appearance(&mut self, ui: &mut egui::Ui, current_theme: &mut Theme) {
        section_title(ui, "Aparência");

        setting_row(ui, "Tema", "Esquema de cores do terminal e interface", |ui| {
            let themes = load_user_themes();
            egui::ComboBox::from_id_salt("theme_selector")
                .selected_text(&self.selected_theme_name)
                .width(200.0)
                .show_ui(ui, |ui| {
                    for theme in &themes {
                        let name = theme.name.clone();
                        if ui.selectable_value(&mut self.selected_theme_name, name.clone(), &name).clicked() {
                            *current_theme = theme.clone();
                        }
                    }
                });
        });

        setting_row(ui, "Tamanho da fonte", "Tamanho em pixels da fonte monospace do terminal", |ui| {
            ui.add(
                egui::Slider::new(&mut self.font_size, 10.0..=22.0)
                    .suffix("px")
                    .step_by(0.5)
                    .min_decimals(1)
                    .max_decimals(1),
            );
        });

        setting_row(ui, "Histórico (scrollback)", "Número de linhas mantidas em buffer acima da tela visível", |ui| {
            let mut val = self.scrollback_lines as f64;
            if ui.add(
                egui::Slider::new(&mut val, 500.0..=50_000.0)
                    .suffix(" linhas")
                    .step_by(500.0)
                    .logarithmic(true),
            ).changed() {
                self.scrollback_lines = val as usize;
            }
        });

        ui.add_space(Ds::SPACE_LG);
        info_box(ui, "Temas customizados: coloque arquivos .toml em ~/.config/nsr-ssh/themes/\nCtrl+Shift+S salva o conteúdo do terminal em arquivo.");
    }
}

fn section_shortcuts(ui: &mut egui::Ui) {
    section_title(ui, "Atalhos de Teclado");

    let shortcuts: &[(&str, &str, &str)] = &[
        ("Nova conexão",          "Abre dialog de nova sessão SSH",         "Ctrl+T"),
        ("Fechar aba/pane",       "Fecha o pane ativo (ou a aba)",          "Ctrl+W"),
        ("Split horizontal",      "Divide o pane em dois lado a lado",      "Ctrl+Shift+\\"),
        ("Split vertical",        "Divide o pane em dois acima/abaixo",     "Ctrl+Shift+-"),
        ("Mostrar/ocultar Vault", "Sidebar com lista de hosts",             "Ctrl+B"),
        ("Configurações",         "Esta tela",                              "Ctrl+,"),
        ("Próxima aba",           "Navega para a aba à direita",            "Ctrl+Tab"),
        ("Aba anterior",          "Navega para a aba à esquerda",           "Ctrl+Shift+Tab"),
        ("Salvar sessão",         "Salva o conteúdo do terminal em .txt",   "Ctrl+Shift+S"),
        ("Colar",                 "Cola da área de transferência",          "Ctrl+Shift+V"),
    ];

    for (action, desc, key) in shortcuts {
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(*action).color(Ds::TEXT_PRIMARY).size(Ds::FONT_MD));
                ui.label(RichText::new(*desc).color(Ds::TEXT_MUTED).size(Ds::FONT_SM));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                kbd_chip(ui, key);
            });
        });
        ui.add_space(2.0);
        ui.add(egui::Separator::default().spacing(2.0));
    }
}

fn section_ssh(ui: &mut egui::Ui) {
    section_title(ui, "Conexão SSH");

    setting_row(ui, "Reconexão automática", "Tenta reconectar ao detectar queda na sessão", |ui| {
        ui.label(RichText::new("✓ Ativada").color(Ds::GREEN).size(Ds::FONT_MD));
    });
    setting_row(ui, "Tentativas", "Número máximo de tentativas de reconexão", |ui| {
        ui.label(RichText::new("3").color(Ds::TEXT_PRIMARY).size(Ds::FONT_MD));
    });
    setting_row(ui, "Intervalo entre tentativas", "Tempo de espera entre cada tentativa", |ui| {
        ui.label(RichText::new("3 segundos").color(Ds::TEXT_PRIMARY).size(Ds::FONT_MD));
    });
    setting_row(ui, "Vault", "Hosts salvos em ~/.config/nsr-ssh/vault.json", |ui| {
        ui.label(RichText::new("✓ Sincronizado com ~/.ssh/config").color(Ds::GREEN).size(Ds::FONT_SM));
    });

    ui.add_space(Ds::SPACE_LG);
    info_box(ui, "O NSR-SSH sincroniza automaticamente o vault com ~/.ssh/config ao salvar.\nAlterações manuais no ~/.ssh/config são detectadas e importadas automaticamente.");
}

fn section_plugins(ui: &mut egui::Ui) {
    section_title(ui, "Plugins");

    ui.add_space(Ds::SPACE_SM);

    egui::Frame::new()
        .fill(Ds::BG_SURFACE)
        .stroke(Stroke::new(1.0, Ds::BORDER))
        .corner_radius(Ds::R_MD)
        .inner_margin(egui::Margin::same(Ds::SPACE_LG as i8))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                RichText::new("Sistema de Plugins — v0.2")
                    .color(Ds::TEXT_PRIMARY)
                    .size(Ds::FONT_LG)
                    .strong(),
            );
            ui.add_space(Ds::SPACE_XS);
            ui.label(RichText::new("Em desenvolvimento").color(Ds::YELLOW).size(Ds::FONT_SM));
            ui.add_space(Ds::SPACE_MD);

            for item in &[
                ("WASM (Extism)", "Plugins em qualquer linguagem compilável para WASM"),
                ("Scripts Lua 5.4", "Automações simples sem compilação"),
                ("Diretório", "~/.config/nsr-ssh/plugins/"),
            ] {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("•").color(Ds::ACCENT).size(Ds::FONT_MD));
                    ui.add_space(4.0);
                    ui.label(RichText::new(item.0).color(Ds::TEXT_PRIMARY).size(Ds::FONT_MD).strong());
                    ui.add_space(4.0);
                    ui.label(RichText::new(item.1).color(Ds::TEXT_SECONDARY).size(Ds::FONT_MD));
                });
                ui.add_space(Ds::SPACE_XS);
            }
        });
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn nav_item(ui: &mut egui::Ui, label: &str, section: Section, active: &mut Section, sidebar_w: f32) {
    let is_active = *active == section;
    let desired = Vec2::new(sidebar_w - Ds::SPACE_LG * 2.0, 36.0);
    let (rect, resp) = ui.allocate_exact_size(desired, egui::Sense::click());

    if is_active {
        // Barra lateral esquerda
        ui.painter().rect_filled(
            egui::Rect::from_min_size(rect.min, Vec2::new(3.0, rect.height())),
            egui::epaint::CornerRadius::same(2),
            Ds::ACCENT,
        );
        ui.painter().rect_filled(rect, Ds::R_SM, Ds::ACCENT_DIM);
    } else if resp.hovered() {
        ui.painter().rect_filled(rect, Ds::R_SM, Ds::BG_HOVER);
    }

    ui.painter().text(
        egui::pos2(rect.left() + Ds::SPACE_MD, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(Ds::FONT_MD),
        if is_active { Ds::ACCENT } else { Ds::TEXT_SECONDARY },
    );

    if resp.clicked() { *active = section; }
    ui.add_space(2.0);
}

fn section_title(ui: &mut egui::Ui, title: &str) {
    ui.label(RichText::new(title).color(Ds::TEXT_PRIMARY).size(20.0).strong());
    ui.add_space(Ds::SPACE_XS);
    ui.add(egui::Separator::default().spacing(Ds::SPACE_MD));
    ui.add_space(Ds::SPACE_MD);
}

fn setting_row(ui: &mut egui::Ui, label: &str, description: &str, content: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_min_width(220.0);
            ui.label(RichText::new(label).color(Ds::TEXT_PRIMARY).size(Ds::FONT_MD));
            ui.label(RichText::new(description).color(Ds::TEXT_MUTED).size(Ds::FONT_SM));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            content(ui);
        });
    });
    ui.add_space(Ds::SPACE_MD);
    ui.add(egui::Separator::default().spacing(2.0));
    ui.add_space(Ds::SPACE_SM);
}

fn info_box(ui: &mut egui::Ui, text: &str) {
    egui::Frame::new()
        .fill(Ds::BG_HOVER)
        .stroke(Stroke::new(1.0, Ds::BORDER))
        .corner_radius(Ds::R_SM)
        .inner_margin(egui::Margin::same(Ds::SPACE_MD as i8))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new(text).color(Ds::TEXT_MUTED).size(Ds::FONT_SM));
        });
}

fn kbd_chip(ui: &mut egui::Ui, key: &str) {
    egui::Frame::new()
        .fill(Ds::BG_PANEL)
        .stroke(Stroke::new(1.0, Ds::BORDER))
        .corner_radius(Ds::R_SM)
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(
                RichText::new(key)
                    .color(Ds::TEXT_SECONDARY)
                    .size(Ds::FONT_SM)
                    .family(egui::FontFamily::Monospace),
            );
        });
}
