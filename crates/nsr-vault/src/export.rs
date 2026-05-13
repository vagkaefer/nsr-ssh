use anyhow::Result;
use crate::host::Host;

pub fn export_json(hosts: &[Host]) -> Result<String> {
    Ok(serde_json::to_string_pretty(hosts)?)
}

pub fn import_json(json: &str) -> Result<Vec<Host>> {
    let hosts: Vec<Host> = serde_json::from_str(json)?;
    Ok(hosts)
}

// Exporta no formato ~/.ssh/config puro (sem metadados NSR)
pub fn export_ssh_config(hosts: &[Host]) -> String {
    hosts
        .iter()
        .map(|h| h.to_ssh_config_block())
        .collect::<Vec<_>>()
        .join("\n\n")
}
