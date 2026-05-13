use egui::{Color32, RichText, ScrollArea, Ui};
use nsr_vault::Host;
use nsr_theme::Theme;

pub struct VaultPanel {
    pub search: String,
    pub selected: Option<uuid::Uuid>,
    pub show_add_dialog: bool,
    pub editing_host: Option<Host>,
}

impl VaultPanel {
    pub fn new() -> Self {
        Self {
            search: String::new(),
            selected: None,
            show_add_dialog: false,
            editing_host: None,
        }
    }

    pub fn show(&mut self, ui: &mut Ui, hosts: &mut Vec<Host>, theme: &Theme) -> Option<VaultAction> {
        let mut action = None;
        let accent = Color32::from_rgb(theme.ui_accent.0, theme.ui_accent.1, theme.ui_accent.2);
        let surface = Color32::from_rgb(theme.ui_surface.0, theme.ui_surface.1, theme.ui_surface.2);
        let text = Color32::from_rgb(theme.ui_text.0, theme.ui_text.1, theme.ui_text.2);
        let text_dim = Color32::from_rgb(theme.ui_text_dim.0, theme.ui_text_dim.1, theme.ui_text_dim.2);

        ui.vertical(|ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("VAULT").color(accent).strong().size(11.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button(RichText::new("+").color(accent)).clicked() {
                        self.show_add_dialog = true;
                        self.editing_host = Some(Host::default());
                    }
                });
            });
            ui.add_space(4.0);

            ui.add(
                egui::TextEdit::singleline(&mut self.search)
                    .hint_text("buscar hosts...")
                    .desired_width(ui.available_width()),
            );

            ui.add_space(4.0);
            ui.separator();

            // Lista de hosts
            let query = self.search.to_lowercase();
            let filtered_ids: Vec<uuid::Uuid> = hosts
                .iter()
                .filter(|h| {
                    query.is_empty()
                        || h.alias.to_lowercase().contains(&query)
                        || h.hostname.to_lowercase().contains(&query)
                        || h.tags.iter().any(|t| t.to_lowercase().contains(&query))
                })
                .map(|h| h.id)
                .collect();

            ScrollArea::vertical().max_height(ui.available_height() - 40.0).show(ui, |ui| {
                if filtered_ids.is_empty() {
                    ui.label(RichText::new("Nenhum host").color(text_dim).italics());
                }

                for &host_id in &filtered_ids {
                    let Some(host) = hosts.iter().find(|h| h.id == host_id) else { continue };
                    let is_selected = self.selected == Some(host.id);
                    let bg = if is_selected { surface } else { Color32::TRANSPARENT };
                    let alias = host.alias.clone();
                    let tags = host.tags.clone();

                    let resp = ui.add(
                        egui::Button::new(
                            RichText::new(format!("  {}", alias))
                                .color(if is_selected { accent } else { text })
                        )
                        .fill(bg)
                        .frame(false)
                        .min_size(egui::vec2(ui.available_width(), 24.0)),
                    );

                    if resp.clicked() {
                        self.selected = Some(host_id);
                    }

                    if resp.double_clicked() {
                        if let Some(h) = hosts.iter().find(|h| h.id == host_id) {
                            action = Some(VaultAction::Connect(h.clone()));
                        }
                    }

                    resp.context_menu(|ui| {
                        if ui.button("Conectar").clicked() {
                            if let Some(h) = hosts.iter().find(|h| h.id == host_id) {
                                action = Some(VaultAction::Connect(h.clone()));
                            }
                            ui.close();
                        }
                        if ui.button("Editar").clicked() {
                            if let Some(h) = hosts.iter().find(|h| h.id == host_id) {
                                action = Some(VaultAction::Edit(h.clone()));
                            }
                            ui.close();
                        }
                        if ui.button("Duplicar").clicked() {
                            if let Some(h) = hosts.iter().find(|h| h.id == host_id) {
                                action = Some(VaultAction::Duplicate(h.clone()));
                            }
                            ui.close();
                        }
                        ui.separator();
                        if ui.button(RichText::new("Remover").color(Color32::RED)).clicked() {
                            action = Some(VaultAction::Delete(host_id));
                            ui.close();
                        }
                    });

                    if !tags.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            ui.add_space(8.0);
                            for tag in &tags {
                                ui.label(
                                    RichText::new(format!("#{}", tag))
                                        .color(text_dim)
                                        .size(10.0),
                                );
                            }
                        });
                    }
                }
            });

            ui.add_space(4.0);
            ui.separator();

            ui.horizontal(|ui| {
                if ui.small_button("Exportar").clicked() {
                    action = Some(VaultAction::Export);
                }
                if ui.small_button("Importar").clicked() {
                    action = Some(VaultAction::Import);
                }
            });
        });

        // Dialog de adicionar/editar host — usa clone para evitar borrow conflict
        if self.show_add_dialog {
            if let Some(mut host) = self.editing_host.clone() {
                let mut open = true;
                let mut save_clicked = false;
                let mut cancel_clicked = false;

                egui::Window::new("Novo Host")
                    .open(&mut open)
                    .resizable(false)
                    .show(ui.ctx(), |ui| {
                        egui::Grid::new("host_form").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                            ui.label("Alias:");
                            ui.text_edit_singleline(&mut host.alias);
                            ui.end_row();

                            ui.label("HostName:");
                            ui.text_edit_singleline(&mut host.hostname);
                            ui.end_row();

                            ui.label("User:");
                            ui.text_edit_singleline(&mut host.user);
                            ui.end_row();

                            ui.label("Port:");
                            let mut port_str = host.port.to_string();
                            if ui.text_edit_singleline(&mut port_str).changed() {
                                host.port = port_str.parse().unwrap_or(22);
                            }
                            ui.end_row();

                            ui.label("IdentityFile:");
                            let mut id_file = host.identity_file.clone().unwrap_or_default();
                            if ui.text_edit_singleline(&mut id_file).changed() {
                                host.identity_file = if id_file.is_empty() { None } else { Some(id_file) };
                            }
                            ui.end_row();

                            ui.label("Descrição:");
                            let mut desc = host.description.clone().unwrap_or_default();
                            if ui.text_edit_singleline(&mut desc).changed() {
                                host.description = if desc.is_empty() { None } else { Some(desc) };
                            }
                            ui.end_row();
                        });

                        ui.separator();
                        ui.horizontal(|ui| {
                            if ui.button("Salvar").clicked() {
                                save_clicked = true;
                            }
                            if ui.button("Cancelar").clicked() {
                                cancel_clicked = true;
                            }
                        });
                    });

                if save_clicked {
                    action = Some(VaultAction::Save(host.clone()));
                    self.show_add_dialog = false;
                    self.editing_host = None;
                } else if cancel_clicked || !open {
                    self.show_add_dialog = false;
                    self.editing_host = None;
                } else {
                    self.editing_host = Some(host);
                }
            }
        }

        action
    }
}

#[derive(Debug, Clone)]
pub enum VaultAction {
    Connect(Host),
    Edit(Host),
    Save(Host),
    Duplicate(Host),
    Delete(uuid::Uuid),
    Export,
    Import,
}
