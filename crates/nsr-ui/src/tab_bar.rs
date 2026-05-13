use egui::{Color32, RichText, Ui};
use nsr_theme::Theme;
use uuid::Uuid;
use crate::pane::Tab;

pub struct TabBar;

impl TabBar {
    pub fn show(
        ui: &mut Ui,
        tabs: &[Tab],
        active_tab: Option<Uuid>,
        theme: &Theme,
    ) -> Option<TabBarAction> {
        let mut action = None;

        let accent = Color32::from_rgb(theme.ui_accent.0, theme.ui_accent.1, theme.ui_accent.2);
        let tab_active = Color32::from_rgb(theme.ui_tab_active.0, theme.ui_tab_active.1, theme.ui_tab_active.2);
        let tab_inactive = Color32::from_rgb(theme.ui_tab_inactive.0, theme.ui_tab_inactive.1, theme.ui_tab_inactive.2);
        let text = Color32::from_rgb(theme.ui_text.0, theme.ui_text.1, theme.ui_text.2);
        let text_dim = Color32::from_rgb(theme.ui_text_dim.0, theme.ui_text_dim.1, theme.ui_text_dim.2);

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;

            for tab in tabs {
                let is_active = active_tab == Some(tab.id);
                let bg = if is_active { tab_active } else { tab_inactive };
                let fg = if is_active { text } else { text_dim };
                let tab_id = tab.id;
                let title = tab.title.clone();

                egui::Frame::new()
                    .fill(bg)
                    .inner_margin(egui::Margin::symmetric(10, 4))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let title_resp = ui.add(
                                egui::Label::new(RichText::new(&title).color(fg).size(13.0))
                                    .sense(egui::Sense::click()),
                            );

                            if title_resp.clicked() {
                                action = Some(TabBarAction::Activate(tab_id));
                            }

                            title_resp.context_menu(|ui| {
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

                            if ui.add(
                                egui::Label::new(RichText::new(" ×").color(text_dim).size(13.0))
                                    .sense(egui::Sense::click()),
                            ).clicked() {
                                action = Some(TabBarAction::Close(tab_id));
                            }
                        });
                    });

                ui.separator();
            }

            if ui.add(
                egui::Button::new(RichText::new(" + ").color(accent))
                    .frame(false)
            ).clicked() {
                action = Some(TabBarAction::New);
            }
        });

        action
    }
}

#[derive(Debug, Clone)]
pub enum TabBarAction {
    Activate(Uuid),
    Close(Uuid),
    New,
    Duplicate(Uuid),
    SplitH(Uuid),
    SplitV(Uuid),
}
