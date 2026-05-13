use egui::{
    Color32, Pos2, Rect, RichText, ScrollArea,
    Sense, Stroke, Ui, Vec2,
};
use egui::epaint::CornerRadius;
use nsr_theme::Theme;
use nsr_vault::Host;
use crate::design::Ds;

pub struct VaultPanel {
    pub search: String,
    pub selected: Option<uuid::Uuid>,
    pub hovered_host: Option<uuid::Uuid>,
}

impl VaultPanel {
    pub fn new() -> Self {
        Self {
            search: String::new(),
            selected: None,
            hovered_host: None,
        }
    }

    pub fn show(&mut self, ui: &mut Ui, hosts: &mut Vec<Host>, _theme: &Theme) -> Option<VaultAction> {
        let mut action = None;

        // Fundo do panel
        let panel_rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(panel_rect, CornerRadius::ZERO, Ds::BG_PANEL);
        ui.painter().line_segment(
            [panel_rect.right_top(), panel_rect.right_bottom()],
            Stroke::new(1.0, Ds::BORDER),
        );

        ui.vertical(|ui| {
            // ── Header ──────────────────────────────────────────────────────
            ui.add_space(Ds::SPACE_MD);
            ui.horizontal(|ui| {
                ui.add_space(Ds::SPACE_MD);
                ui.label(
                    RichText::new("VAULT")
                        .color(Ds::TEXT_MUTED)
                        .size(Ds::FONT_XS)
                        .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(Ds::SPACE_SM);
                    let add_btn = icon_button(ui, "+", "Novo host");
                    if add_btn.clicked() {
                        action = Some(VaultAction::NewBlank);
                    }
                });
            });

            ui.add_space(Ds::SPACE_SM);

            // ── Search ──────────────────────────────────────────────────────
            ui.horizontal(|ui| {
                ui.add_space(Ds::SPACE_SM);
                let _search_resp = ui.add(
                    egui::TextEdit::singleline(&mut self.search)
                        .hint_text("🔍  buscar hosts...")
                        .font(egui::FontId::proportional(Ds::FONT_SM))
                        .desired_width(ui.available_width() - Ds::SPACE_SM * 2.0)
                        .margin(egui::Margin::symmetric(Ds::SPACE_SM as i8, Ds::SPACE_XS as i8)),
                );
                ui.add_space(Ds::SPACE_SM);
            });

            ui.add_space(Ds::SPACE_SM);

            // ── Host list ────────────────────────────────────────────────────
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

            ScrollArea::vertical()
                .max_height(ui.available_height() - 48.0)
                .show(ui, |ui| {
                    ui.add_space(Ds::SPACE_XS);

                    if filtered_ids.is_empty() {
                        ui.add_space(Ds::SPACE_XL);
                        ui.vertical_centered(|ui| {
                            ui.label(RichText::new("󰒃").color(Ds::TEXT_MUTED).size(32.0));
                            ui.add_space(Ds::SPACE_XS);
                            ui.label(
                                RichText::new("Nenhum host encontrado")
                                    .color(Ds::TEXT_MUTED)
                                    .size(Ds::FONT_SM),
                            );
                        });
                    }

                    for &host_id in &filtered_ids {
                        let Some(host) = hosts.iter().find(|h| h.id == host_id) else { continue };
                        let is_selected = self.selected == Some(host.id);
                        let alias = host.alias.clone();
                        let hostname = host.hostname.clone();
                        let user = host.user.clone();
                        let port = host.port;
                        let tags = host.tags.clone();

                        if let Some(a) = draw_host_row(
                            ui,
                            host_id,
                            &alias,
                            &hostname,
                            &user,
                            port,
                            &tags,
                            is_selected,
                        ) {
                            match a {
                                HostRowAction::Select => self.selected = Some(host_id),
                                HostRowAction::Connect => {
                                    if let Some(h) = hosts.iter().find(|h| h.id == host_id) {
                                        action = Some(VaultAction::Connect(h.clone()));
                                    }
                                }
                                HostRowAction::Edit => {
                                    if let Some(h) = hosts.iter().find(|h| h.id == host_id) {
                                        action = Some(VaultAction::Edit(h.clone()));
                                    }
                                }
                                HostRowAction::Duplicate => {
                                    if let Some(h) = hosts.iter().find(|h| h.id == host_id) {
                                        action = Some(VaultAction::Duplicate(h.clone()));
                                    }
                                }
                                HostRowAction::Delete => {
                                    action = Some(VaultAction::Delete(host_id));
                                }
                            }
                        }
                    }

                    ui.add_space(Ds::SPACE_SM);
                });

            // ── Footer ────────────────────────────────────────────────────────
            let footer_rect = Rect::from_min_size(
                Pos2::new(panel_rect.left(), panel_rect.bottom() - 40.0),
                Vec2::new(panel_rect.width(), 40.0),
            );
            ui.painter().rect_filled(footer_rect, CornerRadius::ZERO, Ds::BG_PANEL);
            ui.painter().line_segment(
                [footer_rect.left_top(), footer_rect.right_top()],
                Stroke::new(1.0, Ds::BORDER),
            );

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add_space(Ds::SPACE_SM);
                let exp = ui.add(
                    egui::Button::new(
                        RichText::new("↑ Exportar").color(Ds::TEXT_SECONDARY).size(Ds::FONT_SM),
                    )
                    .fill(Color32::TRANSPARENT)
                    .stroke(Stroke::NONE),
                );
                if exp.clicked() { action = Some(VaultAction::Export); }

                ui.add_space(Ds::SPACE_XS);

                let imp = ui.add(
                    egui::Button::new(
                        RichText::new("↓ Importar").color(Ds::TEXT_SECONDARY).size(Ds::FONT_SM),
                    )
                    .fill(Color32::TRANSPARENT)
                    .stroke(Stroke::NONE),
                );
                if imp.clicked() { action = Some(VaultAction::Import); }
            });
        });

        action
    }
}

fn draw_host_row(
    ui: &mut Ui,
    _id: uuid::Uuid,
    alias: &str,
    hostname: &str,
    user: &str,
    port: u16,
    tags: &[String],
    is_selected: bool,
) -> Option<HostRowAction> {
    let mut action = None;
    let row_h = 52.0;
    let w = ui.available_width();

    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, row_h), Sense::click());

    let bg = if is_selected {
        Ds::BG_ACTIVE
    } else if resp.hovered() {
        Ds::BG_HOVER
    } else {
        Color32::TRANSPARENT
    };

    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, CornerRadius::ZERO, bg);

        // Barra lateral esquerda para selecionados
        if is_selected {
            ui.painter().rect_filled(
                Rect::from_min_size(rect.left_top(), Vec2::new(3.0, row_h)),
                CornerRadius::ZERO,
                Ds::ACCENT,
            );
        }

        // Ícone terminal
        let icon_rect = Rect::from_center_size(
            Pos2::new(rect.left() + Ds::SPACE_MD + 12.0, rect.center().y),
            Vec2::splat(28.0),
        );
        ui.painter().rect_filled(icon_rect, Ds::R_SM, Ds::BG_SURFACE);
        ui.painter().text(
            icon_rect.center(),
            egui::Align2::CENTER_CENTER,
            ">_",
            egui::FontId::monospace(10.0),
            Ds::ACCENT,
        );

        // Alias
        let text_x = icon_rect.right() + Ds::SPACE_SM;
        ui.painter().text(
            Pos2::new(text_x, rect.top() + 10.0),
            egui::Align2::LEFT_TOP,
            alias,
            egui::FontId::proportional(Ds::FONT_MD),
            if is_selected { Ds::TEXT_PRIMARY } else { Ds::TEXT_PRIMARY },
        );

        // user@host:port
        let sub = format!("{}@{}:{}", user, hostname, port);
        ui.painter().text(
            Pos2::new(text_x, rect.top() + 28.0),
            egui::Align2::LEFT_TOP,
            sub,
            egui::FontId::monospace(Ds::FONT_SM),
            Ds::TEXT_SECONDARY,
        );

        // Tags
        let mut tag_x = text_x;
        for tag in tags.iter().take(2) {
            let tag_text = format!("#{}", tag);
            let galley = ui.painter().layout_no_wrap(
                tag_text.clone(),
                egui::FontId::proportional(Ds::FONT_XS),
                Ds::ACCENT,
            );
            let tag_w = galley.size().x + 8.0;
            let tag_rect = Rect::from_min_size(
                Pos2::new(tag_x, rect.bottom() - 14.0),
                Vec2::new(tag_w, 12.0),
            );
            // não cabe na linha, parar
            if tag_rect.right() > rect.right() - Ds::SPACE_SM { break; }
            ui.painter().rect_filled(tag_rect, CornerRadius::same(2), Ds::ACCENT_DIM);
            ui.painter().galley(
                Pos2::new(tag_x + 4.0, rect.bottom() - 14.0),
                galley,
                Ds::ACCENT,
            );
            tag_x += tag_w + Ds::SPACE_XS;
        }
    }

    // Context menu
    resp.context_menu(|ui| {
        ui.set_min_width(160.0);
        styled_menu_item(ui, "⚡  Conectar", || action = Some(HostRowAction::Connect));
        styled_menu_item(ui, "✏  Editar", || action = Some(HostRowAction::Edit));
        styled_menu_item(ui, "⎘  Duplicar", || action = Some(HostRowAction::Duplicate));
        ui.separator();
        if ui.add(
            egui::Button::new(RichText::new("✕  Remover").color(Ds::RED).size(Ds::FONT_MD))
                .fill(Color32::TRANSPARENT)
                .stroke(Stroke::NONE),
        ).clicked() {
            action = Some(HostRowAction::Delete);
            ui.close();
        }
    });

    if resp.double_clicked() {
        action = Some(HostRowAction::Connect);
    } else if resp.clicked() {
        action = Some(HostRowAction::Select);
    }

    action
}

fn styled_menu_item(ui: &mut Ui, label: &str, mut on_click: impl FnMut()) {
    if ui.add(
        egui::Button::new(RichText::new(label).size(Ds::FONT_MD))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::NONE),
    ).clicked() {
        on_click();
        ui.close();
    }
}

fn icon_button(ui: &mut Ui, icon: &str, tooltip: &str) -> egui::Response {
    let btn = ui.add(
        egui::Button::new(
            RichText::new(icon).color(Ds::TEXT_SECONDARY).size(16.0),
        )
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::NONE),
    );
    btn.on_hover_text(tooltip)
}

pub fn accent_button(ui: &mut Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(
            RichText::new(label).color(Color32::WHITE).size(Ds::FONT_MD).strong(),
        )
        .fill(Ds::ACCENT)
        .stroke(Stroke::NONE)
        .corner_radius(Ds::R_SM),
    )
}

pub fn ghost_button(ui: &mut Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(
            RichText::new(label).color(Ds::TEXT_SECONDARY).size(Ds::FONT_MD),
        )
        .fill(Ds::BG_SURFACE)
        .stroke(Stroke::new(1.0, Ds::BORDER))
        .corner_radius(Ds::R_SM),
    )
}


enum HostRowAction {
    Select,
    Connect,
    Edit,
    Duplicate,
    Delete,
}

#[derive(Debug, Clone)]
pub enum VaultAction {
    Connect(Host),
    Edit(Host),
    NewBlank,
    Save(Host),
    Duplicate(Host),
    Delete(uuid::Uuid),
    Export,
    Import,
}
