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

        let bar_rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(bar_rect, CornerRadius::ZERO, Ds::BG_PANEL);
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
                egui::Button::new(RichText::new("+").color(Ds::TEXT_SECONDARY).size(18.0))
                    .fill(Color32::TRANSPARENT)
                    .stroke(Stroke::NONE),
            );
            if btn.clicked() { action = Some(TabBarAction::New); }
            if btn.hovered() {
                ui.painter().rect_filled(btn.rect.shrink(4.0), Ds::R_SM, Ds::BG_HOVER);
            }

            // Botões à direita: controles de janela + ferramentas
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // ── Controles de janela (sem decoração do SO) ──────────────
                win_close_btn(ui, &mut action);
                win_max_btn(ui, &mut action);
                win_min_btn(ui, &mut action);

                // Separador
                ui.add_space(4.0);
                ui.painter().line_segment(
                    [
                        Pos2::new(ui.cursor().right(), bar_rect.top() + 8.0),
                        Pos2::new(ui.cursor().right(), bar_rect.bottom() - 8.0),
                    ],
                    Stroke::new(1.0, Ds::BORDER),
                );
                ui.add_space(4.0);

                // Ferramentas
                let gear = tab_icon_btn(ui, "⚙", "Configurações  (Ctrl+,)");
                if gear.clicked() { action = Some(TabBarAction::OpenSettings); }

                if !tabs.is_empty() {
                    ui.add_space(2.0);
                    let sv = tab_icon_btn(ui, "⊟", "Split vertical  (Ctrl+Shift+-)");
                    if sv.clicked() {
                        if let Some(id) = active_tab { action = Some(TabBarAction::SplitV(id)); }
                    }
                    ui.add_space(2.0);
                    let sh = tab_icon_btn(ui, "⊞", "Split horizontal  (Ctrl+Shift+\\)");
                    if sh.clicked() {
                        if let Some(id) = active_tab { action = Some(TabBarAction::SplitH(id)); }
                    }
                }
            });
        });

        // Drag na área vazia = mover janela.
        // Só dispara se não há widget do egui sendo arrastado (abas, etc.)
        let no_widget_drag = ui.ctx().dragged_id().is_none();
        let (primary_pressed, is_moving, press_origin, dbl_click_pos) = ui.input(|i| (
            i.pointer.primary_pressed(),
            i.pointer.is_moving(),
            i.pointer.press_origin(),
            if i.pointer.button_double_clicked(egui::PointerButton::Primary) { i.pointer.interact_pos() } else { None },
        ));

        // Inicia move ao pressionar+arrastar na barra (não em widget interativo)
        if primary_pressed && is_moving && no_widget_drag {
            if press_origin.map(|p| bar_rect.contains(p)).unwrap_or(false) {
                action = Some(TabBarAction::DragWindow);
            }
        }

        // Double-click na barra = maximizar toggle
        if let Some(pos) = dbl_click_pos {
            if bar_rect.contains(pos) {
                action = Some(TabBarAction::MaximizeToggle);
            }
        }

        action
    }
}

fn win_close_btn(ui: &mut Ui, action: &mut Option<TabBarAction>) {
    let size = Vec2::splat(32.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let hovered = resp.hovered();
    let fill = if hovered { Color32::from_rgb(220, 53, 69) } else { Color32::TRANSPARENT };
    ui.painter().rect_filled(rect, Ds::R_SM, fill);
    // Desenha X manualmente com duas linhas para garantir visibilidade
    let c = rect.center();
    let r = 5.0_f32;
    let col = if hovered { Color32::WHITE } else { Ds::TEXT_MUTED };
    ui.painter().line_segment([egui::pos2(c.x - r, c.y - r), egui::pos2(c.x + r, c.y + r)], Stroke::new(1.5, col));
    ui.painter().line_segment([egui::pos2(c.x + r, c.y - r), egui::pos2(c.x - r, c.y + r)], Stroke::new(1.5, col));
    if resp.clicked() { *action = Some(TabBarAction::CloseWindow); }
    resp.on_hover_text("Fechar");
}

fn win_max_btn(ui: &mut Ui, action: &mut Option<TabBarAction>) {
    let size = Vec2::splat(32.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let hovered = resp.hovered();
    ui.painter().rect_filled(rect, Ds::R_SM, if hovered { Ds::BG_HOVER } else { Color32::TRANSPARENT });
    let inner = rect.shrink(10.0);
    let col = if hovered { Ds::TEXT_PRIMARY } else { Ds::TEXT_MUTED };
    ui.painter().rect_stroke(inner, egui::epaint::CornerRadius::same(1), Stroke::new(1.5, col), egui::StrokeKind::Inside);
    if resp.clicked() { *action = Some(TabBarAction::MaximizeToggle); }
    resp.on_hover_text("Maximizar / Restaurar");
}

fn win_min_btn(ui: &mut Ui, action: &mut Option<TabBarAction>) {
    let size = Vec2::splat(32.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let hovered = resp.hovered();
    ui.painter().rect_filled(rect, Ds::R_SM, if hovered { Ds::BG_HOVER } else { Color32::TRANSPARENT });
    let c = rect.center();
    let col = if hovered { Ds::TEXT_PRIMARY } else { Ds::TEXT_MUTED };
    ui.painter().line_segment([Pos2::new(c.x - 5.0, c.y + 3.0), Pos2::new(c.x + 5.0, c.y + 3.0)], Stroke::new(1.5, col));
    if resp.clicked() { *action = Some(TabBarAction::Minimize); }
    resp.on_hover_text("Minimizar");
}

fn tab_icon_btn(ui: &mut Ui, icon: &str, tooltip: &str) -> egui::Response {
    let btn = ui.add(
        egui::Button::new(RichText::new(icon).color(Ds::TEXT_SECONDARY).size(15.0))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::NONE)
            .min_size(Vec2::new(28.0, 28.0)),
    );
    btn.on_hover_text(tooltip)
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

    let (tab_rect, resp) = ui.allocate_exact_size(Vec2::new(tab_w, h), egui::Sense::click_and_drag());

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

    // Drag iniciado — notifica app para mostrar zonas de drop
    if resp.drag_started() && !close_hovered {
        action = Some(TabBarAction::StartDrag(tab_id));
    }
    if resp.drag_stopped() {
        action = Some(TabBarAction::EndDrag);
    }

    // Feedback visual durante drag: ghost da aba segue o cursor em layer global
    if resp.dragged() {
        let drag_pos = resp.interact_pointer_pos().unwrap_or(tab_rect.center());
        let ghost_painter = ui.ctx().layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::new("tab_ghost"),
        ));
        let ghost_rect = Rect::from_center_size(drag_pos, Vec2::new(tab_w, h * 0.85));
        ghost_painter.rect_filled(
            ghost_rect,
            CornerRadius::same(4),
            Color32::from_rgba_premultiplied(
                Ds::ACCENT.r(), Ds::ACCENT.g(), Ds::ACCENT.b(), 180,
            ),
        );
        ghost_painter.text(
            drag_pos,
            egui::Align2::CENTER_CENTER,
            title,
            egui::FontId::proportional(Ds::FONT_MD),
            Color32::WHITE,
        );
    }

    // Context menu
    resp.context_menu(|ui| {
        ui.set_min_width(180.0);
        if ui.button("Duplicar aba").clicked() {
            action = Some(TabBarAction::Duplicate(tab_id));
            ui.close();
        }
        if ui.button("Restaurar pane →  nova aba").clicked() {
            action = Some(TabBarAction::DetachPane(tab_id));
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
    StartDrag(Uuid),
    EndDrag,
    DetachPane(Uuid),
    // Controles de janela (sem decoração do SO)
    CloseWindow,
    Minimize,
    MaximizeToggle,
    DragWindow,
}
