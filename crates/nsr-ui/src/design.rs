// Design tokens — paleta e medidas do NSR-SSH
use egui::{Color32, FontId, Margin, Stroke};
use egui::epaint::{CornerRadius, Shadow};

pub struct Ds;

// Helper para criar CornerRadius uniforme
pub fn cr(r: u8) -> CornerRadius { CornerRadius::same(r) }
// Helper para criar Margin uniforme (i8)
pub fn margin(v: i8) -> Margin { Margin { left: v, right: v, top: v, bottom: v } }
pub fn margin_xy(x: i8, y: i8) -> Margin { Margin { left: x, right: x, top: y, bottom: y } }

impl Ds {
    // ── Background layers ─────────────────────────────────────────────────────
    pub const BG_BASE: Color32    = Color32::from_rgb(13, 13, 18);
    pub const BG_PANEL: Color32   = Color32::from_rgb(18, 18, 26);
    pub const BG_SURFACE: Color32 = Color32::from_rgb(24, 24, 36);
    pub const BG_HOVER: Color32   = Color32::from_rgb(32, 32, 48);
    pub const BG_ACTIVE: Color32  = Color32::from_rgb(38, 38, 58);
    pub const BG_TAB_ACTIVE: Color32 = Color32::from_rgb(22, 22, 34);

    // ── Text ─────────────────────────────────────────────────────────────────
    pub const TEXT_PRIMARY: Color32   = Color32::from_rgb(225, 225, 235);
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(140, 140, 165);
    pub const TEXT_MUTED: Color32     = Color32::from_rgb(75, 75, 100);

    // ── Accent ────────────────────────────────────────────────────────────────
    pub const ACCENT: Color32       = Color32::from_rgb(130, 100, 255);
    pub const ACCENT_DIM: Color32   = Color32::from_rgb(55, 42, 110);
    pub const GREEN: Color32        = Color32::from_rgb(68, 210, 120);
    pub const RED: Color32          = Color32::from_rgb(240, 80, 80);
    pub const YELLOW: Color32       = Color32::from_rgb(240, 180, 60);

    // ── Borders ───────────────────────────────────────────────────────────────
    pub const BORDER: Color32       = Color32::from_rgb(40, 40, 60);
    pub const BORDER_FOCUS: Color32 = Color32::from_rgb(130, 100, 255);

    // ── Spacing ───────────────────────────────────────────────────────────────
    pub const SPACE_XS: f32 = 4.0;
    pub const SPACE_SM: f32 = 8.0;
    pub const SPACE_MD: f32 = 14.0;
    pub const SPACE_LG: f32 = 20.0;
    pub const SPACE_XL: f32 = 32.0;

    // ── Font sizes ────────────────────────────────────────────────────────────
    pub const FONT_XS: f32   = 10.0;
    pub const FONT_SM: f32   = 12.0;
    pub const FONT_MD: f32   = 13.5;
    pub const FONT_LG: f32   = 16.0;

    // ── Dimensions ────────────────────────────────────────────────────────────
    pub const SIDEBAR_W: f32 = 220.0;
    pub const TAB_H: f32     = 36.0;
    pub const STATUS_H: f32  = 24.0;

    // ── Rounding helpers (u8 for CornerRadius) ────────────────────────────────
    pub const R_SM: u8   = 4;
    pub const R_MD: u8   = 8;
    pub const R_LG: u8   = 12;
    pub const R_PILL: u8 = 100;

    pub fn border() -> Stroke { Stroke::new(1.0, Self::BORDER) }
    pub fn border_focus() -> Stroke { Stroke::new(1.5, Self::BORDER_FOCUS) }
    pub fn font_mono(size: f32) -> FontId { FontId::monospace(size) }

    pub fn apply_global_visuals(ctx: &egui::Context) {
        let mut v = egui::Visuals::dark();

        v.panel_fill             = Self::BG_BASE;
        v.window_fill            = Self::BG_SURFACE;
        v.extreme_bg_color       = Self::BG_BASE;
        v.override_text_color    = Some(Self::TEXT_PRIMARY);
        v.selection.bg_fill      = Self::ACCENT_DIM;
        v.selection.stroke       = Stroke::new(1.0, Self::ACCENT);

        v.widgets.noninteractive.bg_fill   = Self::BG_SURFACE;
        v.widgets.noninteractive.bg_stroke = Self::border();
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Self::TEXT_SECONDARY);
        v.widgets.noninteractive.corner_radius = cr(Self::R_SM);

        v.widgets.inactive.bg_fill    = Self::BG_SURFACE;
        v.widgets.inactive.bg_stroke  = Self::border();
        v.widgets.inactive.fg_stroke  = Stroke::new(1.0, Self::TEXT_SECONDARY);
        v.widgets.inactive.corner_radius = cr(Self::R_SM);

        v.widgets.hovered.bg_fill    = Self::BG_HOVER;
        v.widgets.hovered.bg_stroke  = Self::border_focus();
        v.widgets.hovered.fg_stroke  = Stroke::new(1.0, Self::TEXT_PRIMARY);
        v.widgets.hovered.corner_radius = cr(Self::R_SM);

        v.widgets.active.bg_fill    = Self::BG_ACTIVE;
        v.widgets.active.bg_stroke  = Self::border_focus();
        v.widgets.active.fg_stroke  = Stroke::new(1.5, Self::ACCENT);
        v.widgets.active.corner_radius = cr(Self::R_SM);

        v.widgets.open.bg_fill   = Self::BG_ACTIVE;
        v.widgets.open.bg_stroke = Self::border_focus();
        v.widgets.open.corner_radius = cr(Self::R_SM);

        v.window_corner_radius = cr(Self::R_MD);
        v.menu_corner_radius   = cr(Self::R_MD);
        v.popup_shadow = Shadow {
            blur: 20,
            spread: 2,
            offset: [0, 4].into(),
            color: Color32::from_black_alpha(120),
        };
        v.window_shadow = v.popup_shadow;

        ctx.set_visuals(v);
    }
}
