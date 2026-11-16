// 侧栏卡片（Spaces 与 Agents 列表的 compact/full 样式）悬停只在客户端重着色圆角边框，不改变内容或焦点。
use super::*;
use crossterm::event::{MouseEvent, MouseEventKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum HoveredCard {
    Workspace(String),
    Agent(String),
    EndpointAgent(ClientEndpointId, String),
}

impl ClientShellState {
    fn card_hover_allowed(&self) -> bool {
        self.config.mouse_capture
            && self.mode == ClientShellMode::Terminal
            && self.overlay.is_none()
            && self.popup_terminal_id.is_none()
            && self.outer_focused != Some(false)
            && self.chrome_drag.is_none()
    }

    fn workspace_cards_enabled(&self) -> bool {
        self.config.spaces.card_style != SidebarCardStyle::Plain
    }

    fn agent_cards_enabled(&self) -> bool {
        self.config.agents.card_style != SidebarCardStyle::Plain
    }

    fn card_at(&self, point: (u16, u16)) -> Option<HoveredCard> {
        let workspace = self
            .workspace_cards_enabled()
            .then(|| {
                self.hits
                    .workspaces
                    .iter()
                    .find(|hit| contains(hit.rect, point))
                    .map(|hit| HoveredCard::Workspace(hit.workspace_id.clone()))
            })
            .flatten();
        let agent = || {
            self.agent_cards_enabled()
                .then(|| {
                    self.hits
                        .agents
                        .iter()
                        .find(|(rect, _)| contains(*rect, point))
                        .map(|(_, pane_id)| HoveredCard::Agent(pane_id.clone()))
                        .or_else(|| {
                            self.hits
                                .endpoint_agents
                                .iter()
                                .find(|(rect, _, _)| contains(*rect, point))
                                .map(|(_, endpoint_id, pane_id)| {
                                    HoveredCard::EndpointAgent(endpoint_id.clone(), pane_id.clone())
                                })
                        })
                })
                .flatten()
        };
        workspace.or_else(agent)
    }

    fn card_rect(&self, card: &HoveredCard) -> Option<Rect> {
        match card {
            HoveredCard::Workspace(id) if self.workspace_cards_enabled() => self
                .hits
                .workspaces
                .iter()
                .find(|hit| &hit.workspace_id == id)
                .map(|hit| hit.rect),
            HoveredCard::Agent(id) if self.agent_cards_enabled() => self
                .hits
                .agents
                .iter()
                .find(|(_, pane_id)| pane_id == id)
                .map(|(rect, _)| *rect),
            HoveredCard::EndpointAgent(endpoint, id) if self.agent_cards_enabled() => self
                .hits
                .endpoint_agents
                .iter()
                .find(|(_, endpoint_id, pane_id)| endpoint_id == endpoint && pane_id == id)
                .map(|(rect, _, _)| *rect),
            _ => None,
        }
    }

    pub(super) fn update_card_hover(&mut self, mouse: MouseEvent, outcome: &mut ClientShellInput) {
        let hovered = (mouse.kind == MouseEventKind::Moved && self.card_hover_allowed())
            .then(|| self.card_at((mouse.column, mouse.row)))
            .flatten();
        if hovered != self.hovered_card {
            tracing::trace!(card = ?hovered, "sidebar card hover changed");
            self.hovered_card = hovered;
            outcome.repaint = true;
        }
    }

    pub(super) fn render_card_hover(&self, frame: &mut FrameData) {
        if !self.card_hover_allowed() {
            return;
        }
        let Some(rect) = self
            .hovered_card
            .as_ref()
            .and_then(|card| self.card_rect(card))
        else {
            return;
        };
        let muted = crate::protocol::color_to_u32(self.config.palette.overlay0);
        let hover = crate::protocol::color_to_u32(self.config.palette.hover_border_color());
        let mut highlight = |x: u16, y: u16| {
            if x >= frame.width || y >= frame.height {
                return;
            }
            let index = usize::from(y) * usize::from(frame.width) + usize::from(x);
            let Some(cell) = frame.cells.get_mut(index) else {
                return;
            };
            if cell.fg == muted && super::sidebar_card::is_card_border_symbol(&cell.symbol) {
                cell.fg = hover;
            }
        };
        let right = rect.right().saturating_sub(1);
        let bottom = rect.bottom().saturating_sub(1);
        for x in rect.x..rect.right() {
            highlight(x, rect.y);
            highlight(x, bottom);
        }
        for y in rect.y..rect.bottom() {
            highlight(rect.x, y);
            highlight(right, y);
        }
    }
}
