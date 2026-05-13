use serde::{Deserialize, Serialize};
use nsr_vault::Host;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostInfo {
    pub alias: String,
    pub hostname: String,
    pub user: String,
    pub port: u16,
}

impl From<&Host> for HostInfo {
    fn from(h: &Host) -> Self {
        Self {
            alias: h.alias.clone(),
            hostname: h.hostname.clone(),
            user: h.user.clone(),
            port: h.port,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginEvent {
    OnConnect { host: HostInfo },
    OnDisconnect { host: HostInfo },
    OnOutput { session_id: String, data: Vec<u8> },
    OnCommand { command: String },
}

pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn on_event(&mut self, event: &PluginEvent);
}
