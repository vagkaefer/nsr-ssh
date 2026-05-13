use egui::{Color32, Pos2, Rect, RichText, Stroke, Ui, Vec2};
use egui::epaint::CornerRadius;
use nsr_theme::Theme;
use uuid::Uuid;
use crate::design::Ds;
use crate::pane::Tab;

pub struct TabBar;

impl TabBar {
    pub fn show(
        ui: &mut Ui,
        tabs: &[Tab],
        active_tab: Option<Uuid>,
        _theme: &Theme,
    ) -> Option<TabBarAction> {
        let mut action = None;
        let h = Ds::TAB_H;

        // Fundo da barra de abas
        let bar_rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(bar_rect, CornerRadius::ZERO, Ds::BG_PANEL);
        // linha separadora na base
        ui.painter().line_segment(
            [bar_rect.left_bottom(), bar_rect.right_bottom()],
            Stroke::new(1.0, Ds::BORDER),
        );

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);
            ui.set_min_height(h);

            // Logo compacto
            ui.add_space(Ds::SPACE_SM);
            ui.label(
                RichText::new("NSR")
                    .color(Ds::ACCENT)
                    .strong()
                    .size(13.0),
            );
            ui.add_space(Ds::SPACE_SM);
            ui.painter().line_segment(
                [
                    Pos2::new(ui.cursor().left(), bar_rect.top() + 8.0),
                    Pos2::new(ui.cursor().left(), bar_rect.bottom() - 8.0),
                ],
                Stroke::new(1.0, Ds::BORDER),
            );
            ui.add_space(Ds::SPACE_SM);

            // Abas
            for tab in tabs {
                let is_active = active_tab == Some(tab.id);
                let tab_id = tab.id;
                let title = tab.title.clone();

                if let Some(a) = draw_tab(ui, &title, is_active, tab_id) {
                    action = Some(a);
                }
            }

            // Botão nova aba (+)
            let btn = ui.add_sized(
                Vec2::new(h, h),
                egui::Button::new(
                    RichText::new("+")
                        .color(Ds::TEXT_SECONDARY)
                        .size(18.0),
                )
                .fill(Color32::TRANSPARENT)
                .stroke(Stroke::NONE),
            );
            if btn.clicked() {
                action = Some(TabBarAction::New);
            }
            if btn.hovered() {
                ui.painter().rect_filled(
                    btn.rect.shrink(4.0),
                    Ds::R_SM,
                    Ds::BG_HOVER,
                );
            }

            // Botões à direita: settings
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(Ds::SPACE_SM);
                let gear = ui.add(
                    egui::Button::new(
                        RichText::new("⚙").color(Ds::TEXT_SECONDARY).size(14.0),
                    )
                    .fill(Color32::TRANSPARENT)
                    .stroke(Stroke::NONE),
                );
                if gear.clicked() {
                    action = Some(TabBarAction::OpenSettings);
                }
            });
        });

        action
    }
}

fn draw_tab(ui: &mut Ui, title: &str, is_active: bool, tab_id: Uuid) -> Option<TabBarAction> {
    let mut action = None;
    let h = Ds::TAB_H;
    let padding = Vec2::new(Ds::SPACE_MD, 0.0);

    // Medimos o texto primeiro para calcular a largura
    let title_galley = ui.painter().layout_no_wrap(
        title.to_string(),
        egui::FontId::proportional(Ds::FONT_MD),
        Ds::TEXT_PRIMARY,
    );
    let tab_w = (title_galley.size().x + padding.x * 2.0 + Ds::SPACE_MD + 20.0).max(100.0).min(200.0);

    let (tab_rect, resp) = ui.allocate_exact_size(Vec2::new(tab_w, h), egui::Sense::click());

    let bg = if is_active {
        Ds::BG_TAB_ACTIVE
    } else if resp.hovered() {
        Ds::BG_HOVER
    } else {
        Color32::TRANSPARENT
    };

    // Fundo da aba
    ui.painter().rect_filled(tab_rect, CornerRadius::ZERO, bg);

    // Indicador de aba ativa: barra roxa na base
    if is_active {
        let indicator_rect = Rect::from_min_size(
            Pos2::new(tab_rect.left() + 8.0, tab_rect.bottom() - 2.0),
            Vec2::new(tab_rect.width() - 16.0, 2.0),
        );
        ui.painter().rect_filled(indicator_rect, CornerRadius::same(1), Ds::ACCENT);
    }

    // Ponto de status (verde = conectado)
    let dot_pos = Pos2::new(tab_rect.left() + Ds::SPACE_MD, tab_rect.center().y);
    ui.painter().circle_filled(dot_pos, 4.0, Ds::GREEN);

    // Texto do título
    let text_color = if is_active { Ds::TEXT_PRIMARY } else { Ds::TEXT_SECONDARY };
    let text_x = dot_pos.x + 10.0;
    let text_y = tab_rect.center().y - title_galley.size().y / 2.0;
    ui.painter().galley(Pos2::new(text_x, text_y), title_galley, text_color);

    // Botão X (fechar)
    let close_size = 16.0;
    let close_rect = Rect::from_center_size(
        Pos2::new(tab_rect.right() - Ds::SPACE_SM - close_size / 2.0, tab_rect.center().y),
        Vec2::splat(close_size),
    );
    let close_hovered = ui.rect_contains_pointer(close_rect);
    if is_active || resp.hovered() {
        if close_hovered {
            ui.painter().circle_filled(close_rect.center(), close_size / 2.0 + 1.0, Ds::BG_ACTIVE);
        }
        ui.painter().text(
            close_rect.center(),
            egui::Align2::CENTER_CENTER,
            "×",
            egui::FontId::proportional(13.0),
            if close_hovered { Ds::TEXT_PRIMARY } else { Ds::TEXT_MUTED },
        );
    }

    // Context menu
    resp.context_menu(|ui| {
        ui.set_min_width(160.0);
        if ui.button("Duplicar aba").clicked() {
            action = Some(TabBarAction::Duplicate(tab_id));
            ui.close();
        }
        if ui.button("Split horizontal").clicked() {
            action = Some(TabBarAction::SplitH(tab_id));
            ui.close();
        }
        if ui.button("Split vertical").clicked() {
            action = Some(TabBarAction::SplitV(tab_id));
            ui.close();
        }
        ui.separator();
        if ui.button("Fechar aba").clicked() {
            action = Some(TabBarAction::Close(tab_id));
            ui.close();
        }
    });

    // Lida com cliques
    if resp.clicked() {
        if close_hovered {
            action = Some(TabBarAction::Close(tab_id));
        } else {
            action = Some(TabBarAction::Activate(tab_id));
        }
    }

    action
}

#[derive(Debug, Clone)]
pub enum TabBarAction {
    Activate(Uuid),
    Close(Uuid),
    New,
    Duplicate(Uuid),
    SplitH(Uuid),
    SplitV(Uuid),
    OpenSettings,
}
