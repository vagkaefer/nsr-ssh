use egui::Context;
use nsr_theme::{Theme, load_user_themes};

pub struct SettingsPanel {
    pub open: bool,
    pub font_size: f32,
    pub selected_theme_name: String,
}

impl SettingsPanel {
    pub fn new(theme_name: &str) -> Self {
        Self {
            open: false,
            font_size: 14.0,
            selected_theme_name: theme_name.to_string(),
        }
    }

    pub fn show_window(&mut self, ctx: &Context, current_theme: &mut Theme) {
        if !self.open {
            return;
        }

        let mut open = self.open;
        egui::Window::new("Configurações")
            .open(&mut open)
            .resizable(true)
            .min_width(400.0)
            .show(ctx, |ui| {
                egui::CollapsingHeader::new("Aparência").default_open(true).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Tema:");
                        let themes = load_user_themes();
                        egui::ComboBox::from_id_salt("theme_selector")
                            .selected_text(&self.selected_theme_name)
                            .show_ui(ui, |ui| {
                                for theme in &themes {
                                    let name = theme.name.clone();
                                    if ui.selectable_value(
                                        &mut self.selected_theme_name,
                                        name.clone(),
                                        &name,
                                    ).clicked() {
                                        *current_theme = theme.clone();
                                    }
                                }
                            });
                    });

                    ui.horizontal(|ui| {
                        ui.label("Tamanho da fonte:");
                        ui.add(egui::Slider::new(&mut self.font_size, 10.0..=24.0).suffix("px"));
                    });
                });

                egui::CollapsingHeader::new("Atalhos").default_open(false).show(ui, |ui| {
                    egui::Grid::new("shortcuts").num_columns(2).show(ui, |ui| {
                        ui.label("Nova aba");
                        ui.label("Ctrl+T");
                        ui.end_row();
                        ui.label("Fechar aba");
                        ui.label("Ctrl+W");
                        ui.end_row();
                        ui.label("Split horizontal");
                        ui.label("Ctrl+Shift+\\");
                        ui.end_row();
                        ui.label("Split vertical");
                        ui.label("Ctrl+Shift+-");
                        ui.end_row();
                        ui.label("Vault");
                        ui.label("Ctrl+B");
                        ui.end_row();
                        ui.label("Configurações");
                        ui.label("Ctrl+,");
                        ui.end_row();
                    });
                });

                egui::CollapsingHeader::new("SSH").default_open(false).show(ui, |ui| {
                    ui.label("Reconexão automática: ativada");
                    ui.label("Tentativas: 3");
                    ui.label("Intervalo: 3s");
                });

                egui::CollapsingHeader::new("Plugins").default_open(false).show(ui, |ui| {
                    ui.label("Diretório: ~/.config/nsr-ssh/plugins/");
                    ui.label("Suporte: .wasm (WASM/Extism) e .lua (Lua 5.4)");
                    ui.label("Status: disponível na v0.2");
                });
            });

        self.open = open;
    }
}
