pub mod api;

pub use api::{HostInfo, Plugin, PluginEvent};

use std::path::Path;
use tracing::warn;

pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self { plugins: Vec::new() }
    }

    pub fn load_from_dir(&mut self, dir: &Path) {
        if !dir.exists() {
            return;
        }
        // TODO Fase 5: carregar .wasm e .lua do diretório
        warn!("Sistema de plugins será implementado na Fase 5");
    }

    pub fn dispatch(&mut self, event: &PluginEvent) {
        for plugin in &mut self.plugins {
            plugin.on_event(event);
        }
    }

    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}
