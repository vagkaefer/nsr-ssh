pub mod builtin;
pub mod theme;

pub use theme::{Color, Theme};
pub use builtin::all_themes;

use anyhow::Result;
use std::path::Path;

pub fn load_from_toml(path: &Path) -> Result<Theme> {
    let content = std::fs::read_to_string(path)?;
    let theme: Theme = toml::from_str(&content)?;
    Ok(theme)
}

pub fn load_user_themes() -> Vec<Theme> {
    let mut themes = builtin::all_themes();

    let Some(config_dir) = dirs::config_dir() else { return themes };
    let themes_dir = config_dir.join("nsr-ssh").join("themes");

    if let Ok(entries) = std::fs::read_dir(&themes_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                match load_from_toml(&path) {
                    Ok(t) => themes.push(t),
                    Err(e) => eprintln!("Erro ao carregar tema {:?}: {}", path, e),
                }
            }
        }
    }

    themes
}
