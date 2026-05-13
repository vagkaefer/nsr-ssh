use egui::{Color32, FontId, Id, Pos2, Rect, Sense, Vec2};
use nsr_theme::Theme;
use uuid::Uuid;

pub const DEFAULT_SCROLLBACK: usize = 5_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CellPos {
    pub row: usize,
    pub col: usize,
}

#[derive(Debug, Clone, Default)]
pub struct Selection {
    pub start: Option<CellPos>,
    pub end: Option<CellPos>,
    pub active: bool,
}

impl Selection {
    pub fn clear(&mut self) {
        self.start = None;
        self.end = None;
        self.active = false;
    }

    pub fn normalized(&self) -> Option<(CellPos, CellPos)> {
        match (self.start, self.end) {
            (Some(s), Some(e)) => {
                if (s.row, s.col) <= (e.row, e.col) {
                    Some((s, e))
                } else {
                    Some((e, s))
                }
            }
            _ => None,
        }
    }

    pub fn contains(&self, row: usize, col: usize) -> bool {
        if let Some((s, e)) = self.normalized() {
            let pos = (row, col);
            pos >= (s.row, s.col) && pos <= (e.row, e.col)
        } else {
            false
        }
    }

    pub fn is_empty(&self) -> bool {
        match (self.start, self.end) {
            (Some(s), Some(e)) => s == e,
            _ => true,
        }
    }
}

pub struct TerminalBuffer {
    pub session_id: Uuid,
    pub cols: usize,
    pub rows: usize,
    pub vt_parser: vt100::Parser,
    pub scrollback_max: usize,
    pub selection: Selection,
    pub recording: bool,
    pub record_path: Option<std::path::PathBuf>,
    record_file: Option<std::fs::File>,
}

impl TerminalBuffer {
    pub fn new(session_id: Uuid, cols: usize, rows: usize, scrollback_max: usize) -> Self {
        Self {
            session_id,
            cols,
            rows,
            vt_parser: vt100::Parser::new(rows as u16, cols as u16, scrollback_max),
            scrollback_max,
            selection: Selection::default(),
            recording: false,
            record_path: None,
            record_file: None,
        }
    }

    pub fn resize(&mut self, cols: usize, rows: usize) {
        if self.cols == cols && self.rows == rows {
            return;
        }
        self.cols = cols;
        self.rows = rows;
        self.vt_parser.screen_mut().set_size(rows as u16, cols as u16);
    }

    /// Offset de scroll atual (0 = tela normal, >0 = scrollback).
    pub fn scroll_offset(&self) -> usize {
        self.vt_parser.screen().scrollback()
    }

    /// Total máximo de linhas de scrollback configurado.
    pub fn scrollback_capacity(&self) -> usize {
        self.scrollback_max
    }

    /// Rola N linhas para cima (entra no histórico).
    pub fn scroll_up(&mut self, lines: usize) {
        let current = self.scroll_offset();
        let new_offset = current + lines;
        self.vt_parser.screen_mut().set_scrollback(new_offset);
    }

    /// Rola N linhas para baixo (volta ao presente).
    pub fn scroll_down(&mut self, lines: usize) {
        let current = self.scroll_offset();
        let new_offset = current.saturating_sub(lines);
        self.vt_parser.screen_mut().set_scrollback(new_offset);
    }

    /// Volta ao final (tela atual).
    pub fn scroll_to_bottom(&mut self) {
        self.vt_parser.screen_mut().set_scrollback(0);
    }

    /// Vai para o topo do histórico.
    pub fn scroll_to_top(&mut self) {
        self.vt_parser.screen_mut().set_scrollback(usize::MAX);
    }

    /// Quando recebe novo output, volta ao final automaticamente (comportamento padrão).
    pub fn process(&mut self, data: &[u8]) {
        self.vt_parser.process(data);
        // Scroll para o final quando chega novo output
        self.vt_parser.screen_mut().set_scrollback(0);
        if let Some(ref mut f) = self.record_file {
            use std::io::Write;
            let _ = f.write_all(data);
        }
    }

    /// Retorna o texto completo da view atual (respeitando scrollback offset).
    pub fn full_text(&self) -> String {
        let screen = self.vt_parser.screen();
        let rows = screen.size().0 as usize;
        let cols = screen.size().1 as usize;
        let mut out = String::new();
        for r in 0..rows {
            let mut line = String::new();
            for c in 0..cols {
                if let Some(cell) = screen.cell(r as u16, c as u16) {
                    let s = cell.contents();
                    if s.is_empty() { line.push(' '); } else { line.push_str(s); }
                }
            }
            let trimmed = line.trim_end().to_string();
            out.push_str(&trimmed);
            out.push('\n');
        }
        out
    }

    /// Retorna o texto selecionado.
    pub fn selected_text(&self) -> String {
        let Some((s, e)) = self.selection.normalized() else { return String::new() };
        let screen = self.vt_parser.screen();
        let mut out = String::new();
        for r in s.row..=e.row {
            let col_start = if r == s.row { s.col } else { 0 };
            let col_end = if r == e.row { e.col } else { self.cols.saturating_sub(1) };
            for c in col_start..=col_end {
                if let Some(cell) = screen.cell(r as u16, c as u16) {
                    let s = cell.contents();
                    if s.is_empty() { out.push(' '); } else { out.push_str(s); }
                }
            }
            if r < e.row { out.push('\n'); }
        }
        out.trim_end().to_string()
    }

    pub fn start_recording(&mut self, path: std::path::PathBuf) -> std::io::Result<()> {
        let f = std::fs::OpenOptions::new().create(true).append(true).open(&path)?;
        self.record_file = Some(f);
        self.record_path = Some(path);
        self.recording = true;
        Ok(())
    }

    pub fn stop_recording(&mut self) {
        self.record_file = None;
        self.recording = false;
    }

    /// Salva o conteúdo atual (tela visível) em arquivo.
    pub fn save_content(&self, path: &std::path::Path) -> std::io::Result<()> {
        std::fs::write(path, self.full_text())
    }

    /// Encontra a palavra em torno de (row, col).
    pub fn word_at(&self, row: usize, col: usize) -> (CellPos, CellPos) {
        let screen = self.vt_parser.screen();
        let cols = self.cols;

        let is_word = |r: usize, c: usize| -> bool {
            screen.cell(r as u16, c as u16)
                .map(|cell| {
                    let s = cell.contents();
                    !s.is_empty() && s != " " && s.chars().all(|ch| ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == '.')
                })
                .unwrap_or(false)
        };

        let mut start_col = col;
        let mut end_col = col;

        while start_col > 0 && is_word(row, start_col - 1) {
            start_col -= 1;
        }
        while end_col + 1 < cols && is_word(row, end_col + 1) {
            end_col += 1;
        }

        (CellPos { row, col: start_col }, CellPos { row, col: end_col })
    }
}

pub struct TerminalWidget<'a> {
    pub buffer: &'a mut TerminalBuffer,
    pub theme: &'a Theme,
    pub font_size: f32,
    pub is_active: bool,
}

/// Resultado de show(): além da response e resize, inclui ações solicitadas pelo widget.
pub struct TerminalWidgetResult {
    pub response: egui::Response,
    pub resize: Option<(u16, u16)>,
    pub copy_text: Option<String>,
    pub paste_requested: bool,
    pub save_content_requested: bool,
    pub toggle_recording: bool,
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

    pub fn show(&mut self, ui: &mut egui::Ui) -> TerminalWidgetResult {
        let char_w = self.font_size * 0.601;
        let char_h = self.font_size * 1.25;

        let avail = ui.available_size();

        if avail.x < char_w * 2.0 || avail.y < char_h {
            let (_, response) = ui.allocate_exact_size(avail.max(Vec2::new(1.0, 1.0)), Sense::click_and_drag());
            return TerminalWidgetResult { response, resize: None, copy_text: None, paste_requested: false, save_content_requested: false, toggle_recording: false };
        }

        let new_cols = ((avail.x / char_w).floor() as usize).max(4);
        let new_rows = ((avail.y / char_h).floor() as usize).max(2);

        let resize = if new_cols != self.buffer.cols || new_rows != self.buffer.rows {
            self.buffer.resize(new_cols, new_rows);
            Some((new_cols as u16, new_rows as u16))
        } else {
            None
        };

        let rows = self.buffer.rows;
        let cols = self.buffer.cols;

        let term_id = Id::new("terminal").with(self.buffer.session_id);
        let (rect, response) = ui.allocate_exact_size(avail, Sense::click_and_drag());

        if response.clicked() || response.gained_focus() {
            ui.memory_mut(|m| m.request_focus(term_id));
        }
        let has_focus = ui.memory(|m| m.has_focus(term_id)) || self.is_active;

        // — Converter posição de pixel para célula —
        let pixel_to_cell = |pos: Pos2| -> Option<CellPos> {
            if !rect.contains(pos) { return None; }
            let col = ((pos.x - rect.left()) / char_w).floor() as usize;
            let row = ((pos.y - rect.top()) / char_h).floor() as usize;
            Some(CellPos { row: row.min(rows.saturating_sub(1)), col: col.min(cols.saturating_sub(1)) })
        };

        // — Ações de mouse para seleção —
        let mut copy_text: Option<String> = None;
        let mut paste_requested = false;
        let mut save_content_requested = false;
        let mut toggle_recording = false;

        // Arrastar para selecionar
        if response.drag_started() {
            if let Some(pos) = ui.input(|i| i.pointer.press_origin()) {
                if let Some(cell) = pixel_to_cell(pos) {
                    self.buffer.selection.start = Some(cell);
                    self.buffer.selection.end = Some(cell);
                    self.buffer.selection.active = true;
                }
            }
        }
        if response.dragged() && self.buffer.selection.active {
            if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                if let Some(cell) = pixel_to_cell(pos) {
                    self.buffer.selection.end = Some(cell);
                }
            }
        }
        if response.drag_stopped() {
            self.buffer.selection.active = false;
        }

        // Clique simples: limpa seleção
        if response.clicked() {
            ui.memory_mut(|m| m.request_focus(term_id));
            if !response.double_clicked() {
                self.buffer.selection.clear();
            }
        }

        // Duplo-clique: seleciona palavra
        if response.double_clicked() {
            if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                if let Some(cell) = pixel_to_cell(pos) {
                    let (s, e) = self.buffer.word_at(cell.row, cell.col);
                    self.buffer.selection.start = Some(s);
                    self.buffer.selection.end = Some(e);
                    self.buffer.selection.active = false;
                    // Copia imediatamente ao selecionar palavra
                    let text = self.buffer.selected_text();
                    if !text.is_empty() {
                        copy_text = Some(text);
                    }
                }
            }
        }

        // — Scroll do mouse —
        if response.hovered() || has_focus {
            let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll_delta != 0.0 {
                let lines = (scroll_delta.abs() / char_h).ceil() as usize;
                let lines = lines.max(1);
                if scroll_delta > 0.0 {
                    self.buffer.scroll_up(lines);
                } else {
                    self.buffer.scroll_down(lines);
                }
            }
        }

        // Atalhos de teclado (only when focused)
        if has_focus || self.is_active {
            let (ctrl_shift_c, ctrl_shift_v, ctrl_shift_s, ctrl_shift_r) = ui.input_mut(|i| (
                i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::C),
                i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::V),
                i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::S),
                i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::R),
            ));

            if ctrl_shift_c {
                let text = if self.buffer.selection.is_empty() {
                    self.buffer.full_text()
                } else {
                    self.buffer.selected_text()
                };
                if !text.is_empty() { copy_text = Some(text); }
            }
            if ctrl_shift_v { paste_requested = true; }
            if ctrl_shift_s { save_content_requested = true; }
            if ctrl_shift_r { toggle_recording = true; }
        }

        // Menu contextual via context_menu do egui
        {
            let has_sel = !self.buffer.selection.is_empty();
            let recording = self.buffer.recording;
            let mut ctx_copy = false;
            let mut ctx_paste = false;
            let mut ctx_save = false;
            let mut ctx_rec = false;

            response.context_menu(|ui| {
                if ui.button(if has_sel { "Copiar seleção  Ctrl+Shift+C" } else { "Copiar tudo  Ctrl+Shift+C" }).clicked() {
                    ctx_copy = true;
                    ui.close();
                }
                if ui.button("Colar  Ctrl+Shift+V").clicked() {
                    ctx_paste = true;
                    ui.close();
                }
                ui.separator();
                if ui.button("Salvar conteúdo...  Ctrl+Shift+S").clicked() {
                    ctx_save = true;
                    ui.close();
                }
                let rec_label = if recording { "Parar gravação  Ctrl+Shift+R" } else { "Iniciar gravação  Ctrl+Shift+R" };
                if ui.button(rec_label).clicked() {
                    ctx_rec = true;
                    ui.close();
                }
            });

            if ctx_copy {
                let text = if has_sel { self.buffer.selected_text() } else { self.buffer.full_text() };
                if !text.is_empty() { copy_text = Some(text); }
            }
            if ctx_paste { paste_requested = true; }
            if ctx_save { save_content_requested = true; }
            if ctx_rec { toggle_recording = true; }
        }

        // — Render —
        if ui.is_rect_visible(rect) {
            let painter = ui.painter_at(rect);
            let bg = theme_color_to_egui(&self.theme.term_background);
            painter.rect_filled(rect, 0.0, bg);

            let screen = self.buffer.vt_parser.screen();
            let font_id = FontId::monospace(self.font_size);
            let sel_bg = Color32::from_rgba_premultiplied(100, 120, 255, 120);

            for row_idx in 0..rows {
                for col_idx in 0..cols {
                    let Some(cell) = screen.cell(row_idx as u16, col_idx as u16) else { continue };

                    let contents = cell.contents();
                    let ch = contents.chars().next().unwrap_or(' ');

                    let x = rect.left() + col_idx as f32 * char_w;
                    let y = rect.top() + row_idx as f32 * char_h;
                    let cell_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(char_w, char_h));

                    let in_selection = self.buffer.selection.contains(row_idx, col_idx);

                    if in_selection {
                        painter.rect_filled(cell_rect, 0.0, sel_bg);
                    } else {
                        let bg_color = vt_bg_color(cell.bgcolor(), self.theme);
                        let default_bg = theme_color_to_egui(&self.theme.term_background);
                        if bg_color != default_bg {
                            painter.rect_filled(cell_rect, 0.0, bg_color);
                        }
                    }

                    if ch != ' ' {
                        let fg_color = if in_selection {
                            Color32::WHITE
                        } else {
                            vt_fg_color(cell.fgcolor(), self.theme, cell.bold())
                        };
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
            let cursor = screen.cursor_position();
            let cy = (cursor.0 as usize).min(rows.saturating_sub(1));
            let cx = (cursor.1 as usize).min(cols.saturating_sub(1));
            let cx_px = rect.left() + cx as f32 * char_w;
            let cy_px = rect.top() + cy as f32 * char_h;
            let cursor_rect = Rect::from_min_size(Pos2::new(cx_px, cy_px), Vec2::new(char_w, char_h));

            if has_focus {
                let cursor_color = theme_color_to_egui(&self.theme.term_cursor);
                painter.rect_filled(cursor_rect, 0.0, cursor_color);
                let cell_under = screen.cell(cy as u16, cx as u16);
                if let Some(cell) = cell_under {
                    let ch = cell.contents().chars().next().unwrap_or(' ');
                    if ch != ' ' {
                        painter.text(
                            cursor_rect.min,
                            egui::Align2::LEFT_TOP,
                            ch.to_string(),
                            font_id.clone(),
                            invert_color(cursor_color),
                        );
                    }
                }
            } else {
                let cursor_color = theme_color_to_egui(&self.theme.term_cursor);
                painter.rect_stroke(cursor_rect, 0.0, egui::Stroke::new(1.0, cursor_color), egui::StrokeKind::Inside);
            }

            // Indicador de gravação
            if self.buffer.recording {
                let dot_pos = Pos2::new(rect.right() - 12.0, rect.top() + 10.0);
                painter.circle_filled(dot_pos, 5.0, Color32::from_rgb(220, 50, 50));
            }

            // Scrollbar lateral
            let scroll_offset = self.buffer.scroll_offset();
            let scrollback_cap = self.buffer.scrollback_capacity();
            if scrollback_cap > 0 {
                let bar_w = 4.0;
                let bar_x = rect.right() - bar_w - 2.0;
                let total_lines = scrollback_cap + rows;
                let visible_ratio = rows as f32 / total_lines as f32;
                let thumb_h = (rect.height() * visible_ratio).max(20.0);
                let max_offset = scrollback_cap.max(1);
                let scroll_ratio = 1.0 - (scroll_offset as f32 / max_offset as f32).clamp(0.0, 1.0);
                let track_h = rect.height() - thumb_h;
                let thumb_y = rect.top() + scroll_ratio * track_h;

                // Track
                painter.rect_filled(
                    Rect::from_min_size(Pos2::new(bar_x, rect.top()), Vec2::new(bar_w, rect.height())),
                    2.0,
                    Color32::from_rgba_premultiplied(255, 255, 255, 15),
                );
                // Thumb
                let thumb_color = if scroll_offset > 0 {
                    Color32::from_rgba_premultiplied(180, 140, 255, 160)
                } else {
                    Color32::from_rgba_premultiplied(255, 255, 255, 40)
                };
                painter.rect_filled(
                    Rect::from_min_size(Pos2::new(bar_x, thumb_y), Vec2::new(bar_w, thumb_h)),
                    2.0,
                    thumb_color,
                );

                // Banner "SCROLLBACK" quando no histórico
                if scroll_offset > 0 {
                    let banner_text = format!("HISTÓRICO  ({} linhas acima)  Scroll para voltar", scroll_offset);
                    let banner_rect = Rect::from_min_size(
                        Pos2::new(rect.left(), rect.top()),
                        Vec2::new(rect.width(), char_h),
                    );
                    painter.rect_filled(banner_rect, 0.0, Color32::from_rgba_premultiplied(80, 60, 140, 200));
                    painter.text(
                        banner_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        banner_text,
                        egui::FontId::monospace(self.font_size * 0.75),
                        Color32::from_rgb(200, 180, 255),
                    );
                }
            }

            // Borda de foco
            if has_focus {
                painter.rect_stroke(
                    rect,
                    0.0,
                    egui::Stroke::new(1.0, Color32::from_rgba_premultiplied(130, 100, 255, 80)),
                    egui::StrokeKind::Inside,
                );
            }
        }

        TerminalWidgetResult { response, resize, copy_text, paste_requested, save_content_requested, toggle_recording }
    }
}

fn invert_color(c: Color32) -> Color32 {
    Color32::from_rgb(255 - c.r(), 255 - c.g(), 255 - c.b())
}

fn vt_fg_color(color: vt100::Color, theme: &Theme, bold: bool) -> Color32 {
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

fn vt_bg_color(color: vt100::Color, theme: &Theme) -> Color32 {
    match color {
        vt100::Color::Default => theme_color_to_egui(&theme.term_background),
        vt100::Color::Idx(i) => {
            let idx = i as usize;
            if idx < 16 {
                let c = &theme.ansi[idx];
                Color32::from_rgb(c.0, c.1, c.2)
            } else {
                xterm256_to_egui(i)
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
