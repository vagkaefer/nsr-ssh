use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Color(pub u8, pub u8, pub u8);

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self(r, g, b)
    }

    pub fn to_egui(&self) -> [f32; 3] {
        [self.0 as f32 / 255.0, self.1 as f32 / 255.0, self.2 as f32 / 255.0]
    }

    pub fn to_u32_rgb(&self) -> u32 {
        ((self.0 as u32) << 16) | ((self.1 as u32) << 8) | (self.2 as u32)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub author: Option<String>,

    // UI
    pub ui_background: Color,
    pub ui_surface: Color,
    pub ui_border: Color,
    pub ui_accent: Color,
    pub ui_text: Color,
    pub ui_text_dim: Color,
    pub ui_tab_active: Color,
    pub ui_tab_inactive: Color,

    // Terminal
    pub term_background: Color,
    pub term_foreground: Color,
    pub term_cursor: Color,
    pub term_selection: Color,

    // ANSI 16 cores: [normal 0-7, bright 8-15]
    pub ansi: [Color; 16],
}

impl Theme {
    pub fn ansi_color(&self, index: usize) -> &Color {
        &self.ansi[index.min(15)]
    }
}
