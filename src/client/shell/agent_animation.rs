// Agent animation（智能体状态动画）缓存可见状态单元，只提交客户端局部更新。
use super::*;
use crate::api::schema::AgentStatus;
use crate::protocol::{CellData, PaneSurfacePatchRow};
use std::time::{Duration, Instant};

pub(super) struct AgentIndicator {
    pub(super) rect: Rect,
    pub(super) status: AgentStatus,
    pub(super) spinner: bool,
}

struct AnimatedCell {
    x: u16,
    y: u16,
    base: CellData,
    status: AgentStatus,
    spinner: bool,
}

#[derive(Default)]
pub(super) struct AgentAnimation {
    phase: u8,
    deadline: Option<Instant>,
    cells: Vec<AnimatedCell>,
}

impl AgentAnimation {
    pub(super) fn deadline(&self) -> Option<Instant> {
        self.deadline
    }

    pub(super) fn clear(&mut self) {
        self.deadline = None;
        self.cells.clear();
    }
}

impl AnimatedCell {
    fn at_phase(&self, phase: u8) -> CellData {
        let mut cell = self.base.clone();
        match self.status {
            AgentStatus::Working if self.spinner => {
                cell.symbol = ["◐", "◓", "◑", "◒"][usize::from(phase % 4)].into();
            }
            AgentStatus::Blocked if phase / 5 % 2 == 1 => {
                cell.modifier |= Modifier::DIM.bits();
            }
            _ => {}
        }
        cell
    }
}

pub(super) fn indicator_icon(
    status: AgentStatus,
    style: crate::config::StatusIndicatorStyle,
) -> &'static str {
    match status {
        AgentStatus::Working => "◐",
        AgentStatus::Blocked => "!",
        AgentStatus::Done => "✓",
        AgentStatus::Unknown => "?",
        _ => status_icon(status, style),
    }
}

impl ClientShellState {
    pub(crate) fn pause_agent_animation(&mut self) {
        self.agent_animation.clear();
    }

    pub(super) fn sync_agent_animation(&mut self, frame: &mut FrameData) {
        let mobile_switcher = self.mobile_layout_active() && self.mode == ClientShellMode::Navigate;
        let animation = &mut self.agent_animation;
        animation.cells.clear();
        if self.overlay.is_none()
            && self.popup_terminal_id.is_none()
            && self.outer_focused != Some(false)
            && !mobile_switcher
        {
            for indicator in &self.hits.agent_indicators {
                if !matches!(
                    indicator.status,
                    AgentStatus::Working | AgentStatus::Blocked
                ) {
                    continue;
                }
                for x in indicator.rect.x..indicator.rect.right().min(frame.width) {
                    let y = indicator.rect.y;
                    if y >= frame.height {
                        continue;
                    }
                    if contains(self.hits.notification_toast, (x, y)) {
                        continue;
                    }
                    let index = usize::from(y) * usize::from(frame.width) + usize::from(x);
                    let base = frame.cells[index].clone();
                    let cell = AnimatedCell {
                        x,
                        y,
                        base,
                        status: indicator.status,
                        spinner: indicator.spinner,
                    };
                    frame.cells[index] = cell.at_phase(animation.phase);
                    animation.cells.push(cell);
                }
            }
        }
        if animation.cells.is_empty() {
            animation.deadline = None;
        } else if animation.deadline.is_none() {
            animation.deadline = Some(Instant::now() + Duration::from_millis(120));
        }
    }

    pub(crate) fn tick_agent_animation(&mut self, now: Instant) -> Vec<PaneSurfacePatchRow> {
        let animation = &mut self.agent_animation;
        if self.outer_focused == Some(false)
            || self.overlay.is_some()
            || self.popup_terminal_id.is_some()
            || self.pane_surface.is_none()
            || self.pending_pane_surface.is_some()
            || self.pane_surface_generation != self.active_snapshot_generation
            || self
                .snapshot
                .as_ref()
                .zip(self.pane_surface.as_ref())
                .is_none_or(|(snapshot, surface)| {
                    snapshot.boot_id != surface.boot_id
                        || snapshot.revision != surface.projection_revision
                })
        {
            animation.clear();
            return Vec::new();
        }
        if animation.deadline.is_none_or(|deadline| now < deadline) {
            return Vec::new();
        }
        let previous_phase = animation.phase;
        animation.phase = (animation.phase + 1) % 20;
        animation.deadline = Some(now + Duration::from_millis(120));
        animation
            .cells
            .iter()
            .filter_map(|cell| {
                let next = cell.at_phase(animation.phase);
                (next != cell.at_phase(previous_phase)).then_some(PaneSurfacePatchRow {
                    x: cell.x,
                    y: cell.y,
                    cells: vec![next],
                })
            })
            .collect()
    }
}
