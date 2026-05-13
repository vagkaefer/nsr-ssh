use egui::{Color32, Context, RichText, Stroke, Vec2};
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

    pub fn show_window(&mut self, ctx: &Context, current_theme: &mut Theme) {
        if !self.open {
            return;
        }

        // Overlay escuro
        egui::Area::new(egui::Id::new("settings_overlay"))
            .fixed_pos(egui::pos2(0.0, 0.0))
            .order(egui::Order::Background)
            .show(ctx, |ui| {
                let screen = ctx.content_rect();
                ui.painter().rect_filled(
                    screen,
                    egui::epaint::CornerRadius::ZERO,
                    Color32::from_black_alpha(140),
                );
            });

        let mut should_close = false;

        egui::Window::new("##settings_dialog")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .fixed_size(Vec2::new(640.0, 440.0))
            .frame(
                egui::Frame::window(&ctx.global_style())
                    .fill(Ds::BG_SURFACE)
                    .stroke(Stroke::new(1.0, Ds::BORDER))
                    .corner_radius(Ds::R_LG)
                    .inner_margin(egui::Margin::ZERO),
            )
            .show(ctx, |ui| {
                // ── Titlebar ───────────────────────────────────────────────
                egui::Frame::new()
                    .fill(Ds::BG_PANEL)
                    .inner_margin(egui::Margin::symmetric(Ds::SPACE_LG as i8, Ds::SPACE_MD as i8))
                    .show(ui, |ui| {
                        ui.set_min_size(Vec2::new(640.0, 0.0));
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Configurações")
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
                    });

                // ── Layout: sidebar + content ──────────────────────────────
                ui.horizontal(|ui| {
                    // Sidebar
                    egui::Frame::new()
                        .fill(Ds::BG_PANEL)
                        .inner_margin(egui::Margin::symmetric(Ds::SPACE_SM as i8, Ds::SPACE_MD as i8))
                        .show(ui, |ui| {
                            ui.set_min_size(Vec2::new(148.0, 380.0));
                            ui.set_max_size(Vec2::new(148.0, 380.0));
                            ui.vertical(|ui| {
                                settings_nav(ui, "Aparência", Section::Appearance, &mut self.active_section);
                                settings_nav(ui, "Atalhos",   Section::Shortcuts,  &mut self.active_section);
                                settings_nav(ui, "SSH",        Section::Ssh,        &mut self.active_section);
                                settings_nav(ui, "Plugins",    Section::Plugins,    &mut self.active_section);
                            });
                        });

                    // Conteúdo
                    egui::Frame::new()
                        .fill(Ds::BG_SURFACE)
                        .inner_margin(egui::Margin::same(Ds::SPACE_LG as i8))
                        .show(ui, |ui| {
                            ui.set_min_size(Vec2::new(472.0, 380.0));
                            ui.set_max_size(Vec2::new(472.0, 380.0));
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
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

        if should_close { self.open = false; }
    }

    fn section_appearance(&mut self, ui: &mut egui::Ui, current_theme: &mut Theme) {
        section_title(ui, "Aparência");

        setting_row(ui, "Tema", |ui| {
            let themes = load_user_themes();
            egui::ComboBox::from_id_salt("theme_selector")
                .selected_text(&self.selected_theme_name)
                .show_ui(ui, |ui| {
                    for theme in &themes {
                        let name = theme.name.clone();
                        if ui.selectable_value(&mut self.selected_theme_name, name.clone(), &name).clicked() {
                            *current_theme = theme.clone();
                        }
                    }
                });
        });

        setting_row(ui, "Tamanho da fonte", |ui| {
            ui.add(egui::Slider::new(&mut self.font_size, 10.0..=22.0)
                .suffix("px")
                .step_by(0.5));
        });

        setting_row(ui, "Linhas de histórico", |ui| {
            let mut val = self.scrollback_lines as f64;
            if ui.add(egui::Slider::new(&mut val, 500.0..=50_000.0)
                .suffix(" linhas")
                .step_by(500.0)
                .logarithmic(true)
            ).changed() {
                self.scrollback_lines = val as usize;
            }
        });

        ui.add_space(Ds::SPACE_XS);
        ui.label(
            RichText::new("Histórico: use scroll do mouse para subir. Ctrl+Shift+S salva o conteúdo.")
                .color(Ds::TEXT_MUTED)
                .size(Ds::FONT_SM),
        );

        ui.add_space(Ds::SPACE_SM);
        ui.label(
            RichText::new("Temas customizados: coloque .toml em ~/.config/nsr-ssh/themes/")
                .color(Ds::TEXT_MUTED)
                .size(Ds::FONT_SM),
        );
    }
}

fn section_shortcuts(ui: &mut egui::Ui) {
    section_title(ui, "Atalhos de Teclado");

    let shortcuts = [
        ("Nova conexão",        "Ctrl+T"),
        ("Fechar aba",          "Ctrl+W"),
        ("Split horizontal",    "Ctrl+Shift+\\"),
        ("Split vertical",      "Ctrl+Shift+-"),
        ("Mostrar/ocultar Vault","Ctrl+B"),
        ("Configurações",       "Ctrl+,"),
        ("Próxima aba",         "Ctrl+Tab"),
        ("Aba anterior",        "Ctrl+Shift+Tab"),
    ];

    for (action, key) in shortcuts {
        ui.horizontal(|ui| {
            ui.label(RichText::new(action).color(Ds::TEXT_PRIMARY).size(Ds::FONT_MD));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                kbd_chip(ui, key);
            });
        });
        ui.separator();
    }
}

fn section_ssh(ui: &mut egui::Ui) {
    section_title(ui, "Conexão SSH");
    setting_row(ui, "Reconexão automática", |ui| {
        ui.label(RichText::new("✓ Ativada").color(Ds::GREEN).size(Ds::FONT_MD));
    });
    setting_row(ui, "Tentativas", |ui| {
        ui.label(RichText::new("3").color(Ds::TEXT_PRIMARY));
    });
    setting_row(ui, "Intervalo", |ui| {
        ui.label(RichText::new("3 segundos").color(Ds::TEXT_PRIMARY));
    });
    setting_row(ui, "Verificar known_hosts", |ui| {
        ui.label(RichText::new("em breve").color(Ds::TEXT_MUTED).size(Ds::FONT_SM));
    });
}

fn section_plugins(ui: &mut egui::Ui) {
    section_title(ui, "Plugins");

    ui.add_space(Ds::SPACE_SM);

    // Badge "em breve"
    egui::Frame::new()
        .fill(Ds::BG_HOVER)
        .stroke(Stroke::new(1.0, Ds::BORDER))
        .corner_radius(Ds::R_MD)
        .inner_margin(egui::Margin::same(Ds::SPACE_MD as i8))
        .show(ui, |ui| {
            ui.label(RichText::new("🔌  Sistema de Plugins").color(Ds::TEXT_PRIMARY).size(Ds::FONT_MD).strong());
            ui.add_space(Ds::SPACE_XS);
            ui.label(RichText::new("Disponível na v0.2").color(Ds::YELLOW).size(Ds::FONT_SM));
            ui.add_space(Ds::SPACE_SM);
            ui.label(RichText::new("• Plugins WASM (Extism) — qualquer linguagem").color(Ds::TEXT_SECONDARY).size(Ds::FONT_SM));
            ui.label(RichText::new("• Scripts Lua 5.4 — automações simples").color(Ds::TEXT_SECONDARY).size(Ds::FONT_SM));
            ui.label(
                RichText::new("• Diretório: ~/.config/nsr-ssh/plugins/")
                    .color(Ds::TEXT_MUTED)
                    .size(Ds::FONT_SM),
            );
        });
}

fn settings_nav(ui: &mut egui::Ui, label: &str, section: Section, active: &mut Section) {
    let is_active = *active == section;
    let resp = ui.add(
        egui::Button::new(
            RichText::new(label)
                .color(if is_active { Ds::ACCENT } else { Ds::TEXT_SECONDARY })
                .size(Ds::FONT_MD),
        )
        .fill(if is_active { Ds::ACCENT_DIM } else { Color32::TRANSPARENT })
        .stroke(Stroke::NONE)
        .corner_radius(Ds::R_SM)
        .min_size(Vec2::new(128.0, 30.0)),
    );
    if resp.clicked() { *active = section; }
}

fn section_title(ui: &mut egui::Ui, title: &str) {
    ui.label(RichText::new(title).color(Ds::TEXT_PRIMARY).size(Ds::FONT_LG).strong());
    ui.add_space(Ds::SPACE_SM);
    ui.add(egui::Separator::default().spacing(Ds::SPACE_SM));
    ui.add_space(Ds::SPACE_SM);
}

fn setting_row(ui: &mut egui::Ui, label: &str, content: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(Ds::TEXT_PRIMARY).size(Ds::FONT_MD));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            content(ui);
        });
    });
    ui.add_space(Ds::SPACE_XS);
}

fn kbd_chip(ui: &mut egui::Ui, key: &str) {
    egui::Frame::new()
        .fill(Ds::BG_PANEL)
        .stroke(Stroke::new(1.0, Ds::BORDER))
        .corner_radius(Ds::R_SM)
        .inner_margin(egui::Margin::symmetric(6, 2))
        .show(ui, |ui| {
            ui.label(
                RichText::new(key)
                    .color(Ds::TEXT_SECONDARY)
                    .size(Ds::FONT_SM)
                    .family(egui::FontFamily::Monospace),
            );
        });
}
