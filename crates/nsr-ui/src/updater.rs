use serde::Deserialize;
use std::sync::{Arc, Mutex};

const REPO: &str = "vagkaefer/nsr-ssh";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub latest_version: String,
    pub release_url: String,
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
}

/// Handle compartilhado entre a task de background e a UI.
/// Estado: None = ainda checando, Some(None) = sem update, Some(Some(...)) = update disponível.
pub type UpdateState = Arc<Mutex<Option<Option<UpdateInfo>>>>;

pub fn spawn_update_check(rt: &tokio::runtime::Runtime) -> UpdateState {
    let state: UpdateState = Arc::new(Mutex::new(None));
    let state_clone = Arc::clone(&state);

    rt.spawn(async move {
        let result = check_latest().await;
        if let Ok(mut guard) = state_clone.lock() {
            *guard = Some(result);
        }
    });

    state
}

async fn check_latest() -> Option<UpdateInfo> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", REPO);

    let client = reqwest::Client::builder()
        .user_agent(format!("nsr-ssh/{}", CURRENT_VERSION))
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .ok()?;

    let release: GithubRelease = client.get(&url).send().await.ok()?.json().await.ok()?;

    let tag = release.tag_name.trim_start_matches('v');
    let current = CURRENT_VERSION.trim_start_matches('v');

    if is_newer(tag, current) {
        Some(UpdateInfo {
            latest_version: release.tag_name.clone(),
            release_url: release.html_url,
        })
    } else {
        None
    }
}

/// Compara versões semver simples (major.minor.patch).
fn is_newer(latest: &str, current: &str) -> bool {
    fn parse(v: &str) -> [u32; 3] {
        let mut parts = v.splitn(3, '.').map(|p| p.parse::<u32>().unwrap_or(0));
        [
            parts.next().unwrap_or(0),
            parts.next().unwrap_or(0),
            parts.next().unwrap_or(0),
        ]
    }
    parse(latest) > parse(current)
}
