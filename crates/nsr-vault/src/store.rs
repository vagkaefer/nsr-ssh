use std::path::PathBuf;
use anyhow::{Context, Result};
use tracing::debug;
use crate::host::Host;

pub struct VaultStore {
    ssh_config_path: PathBuf,
    nsr_vault_path: PathBuf,
}

impl VaultStore {
    pub fn new() -> Result<Self> {
        let home = dirs::home_dir().context("não foi possível encontrar o diretório home")?;
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| home.join(".config"));

        let nsr_dir = config_dir.join("nsr-ssh");
        std::fs::create_dir_all(&nsr_dir)?;

        Ok(Self {
            ssh_config_path: home.join(".ssh").join("config"),
            nsr_vault_path: nsr_dir.join("vault.json"),
        })
    }

    // Lê hosts do vault interno do NSR (inclui metadados como tags/descrição)
    pub fn load_hosts(&self) -> Result<Vec<Host>> {
        if !self.nsr_vault_path.exists() {
            // Fallback: importa do ~/.ssh/config na primeira execução
            return self.import_from_ssh_config();
        }

        let content = std::fs::read_to_string(&self.nsr_vault_path)
            .context("falha ao ler vault")?;
        let hosts: Vec<Host> = serde_json::from_str(&content)
            .context("falha ao parsear vault")?;
        debug!("Carregados {} hosts do vault", hosts.len());
        Ok(hosts)
    }

    pub fn save_hosts(&self, hosts: &[Host]) -> Result<()> {
        let content = serde_json::to_string_pretty(hosts)?;
        std::fs::write(&self.nsr_vault_path, content)?;
        debug!("Salvos {} hosts no vault", hosts.len());
        self.sync_to_ssh_config(hosts)?;
        Ok(())
    }

    // Sincroniza hosts do NSR de volta para ~/.ssh/config
    pub fn sync_to_ssh_config(&self, hosts: &[Host]) -> Result<()> {
        if let Some(parent) = self.ssh_config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut lines: Vec<String> = vec![
            "# Gerenciado pelo NSR-SSH — não edite manualmente os blocos abaixo".into(),
            "# Blocos fora desta seção são preservados".into(),
            String::new(),
        ];

        // Preserva entradas existentes que não são do NSR
        let existing = self.read_non_nsr_blocks();
        if !existing.is_empty() {
            lines.push(existing);
            lines.push(String::new());
        }

        lines.push("# BEGIN NSR-SSH VAULT".into());
        for host in hosts {
            lines.push(String::new());
            lines.push(host.to_ssh_config_block());
        }
        lines.push(String::new());
        lines.push("# END NSR-SSH VAULT".into());

        std::fs::write(&self.ssh_config_path, lines.join("\n"))?;
        Ok(())
    }

    // Importa hosts do ~/.ssh/config existente
    pub fn import_from_ssh_config(&self) -> Result<Vec<Host>> {
        if !self.ssh_config_path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&self.ssh_config_path)?;
        let hosts = parse_ssh_config(&content);
        debug!("Importados {} hosts do ~/.ssh/config", hosts.len());
        Ok(hosts)
    }

    fn read_non_nsr_blocks(&self) -> String {
        if !self.ssh_config_path.exists() {
            return String::new();
        }
        let content = std::fs::read_to_string(&self.ssh_config_path).unwrap_or_default();
        let mut in_nsr_block = false;
        let mut preserved = Vec::new();

        for line in content.lines() {
            if line.trim() == "# BEGIN NSR-SSH VAULT" {
                in_nsr_block = true;
                continue;
            }
            if line.trim() == "# END NSR-SSH VAULT" {
                in_nsr_block = false;
                continue;
            }
            if !in_nsr_block && !line.starts_with("# Gerenciado pelo NSR-SSH") {
                preserved.push(line.to_string());
            }
        }

        preserved.join("\n").trim().to_string()
    }
}

// Parser simples de ~/.ssh/config
pub fn parse_ssh_config(content: &str) -> Vec<Host> {
    let mut hosts = Vec::new();
    let mut current: Option<Host> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let (key, value) = match line.split_once(char::is_whitespace) {
            Some((k, v)) => (k.to_lowercase(), v.trim().to_string()),
            None => continue,
        };

        match key.as_str() {
            "host" => {
                if let Some(h) = current.take() {
                    if h.alias != "*" {
                        hosts.push(h);
                    }
                }
                current = Some(Host {
                    id: uuid::Uuid::new_v4(),
                    alias: value,
                    hostname: String::new(),
                    user: std::env::var("USER").unwrap_or_else(|_| "root".into()),
                    port: 22,
                    identity_file: None,
                    tags: Vec::new(),
                    description: None,
                });
            }
            "hostname" => {
                if let Some(ref mut h) = current {
                    h.hostname = value;
                }
            }
            "user" => {
                if let Some(ref mut h) = current {
                    h.user = value;
                }
            }
            "port" => {
                if let Some(ref mut h) = current {
                    h.port = value.parse().unwrap_or(22);
                }
            }
            "identityfile" => {
                if let Some(ref mut h) = current {
                    h.identity_file = Some(value);
                }
            }
            _ => {}
        }
    }

    if let Some(h) = current {
        if h.alias != "*" && !h.hostname.is_empty() {
            hosts.push(h);
        }
    }

    hosts
}
