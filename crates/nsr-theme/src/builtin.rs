use crate::theme::{Color, Theme};

pub fn dracula() -> Theme {
    Theme {
        name: "Dracula".into(),
        author: Some("Zeno Rocha".into()),
        ui_background: Color::rgb(40, 42, 54),
        ui_surface: Color::rgb(68, 71, 90),
        ui_border: Color::rgb(98, 114, 164),
        ui_accent: Color::rgb(189, 147, 249),
        ui_text: Color::rgb(248, 248, 242),
        ui_text_dim: Color::rgb(98, 114, 164),
        ui_tab_active: Color::rgb(68, 71, 90),
        ui_tab_inactive: Color::rgb(40, 42, 54),
        term_background: Color::rgb(40, 42, 54),
        term_foreground: Color::rgb(248, 248, 242),
        term_cursor: Color::rgb(248, 248, 242),
        term_selection: Color::rgb(68, 71, 90),
        ansi: [
            Color::rgb(40, 42, 54),   // black
            Color::rgb(255, 85, 85),  // red
            Color::rgb(80, 250, 123), // green
            Color::rgb(241, 250, 140),// yellow
            Color::rgb(189, 147, 249),// blue
            Color::rgb(255, 121, 198),// magenta
            Color::rgb(139, 233, 253),// cyan
            Color::rgb(248, 248, 242),// white
            // bright
            Color::rgb(98, 114, 164), // bright black
            Color::rgb(255, 85, 85),  // bright red
            Color::rgb(80, 250, 123), // bright green
            Color::rgb(241, 250, 140),// bright yellow
            Color::rgb(189, 147, 249),// bright blue
            Color::rgb(255, 121, 198),// bright magenta
            Color::rgb(139, 233, 253),// bright cyan
            Color::rgb(255, 255, 255),// bright white
        ],
    }
}

pub fn nord() -> Theme {
    Theme {
        name: "Nord".into(),
        author: Some("Arctic Ice Studio".into()),
        ui_background: Color::rgb(46, 52, 64),
        ui_surface: Color::rgb(59, 66, 82),
        ui_border: Color::rgb(76, 86, 106),
        ui_accent: Color::rgb(136, 192, 208),
        ui_text: Color::rgb(236, 239, 244),
        ui_text_dim: Color::rgb(76, 86, 106),
        ui_tab_active: Color::rgb(59, 66, 82),
        ui_tab_inactive: Color::rgb(46, 52, 64),
        term_background: Color::rgb(46, 52, 64),
        term_foreground: Color::rgb(216, 222, 233),
        term_cursor: Color::rgb(216, 222, 233),
        term_selection: Color::rgb(67, 76, 94),
        ansi: [
            Color::rgb(59, 66, 82),   // black
            Color::rgb(191, 97, 106), // red
            Color::rgb(163, 190, 140),// green
            Color::rgb(235, 203, 139),// yellow
            Color::rgb(129, 161, 193),// blue
            Color::rgb(180, 142, 173),// magenta
            Color::rgb(136, 192, 208),// cyan
            Color::rgb(229, 233, 240),// white
            Color::rgb(76, 86, 106),  // bright black
            Color::rgb(191, 97, 106), // bright red
            Color::rgb(163, 190, 140),// bright green
            Color::rgb(235, 203, 139),// bright yellow
            Color::rgb(129, 161, 193),// bright blue
            Color::rgb(180, 142, 173),// bright magenta
            Color::rgb(143, 188, 187),// bright cyan
            Color::rgb(236, 239, 244),// bright white
        ],
    }
}

pub fn catppuccin_mocha() -> Theme {
    Theme {
        name: "Catppuccin Mocha".into(),
        author: Some("Catppuccin".into()),
        ui_background: Color::rgb(30, 30, 46),
        ui_surface: Color::rgb(49, 50, 68),
        ui_border: Color::rgb(88, 91, 112),
        ui_accent: Color::rgb(203, 166, 247),
        ui_text: Color::rgb(205, 214, 244),
        ui_text_dim: Color::rgb(108, 112, 134),
        ui_tab_active: Color::rgb(49, 50, 68),
        ui_tab_inactive: Color::rgb(30, 30, 46),
        term_background: Color::rgb(30, 30, 46),
        term_foreground: Color::rgb(205, 214, 244),
        term_cursor: Color::rgb(243, 139, 168),
        term_selection: Color::rgb(69, 71, 90),
        ansi: [
            Color::rgb(69, 71, 90),   // black
            Color::rgb(243, 139, 168),// red
            Color::rgb(166, 227, 161),// green
            Color::rgb(249, 226, 175),// yellow
            Color::rgb(137, 180, 250),// blue
            Color::rgb(245, 194, 231),// magenta
            Color::rgb(148, 226, 213),// cyan
            Color::rgb(186, 194, 222),// white
            Color::rgb(88, 91, 112),  // bright black
            Color::rgb(243, 139, 168),// bright red
            Color::rgb(166, 227, 161),// bright green
            Color::rgb(249, 226, 175),// bright yellow
            Color::rgb(137, 180, 250),// bright blue
            Color::rgb(245, 194, 231),// bright magenta
            Color::rgb(148, 226, 213),// bright cyan
            Color::rgb(205, 214, 244),// bright white
        ],
    }
}

pub fn solarized_dark() -> Theme {
    Theme {
        name: "Solarized Dark".into(),
        author: Some("Ethan Schoonover".into()),
        ui_background: Color::rgb(0, 43, 54),
        ui_surface: Color::rgb(7, 54, 66),
        ui_border: Color::rgb(88, 110, 117),
        ui_accent: Color::rgb(38, 139, 210),
        ui_text: Color::rgb(131, 148, 150),
        ui_text_dim: Color::rgb(88, 110, 117),
        ui_tab_active: Color::rgb(7, 54, 66),
        ui_tab_inactive: Color::rgb(0, 43, 54),
        term_background: Color::rgb(0, 43, 54),
        term_foreground: Color::rgb(131, 148, 150),
        term_cursor: Color::rgb(131, 148, 150),
        term_selection: Color::rgb(7, 54, 66),
        ansi: [
            Color::rgb(7, 54, 66),    // black
            Color::rgb(220, 50, 47),  // red
            Color::rgb(133, 153, 0),  // green
            Color::rgb(181, 137, 0),  // yellow
            Color::rgb(38, 139, 210), // blue
            Color::rgb(211, 54, 130), // magenta
            Color::rgb(42, 161, 152), // cyan
            Color::rgb(238, 232, 213),// white
            Color::rgb(0, 43, 54),    // bright black
            Color::rgb(203, 75, 22),  // bright red
            Color::rgb(88, 110, 117), // bright green
            Color::rgb(101, 123, 131),// bright yellow
            Color::rgb(131, 148, 150),// bright blue
            Color::rgb(108, 113, 196),// bright magenta
            Color::rgb(147, 161, 161),// bright cyan
            Color::rgb(253, 246, 227),// bright white
        ],
    }
}

pub fn all_themes() -> Vec<Theme> {
    vec![dracula(), nord(), catppuccin_mocha(), solarized_dark()]
}
