use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum SessionEvent {
    Connected { session_id: Uuid },
    Disconnected { session_id: Uuid },
    Reconnecting { session_id: Uuid, attempt: u32 },
    Output { session_id: Uuid, data: Vec<u8> },
    Error { session_id: Uuid, message: String },
    TitleChanged { session_id: Uuid, title: String },
}

#[derive(Debug, Clone)]
pub enum AppCommand {
    Connect { session_id: Uuid },
    Disconnect { session_id: Uuid },
    SendInput { session_id: Uuid, data: Vec<u8> },
    Resize { session_id: Uuid, cols: u16, rows: u16 },
}
