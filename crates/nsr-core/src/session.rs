use std::sync::Arc;
use std::future::Future;
use tokio::sync::{mpsc, broadcast, Mutex};
use anyhow::Result;
use tracing::{error, info, warn};
use uuid::Uuid;
use russh::client;

use nsr_vault::Host;
use crate::events::{AppCommand, SessionEvent};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SessionState {
    Idle,
    Connecting,
    Connected,
    Disconnected,
    Reconnecting { attempt: u32 },
    Failed(String),
}

pub struct Session {
    pub id: Uuid,
    pub host: Host,
    pub state: SessionState,
    pub input_tx: mpsc::Sender<Vec<u8>>,
    pub cols: u16,
    pub rows: u16,
}

struct SessionHandle {
    id: Uuid,
    host: Host,
    state: SessionState,
    input_tx: mpsc::Sender<Vec<u8>>,
    cols: u16,
    rows: u16,
}

pub struct SessionManager {
    sessions: Arc<Mutex<Vec<SessionHandle>>>,
    event_tx: broadcast::Sender<SessionEvent>,
    command_tx: mpsc::Sender<AppCommand>,
}

impl SessionManager {
    pub fn new() -> (Self, broadcast::Receiver<SessionEvent>) {
        let (event_tx, event_rx) = broadcast::channel(256);
        let (command_tx, command_rx) = mpsc::channel(64);
        let sessions = Arc::new(Mutex::new(Vec::<SessionHandle>::new()));
        let sessions_clone = sessions.clone();
        let event_tx_clone = event_tx.clone();

        tokio::spawn(async move {
            command_loop(command_rx, sessions_clone, event_tx_clone).await;
        });

        (Self { sessions, event_tx, command_tx }, event_rx)
    }

    pub async fn connect(&self, host: Host, password: Option<String>) -> Result<(Uuid, mpsc::Receiver<Vec<u8>>)> {
        let session_id = Uuid::new_v4();
        let (input_tx, input_rx) = mpsc::channel::<Vec<u8>>(256);
        let (output_tx, output_rx) = mpsc::channel::<Vec<u8>>(4096);

        let handle = SessionHandle {
            id: session_id,
            host: host.clone(),
            state: SessionState::Connecting,
            input_tx: input_tx.clone(),
            cols: 220,
            rows: 50,
        };

        self.sessions.lock().await.push(handle);

        let event_tx = self.event_tx.clone();

        tokio::spawn(async move {
            run_ssh_session(session_id, host, password, input_rx, output_tx, event_tx).await;
        });

        Ok((session_id, output_rx))
    }

    pub async fn send_input(&self, session_id: Uuid, data: Vec<u8>) {
        let sessions = self.sessions.lock().await;
        if let Some(h) = sessions.iter().find(|s| s.id == session_id) {
            let _ = h.input_tx.try_send(data);
        }
    }

    pub async fn disconnect(&self, session_id: Uuid) {
        let _ = self.command_tx.send(AppCommand::Disconnect { session_id }).await;
    }

    pub async fn resize(&self, session_id: Uuid, cols: u16, rows: u16) {
        // Envia magic resize packet pela fila de input da sessão
        let resize_packet = vec![
            0x00, 0xFF,
            (cols >> 8) as u8, cols as u8,
            (rows >> 8) as u8, rows as u8,
        ];
        self.send_input(session_id, resize_packet).await;
    }

    pub fn event_receiver(&self) -> broadcast::Receiver<SessionEvent> {
        self.event_tx.subscribe()
    }
}

async fn run_ssh_session(
    session_id: Uuid,
    host: Host,
    password: Option<String>,
    mut input_rx: mpsc::Receiver<Vec<u8>>,
    output_tx: mpsc::Sender<Vec<u8>>,
    event_tx: broadcast::Sender<SessionEvent>,
) {
    info!("Conectando em {}@{}:{}", host.user, host.hostname, host.port);

    let config = Arc::new(client::Config {
        inactivity_timeout: Some(std::time::Duration::from_secs(60)),
        ..<_>::default()
    });

    let addr = format!("{}:{}", host.hostname, host.port);

    let sh = SshClientHandler {
        output_tx: output_tx.clone(),
    };

    let mut session = match client::connect(config, &addr, sh).await {
        Ok(s) => s,
        Err(e) => {
            error!("Falha ao conectar em {}: {}", addr, e);
            let _ = event_tx.send(SessionEvent::Error {
                session_id,
                message: format!("Falha ao conectar: {}", e),
            });
            let _ = event_tx.send(SessionEvent::Disconnected { session_id });
            return;
        }
    };

    // Tenta autenticação: chave privada → senha → padrão
    let auth_ok = try_authenticate(&mut session, &host, password).await;

    if !auth_ok {
        let _ = event_tx.send(SessionEvent::Error {
            session_id,
            message: "Autenticação falhou. Verifique credenciais.".into(),
        });
        let _ = event_tx.send(SessionEvent::Disconnected { session_id });
        return;
    }

    let mut channel = match session.channel_open_session().await {
        Ok(c) => c,
        Err(e) => {
            let _ = event_tx.send(SessionEvent::Error {
                session_id,
                message: format!("Falha ao abrir canal: {}", e),
            });
            let _ = event_tx.send(SessionEvent::Disconnected { session_id });
            return;
        }
    };

    if let Err(e) = channel
        .request_pty(false, "xterm-256color", 220, 50, 0, 0, &[])
        .await
    {
        warn!("Falha ao solicitar PTY: {}", e);
    }

    if let Err(e) = channel.request_shell(false).await {
        let _ = event_tx.send(SessionEvent::Error {
            session_id,
            message: format!("Falha ao iniciar shell: {}", e),
        });
        let _ = event_tx.send(SessionEvent::Disconnected { session_id });
        return;
    }

    info!("Sessão SSH {} estabelecida com {}@{}", session_id, host.user, host.hostname);
    let _ = event_tx.send(SessionEvent::Connected { session_id });

    loop {
        tokio::select! {
            Some(data) = input_rx.recv() => {
                // Resize request: marcado com magic prefix [0x00, 0xFF, cols_hi, cols_lo, rows_hi, rows_lo]
                if data.len() == 6 && data[0] == 0x00 && data[1] == 0xFF {
                    let cols = u16::from_be_bytes([data[2], data[3]]);
                    let rows = u16::from_be_bytes([data[4], data[5]]);
                    let _ = channel.window_change(cols as u32, rows as u32, 0, 0).await;
                } else if let Err(e) = channel.data(data.as_slice()).await {
                    error!("Erro ao enviar dados para {}: {}", session_id, e);
                    break;
                }
            }
            msg = channel.wait() => {
                match msg {
                    Some(russh::ChannelMsg::Data { ref data }) => {
                        if output_tx.send(data.to_vec()).await.is_err() {
                            break;
                        }
                    }
                    Some(russh::ChannelMsg::ExtendedData { ref data, .. }) => {
                        let _ = output_tx.send(data.to_vec()).await;
                    }
                    Some(russh::ChannelMsg::ExitStatus { .. }) | None => {
                        info!("Canal SSH {} fechado", session_id);
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    let _ = event_tx.send(SessionEvent::Disconnected { session_id });
}

async fn try_authenticate(
    session: &mut client::Handle<SshClientHandler>,
    host: &Host,
    password: Option<String>,
) -> bool {
    // Pega o melhor hash RSA suportado pelo servidor (None para ed25519/ecdsa)
    let rsa_hash = session.best_supported_rsa_hash().await
        .ok()
        .flatten()
        .flatten();

    let try_key = |key: russh::keys::PrivateKey| {
        russh::keys::PrivateKeyWithHashAlg::new(Arc::new(key), rsa_hash.clone())
    };

    // 1. Chave privada especificada no host
    if let Some(ref id_file) = host.identity_file {
        let expanded = shellexpand::tilde(id_file).into_owned();
        if let Ok(key) = russh::keys::load_secret_key(&expanded, None) {
            if let Ok(result) = session.authenticate_publickey(&host.user, try_key(key)).await {
                if result.success() {
                    info!("Auth via identity_file OK");
                    return true;
                }
            }
        }
    }

    // 2. Senha fornecida pelo usuário
    if let Some(ref pwd) = password {
        if let Ok(result) = session.authenticate_password(&host.user, pwd).await {
            if result.success() {
                info!("Auth via password OK");
                return true;
            }
        }
    }

    // 3. Chaves padrão (~/.ssh/id_*)
    for default_key in &["~/.ssh/id_ed25519", "~/.ssh/id_rsa", "~/.ssh/id_ecdsa"] {
        let expanded = shellexpand::tilde(default_key).into_owned();
        if std::path::Path::new(&expanded).exists() {
            if let Ok(key) = russh::keys::load_secret_key(&expanded, None) {
                if let Ok(result) = session.authenticate_publickey(&host.user, try_key(key)).await {
                    if result.success() {
                        info!("Auth via default key {} OK", default_key);
                        return true;
                    }
                }
            }
        }
    }

    warn!("Todas as tentativas de autenticação falharam para {}@{}", host.user, host.hostname);
    false
}

struct SshClientHandler {
    output_tx: mpsc::Sender<Vec<u8>>,
}

impl client::Handler for SshClientHandler {
    type Error = russh::Error;

    fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::PublicKey,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
        // TODO: implementar verificação de known_hosts
        async { Ok(true) }
    }
}

async fn command_loop(
    mut rx: mpsc::Receiver<AppCommand>,
    sessions: Arc<Mutex<Vec<SessionHandle>>>,
    _event_tx: broadcast::Sender<SessionEvent>,
) {
    while let Some(cmd) = rx.recv().await {
        match cmd {
            AppCommand::Disconnect { session_id } => {
                let mut sessions = sessions.lock().await;
                sessions.retain(|s| s.id != session_id);
            }
            AppCommand::Resize { session_id, cols, rows } => {
                let mut sessions = sessions.lock().await;
                if let Some(h) = sessions.iter_mut().find(|s| s.id == session_id) {
                    h.cols = cols;
                    h.rows = rows;
                }
            }
            _ => {}
        }
    }
}
