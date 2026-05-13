use egui::{Color32, FontId, Pos2, Rect, Vec2};
use nsr_theme::Theme;
use uuid::Uuid;

const MAX_SCROLLBACK: usize = 10_000;

pub struct TerminalBuffer {
    pub session_id: Uuid,
    pub cols: usize,
    pub rows: usize,
    pub vt_parser: vt100::Parser,
}

impl TerminalBuffer {
    pub fn new(session_id: Uuid, cols: usize, rows: usize) -> Self {
        Self {
            session_id,
            cols,
            rows,
            vt_parser: vt100::Parser::new(rows as u16, cols as u16, MAX_SCROLLBACK),
        }
    }

    pub fn process(&mut self, data: &[u8]) {
        self.vt_parser.process(data);
    }

    pub fn resize(&mut self, cols: usize, rows: usize) {
        self.cols = cols;
        self.rows = rows;
        self.vt_parser.screen_mut().set_size(rows as u16, cols as u16);
    }
}

pub struct TerminalWidget<'a> {
    pub buffer: &'a mut TerminalBuffer,
    pub theme: &'a Theme,
    pub font_size: f32,
    pub is_active: bool,
}

impl<'a> TerminalWidget<'a> {
    pub fn new(buffer: &'a mut TerminalBuffer, theme: &'a Theme) -> Self {
        Self { buffer, theme, font_size: 14.0, is_active: false }
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.is_active = active;
        self
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let screen = self.buffer.vt_parser.screen();

        // vt100 Screen uses .rows() which returns an iterator
        // use stored cols/rows instead
        let rows = self.buffer.rows;
        let cols = self.buffer.cols;

        let char_w = self.font_size * 0.6;
        let char_h = self.font_size * 1.2;
        let desired = Vec2::new(cols as f32 * char_w, rows as f32 * char_h);

        let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click_and_drag());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter_at(rect);
            let bg = theme_color_to_egui(&self.theme.term_background);
            painter.rect_filled(rect, 0.0, bg);

            let font_id = FontId::monospace(self.font_size);

            for row_idx in 0..rows {
                for col_idx in 0..cols {
                    let cell = screen.cell(row_idx as u16, col_idx as u16);
                    let Some(cell) = cell else { continue };

                    let contents = cell.contents();
                    let ch = contents.chars().next().unwrap_or(' ');

                    let x = rect.left() + col_idx as f32 * char_w;
                    let y = rect.top() + row_idx as f32 * char_h;
                    let cell_rect = Rect::from_min_size(
                        Pos2::new(x, y),
                        Vec2::new(char_w, char_h),
                    );

                    let bg_color = vt_color_to_egui(cell.bgcolor(), self.theme, false);
                    let default_bg = theme_color_to_egui(&self.theme.term_background);
                    if bg_color != default_bg {
                        painter.rect_filled(cell_rect, 0.0, bg_color);
                    }

                    if ch != ' ' {
                        let fg_color = vt_color_to_egui(cell.fgcolor(), self.theme, cell.bold());
                        painter.text(
                            Pos2::new(x, y),
                            egui::Align2::LEFT_TOP,
                            ch.to_string(),
                            font_id.clone(),
                            fg_color,
                        );
                    }
                }
            }

            // Cursor
            if self.is_active {
                let cursor = screen.cursor_position();
                let cy = (cursor.0 as usize).min(rows.saturating_sub(1));
                let cx = (cursor.1 as usize).min(cols.saturating_sub(1));
                let x = rect.left() + cx as f32 * char_w;
                let y = rect.top() + cy as f32 * char_h;
                let cursor_rect = Rect::from_min_size(
                    Pos2::new(x, y),
                    Vec2::new(char_w, char_h),
                );
                let cursor_color = theme_color_to_egui(&self.theme.term_cursor);
                painter.rect_filled(cursor_rect, 1.0, cursor_color.linear_multiply(0.7));
            }
        }

        response
    }
}

fn vt_color_to_egui(color: vt100::Color, theme: &Theme, bold: bool) -> Color32 {
    match color {
        vt100::Color::Default => theme_color_to_egui(&theme.term_foreground),
        vt100::Color::Idx(i) => {
            let idx = if bold && i < 8 { i + 8 } else { i } as usize;
            if idx < 16 {
                let c = &theme.ansi[idx];
                Color32::from_rgb(c.0, c.1, c.2)
            } else {
                xterm256_to_egui(idx as u8)
            }
        }
        vt100::Color::Rgb(r, g, b) => Color32::from_rgb(r, g, b),
    }
}

fn theme_color_to_egui(c: &nsr_theme::Color) -> Color32 {
    Color32::from_rgb(c.0, c.1, c.2)
}

fn xterm256_to_egui(idx: u8) -> Color32 {
    if idx < 16 {
        return Color32::WHITE;
    }
    if idx >= 232 {
        let v = 8 + (idx - 232) as u32 * 10;
        let v = v.min(255) as u8;
        return Color32::from_rgb(v, v, v);
    }
    let i = idx - 16;
    let b = i % 6;
    let g = (i / 6) % 6;
    let r = i / 36;
    let to_val = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
    Color32::from_rgb(to_val(r), to_val(g), to_val(b))
}
