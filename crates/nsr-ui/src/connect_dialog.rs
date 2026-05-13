use egui::Context;
use nsr_vault::Host;

pub struct ConnectDialog {
    pub open: bool,
    pub hostname: String,
    pub user: String,
    pub port: String,
    pub identity_file: String,
    pub password: String,
    pub use_password: bool,
}

impl ConnectDialog {
    pub fn new() -> Self {
        Self {
            open: false,
            hostname: String::new(),
            user: std::env::var("USER").unwrap_or_else(|_| "root".into()),
            port: "22".into(),
            identity_file: "~/.ssh/id_rsa".into(),
            password: String::new(),
            use_password: false,
        }
    }

    pub fn show_window(&mut self, ctx: &Context) -> Option<ConnectRequest> {
        if !self.open {
            return None;
        }

        let mut result = None;
        let mut should_close = false;

        egui::Window::new("Conectar")
            .resizable(false)
            .min_width(350.0)
            .show(ctx, |ui| {
                egui::Grid::new("connect_form")
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Host:");
                        ui.text_edit_singleline(&mut self.hostname);
                        ui.end_row();

                        ui.label("User:");
                        ui.text_edit_singleline(&mut self.user);
                        ui.end_row();

                        ui.label("Port:");
                        ui.text_edit_singleline(&mut self.port);
                        ui.end_row();

                        ui.label("Auth:");
                        ui.checkbox(&mut self.use_password, "Senha");
                        ui.end_row();

                        if self.use_password {
                            ui.label("Senha:");
                            ui.add(egui::TextEdit::singleline(&mut self.password).password(true));
                            ui.end_row();
                        } else {
                            ui.label("Chave:");
                            ui.text_edit_singleline(&mut self.identity_file);
                            ui.end_row();
                        }
                    });

                ui.separator();
                ui.horizontal(|ui| {
                    let can_connect = !self.hostname.is_empty() && !self.user.is_empty();
                    if ui.add_enabled(can_connect, egui::Button::new("Conectar")).clicked() {
                        result = Some(ConnectRequest {
                            host: Host {
                                id: uuid::Uuid::new_v4(),
                                alias: self.hostname.clone(),
                                hostname: self.hostname.clone(),
                                user: self.user.clone(),
                                port: self.port.parse().unwrap_or(22),
                                identity_file: if self.use_password {
                                    None
                                } else {
                                    Some(self.identity_file.clone())
                                },
                                tags: vec![],
                                description: None,
                            },
                            password: if self.use_password {
                                Some(self.password.clone())
                            } else {
                                None
                            },
                        });
                        should_close = true;
                    }
                    if ui.button("Cancelar").clicked() {
                        should_close = true;
                    }
                });
            });

        if should_close { self.open = false; }
        result
    }
}

pub struct ConnectRequest {
    pub host: Host,
    pub password: Option<String>,
}
