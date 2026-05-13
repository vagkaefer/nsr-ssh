use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Host {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    pub alias: String,
    pub hostname: String,
    pub user: String,
    pub port: u16,
    pub identity_file: Option<String>,
    pub tags: Vec<String>,
    pub description: Option<String>,
}

impl Host {
    pub fn new(alias: impl Into<String>, hostname: impl Into<String>, user: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            alias: alias.into(),
            hostname: hostname.into(),
            user: user.into(),
            port: 22,
            identity_file: None,
            tags: Vec::new(),
            description: None,
        }
    }

    pub fn display_name(&self) -> &str {
        &self.alias
    }

    pub fn connection_string(&self) -> String {
        format!("{}@{}:{}", self.user, self.hostname, self.port)
    }

    // Serializa no formato ~/.ssh/config
    pub fn to_ssh_config_block(&self) -> String {
        let mut lines = vec![format!("Host {}", self.alias)];
        lines.push(format!("    HostName {}", self.hostname));
        lines.push(format!("    User {}", self.user));
        lines.push(format!("    Port {}", self.port));
        if let Some(ref id_file) = self.identity_file {
            lines.push(format!("    IdentityFile {}", id_file));
        }
        if let Some(ref desc) = self.description {
            lines.push(format!("    # {}", desc));
        }
        lines.join("\n")
    }
}

impl Default for Host {
    fn default() -> Self {
        Self::new("novo-host", "192.168.1.1", "root")
    }
}
