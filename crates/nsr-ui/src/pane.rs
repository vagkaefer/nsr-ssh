use uuid::Uuid;

pub type SessionId = Uuid;

#[derive(Debug, Clone)]
pub enum PaneTree {
    Terminal(SessionId),
    HSplit {
        ratio: f32,
        left: Box<PaneTree>,
        right: Box<PaneTree>,
    },
    VSplit {
        ratio: f32,
        top: Box<PaneTree>,
        bottom: Box<PaneTree>,
    },
}

impl PaneTree {
    pub fn sessions(&self) -> Vec<SessionId> {
        match self {
            PaneTree::Terminal(id) => vec![*id],
            PaneTree::HSplit { left, right, .. } => {
                let mut ids = left.sessions();
                ids.extend(right.sessions());
                ids
            }
            PaneTree::VSplit { top, bottom, .. } => {
                let mut ids = top.sessions();
                ids.extend(bottom.sessions());
                ids
            }
        }
    }

    pub fn split_horizontal(self, new_session: SessionId) -> Self {
        PaneTree::HSplit {
            ratio: 0.5,
            left: Box::new(self),
            right: Box::new(PaneTree::Terminal(new_session)),
        }
    }

    pub fn split_vertical(self, new_session: SessionId) -> Self {
        PaneTree::VSplit {
            ratio: 0.5,
            top: Box::new(self),
            bottom: Box::new(PaneTree::Terminal(new_session)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Tab {
    pub id: Uuid,
    pub title: String,
    pub pane_tree: PaneTree,
    pub active_pane: SessionId,
    pub host_alias: String,
}

impl Tab {
    pub fn new(title: impl Into<String>, host_alias: impl Into<String>, session_id: SessionId) -> Self {
        Self {
            id: Uuid::new_v4(),
            title: title.into(),
            pane_tree: PaneTree::Terminal(session_id),
            active_pane: session_id,
            host_alias: host_alias.into(),
        }
    }
}
