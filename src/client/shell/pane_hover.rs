// Pane hover（窗格悬停）只在客户端改变边框颜色，不改变终端内容或焦点。
use super::*;
use crossterm::event::{MouseEvent, MouseEventKind};

fn border_symbol(symbol: &str) -> bool {
    matches!(
        symbol,
        "│" | "─"
            | "╭"
            | "╮"
            | "╰"
            | "╯"
            | "┌"
            | "┐"
            | "└"
            | "┘"
            | "┼"
            | "┤"
            | "├"
            | "┴"
            | "┬"
    )
}

impl ClientShellState {
    fn pane_hover_allowed(&self) -> bool {
        self.config.mouse_capture
            && self.mode == ClientShellMode::Terminal
            && self.overlay.is_none()
            && self.popup_terminal_id.is_none()
            && self.outer_focused != Some(false)
            && self.chrome_drag.is_none()
            && self.pane_mouse_gesture.is_none()
    }

    pub(super) fn update_pane_hover(&mut self, mouse: MouseEvent, outcome: &mut ClientShellInput) {
        let hovered = (mouse.kind == MouseEventKind::Moved && self.pane_hover_allowed())
            .then(|| {
                self.pane_hit_at((mouse.column, mouse.row)).filter(|hit| {
                    !hit.popup
                        && self.snapshot.as_ref().is_some_and(|snapshot| {
                            snapshot.focused_pane_id.as_deref() != Some(hit.pane_id.as_str())
                        })
                })
            })
            .flatten()
            .map(|hit| hit.pane_id.as_str());
        if hovered != self.hovered_pane_id.as_deref() {
            tracing::trace!(pane = hovered, "pane hover changed");
            self.hovered_pane_id = hovered.map(str::to_owned);
            outcome.repaint = true;
        }
    }

    pub(super) fn render_pane_hover(&self, frame: &mut FrameData) {
        if !self.pane_hover_allowed() {
            return;
        }
        let Some(hovered) = self.hovered_pane_id.as_deref() else {
            return;
        };
        let Some(index) = self.hits.panes.iter().position(|hit| {
            hit.pane_id == hovered
                && self.snapshot.as_ref().is_some_and(|snapshot| {
                    snapshot.focused_pane_id.as_deref() != Some(hit.pane_id.as_str())
                })
        }) else {
            return;
        };
        let hit = &self.hits.panes[index];
        // Hits are topmost first; floating panes listed earlier cover this border.
        let covering: Vec<Rect> = self.hits.panes[..index]
            .iter()
            .map(|hit| hit.rect)
            .collect();
        let muted = crate::protocol::color_to_u32(self.config.palette.overlay0);
        let hover = crate::protocol::color_to_u32(self.config.palette.hover_border_color());
        let mut highlight = |x: u16, y: u16| {
            if x >= frame.width
                || y >= frame.height
                || contains(hit.inner_rect, (x, y))
                || covering.iter().any(|rect| contains(*rect, (x, y)))
                || hit
                    .scrollbar_rect
                    .is_some_and(|rect| contains(rect, (x, y)))
            {
                return;
            }
            let index = usize::from(y) * usize::from(frame.width) + usize::from(x);
            let Some(cell) = frame.cells.get_mut(index) else {
                return;
            };
            if cell.fg == muted && border_symbol(&cell.symbol) {
                cell.fg = hover;
            }
        };
        let shared_right = hit.inner_rect.right() == hit.rect.right()
            && self.hits.pane_splits.iter().any(|split| {
                split.direction == crate::protocol::PaneSurfaceSplitDirection::Horizontal
                    && split.pos == hit.rect.right()
                    && split.area.y < hit.rect.bottom()
                    && split.area.bottom() > hit.rect.y
            });
        let shared_bottom = hit.inner_rect.bottom() == hit.rect.bottom()
            && self.hits.pane_splits.iter().any(|split| {
                split.direction == crate::protocol::PaneSurfaceSplitDirection::Vertical
                    && split.pos == hit.rect.bottom()
                    && split.area.x < hit.rect.right()
                    && split.area.right() > hit.rect.x
            });
        for x in hit.rect.x..hit.rect.right() {
            highlight(x, hit.rect.y);
            highlight(x, hit.rect.bottom().saturating_sub(1));
            if shared_bottom {
                highlight(x, hit.rect.bottom());
            }
        }
        for y in hit.rect.y..hit.rect.bottom() {
            highlight(hit.rect.x, y);
            highlight(hit.rect.right().saturating_sub(1), y);
            if shared_right {
                highlight(hit.rect.right(), y);
            }
        }
    }
}
