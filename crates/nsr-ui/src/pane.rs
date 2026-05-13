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

    // Divide o pane que contém `target` horizontalmente (lado a lado)
    pub fn split_h_at(self, target: SessionId, new_session: SessionId) -> Self {
        match self {
            PaneTree::Terminal(id) if id == target => PaneTree::HSplit {
                ratio: 0.5,
                left: Box::new(PaneTree::Terminal(id)),
                right: Box::new(PaneTree::Terminal(new_session)),
            },
            PaneTree::HSplit { ratio, left, right } => PaneTree::HSplit {
                ratio,
                left: Box::new(left.split_h_at(target, new_session)),
                right: Box::new(right.split_h_at(target, new_session)),
            },
            PaneTree::VSplit { ratio, top, bottom } => PaneTree::VSplit {
                ratio,
                top: Box::new(top.split_h_at(target, new_session)),
                bottom: Box::new(bottom.split_h_at(target, new_session)),
            },
            other => other,
        }
    }

    // Divide o pane que contém `target` verticalmente (cima/baixo)
    pub fn split_v_at(self, target: SessionId, new_session: SessionId) -> Self {
        match self {
            PaneTree::Terminal(id) if id == target => PaneTree::VSplit {
                ratio: 0.5,
                top: Box::new(PaneTree::Terminal(id)),
                bottom: Box::new(PaneTree::Terminal(new_session)),
            },
            PaneTree::HSplit { ratio, left, right } => PaneTree::HSplit {
                ratio,
                left: Box::new(left.split_v_at(target, new_session)),
                right: Box::new(right.split_v_at(target, new_session)),
            },
            PaneTree::VSplit { ratio, top, bottom } => PaneTree::VSplit {
                ratio,
                top: Box::new(top.split_v_at(target, new_session)),
                bottom: Box::new(bottom.split_v_at(target, new_session)),
            },
            other => other,
        }
    }

    // Fecha o pane que contém `target`; retorna None se a árvore inteira foi removida
    pub fn close_pane(self, target: SessionId) -> Option<Self> {
        match self {
            PaneTree::Terminal(id) if id == target => None,
            PaneTree::Terminal(_) => Some(self),
            PaneTree::HSplit { ratio, left, right } => {
                match (left.close_pane(target), right.close_pane(target)) {
                    (None, None) => None,
                    (Some(l), None) => Some(l),
                    (None, Some(r)) => Some(r),
                    (Some(l), Some(r)) => Some(PaneTree::HSplit {
                        ratio,
                        left: Box::new(l),
                        right: Box::new(r),
                    }),
                }
            }
            PaneTree::VSplit { ratio, top, bottom } => {
                match (top.close_pane(target), bottom.close_pane(target)) {
                    (None, None) => None,
                    (Some(t), None) => Some(t),
                    (None, Some(b)) => Some(b),
                    (Some(t), Some(b)) => Some(PaneTree::VSplit {
                        ratio,
                        top: Box::new(t),
                        bottom: Box::new(b),
                    }),
                }
            }
        }
    }

    // Atualiza o ratio de um split que contém `target`
    pub fn set_ratio_for(self, target: SessionId, new_ratio: f32) -> Self {
        match self {
            PaneTree::HSplit { ratio: _, left, right } if left.contains(target) || right.contains(target) => {
                PaneTree::HSplit { ratio: new_ratio.clamp(0.1, 0.9), left, right }
            }
            PaneTree::VSplit { ratio: _, top, bottom } if top.contains(target) || bottom.contains(target) => {
                PaneTree::VSplit { ratio: new_ratio.clamp(0.1, 0.9), top, bottom }
            }
            PaneTree::HSplit { ratio, left, right } => PaneTree::HSplit {
                ratio,
                left: Box::new(left.set_ratio_for(target, new_ratio)),
                right: Box::new(right.set_ratio_for(target, new_ratio)),
            },
            PaneTree::VSplit { ratio, top, bottom } => PaneTree::VSplit {
                ratio,
                top: Box::new(top.set_ratio_for(target, new_ratio)),
                bottom: Box::new(bottom.set_ratio_for(target, new_ratio)),
            },
            other => other,
        }
    }

    pub fn contains(&self, target: SessionId) -> bool {
        self.sessions().contains(&target)
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
