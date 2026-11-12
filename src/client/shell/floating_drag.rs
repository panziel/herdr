//! 浮动窗格（floating pane）的鼠标操作：拖动顶部边框移动窗格，拖动左、右、下边框调整大小，
//! 点击顶部边框右侧的 pin 按钮切换固定状态。

use ratatui::layout::Rect;

use super::{contains, ClientChromeDrag, ClientShellInput, ClientShellState, PaneHit};
use crate::workspace::floating::{
    pin_button_rect, FloatingRect, MIN_FLOATING_HEIGHT, MIN_FLOATING_WIDTH,
};

/// Ignore geometry updates closer together than this while dragging, so a fast
/// pointer does not flood the endpoint with requests.
pub(super) const FLOATING_DRAG_SEND_INTERVAL: std::time::Duration =
    std::time::Duration::from_millis(33);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FloatingDragKind {
    Move,
    Resize {
        left: bool,
        right: bool,
        bottom: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FloatingBorderPress {
    Pin,
    Drag(FloatingDragKind),
}

/// What a left press on `hit`'s frame does, or `None` inside its content.
pub(super) fn floating_border_press(
    hit: &PaneHit,
    pinned: bool,
    point: (u16, u16),
) -> Option<FloatingBorderPress> {
    if !contains(hit.rect, point) || contains(hit.inner_rect, point) {
        return None;
    }
    if pin_button_rect(hit.rect, pinned).is_some_and(|button| contains(button, point)) {
        return Some(FloatingBorderPress::Pin);
    }
    let rect = hit.rect;
    if point.1 == rect.y {
        return Some(FloatingBorderPress::Drag(FloatingDragKind::Move));
    }
    Some(FloatingBorderPress::Drag(FloatingDragKind::Resize {
        left: point.0 == rect.x,
        right: point.0 + 1 == rect.right(),
        bottom: point.1 + 1 == rect.bottom(),
    }))
}

#[derive(Debug, Clone)]
pub(super) struct FloatingDrag {
    pub(super) pane_id: String,
    pub(super) kind: FloatingDragKind,
    start_point: (u16, u16),
    start_rect: Rect,
    pub(super) last_sent: Option<Rect>,
    pub(super) last_sent_at: Option<std::time::Instant>,
}

impl FloatingDrag {
    pub(super) fn new(
        pane_id: String,
        kind: FloatingDragKind,
        start_point: (u16, u16),
        start_rect: Rect,
    ) -> Self {
        Self {
            pane_id,
            kind,
            start_point,
            start_rect,
            last_sent: None,
            last_sent_at: None,
        }
    }

    /// Whether `rect` differs from what the endpoint last received.
    pub(super) fn needs_send(&self, rect: Rect) -> bool {
        self.last_sent
            .map_or(rect != self.start_rect, |sent| sent != rect)
    }

    /// Frame rect after the pointer moved to `point`, kept inside `surface`.
    pub(super) fn rect_at(&self, point: (u16, u16), surface: Rect) -> Rect {
        let dx = i32::from(point.0) - i32::from(self.start_point.0);
        let dy = i32::from(point.1) - i32::from(self.start_point.1);
        let start = self.start_rect;
        match self.kind {
            FloatingDragKind::Move => {
                let width = start.width.min(surface.width);
                let height = start.height.min(surface.height);
                Rect::new(
                    shifted(start.x, dx, surface.x, surface.right() - width),
                    shifted(start.y, dy, surface.y, surface.bottom() - height),
                    width,
                    height,
                )
            }
            FloatingDragKind::Resize {
                left,
                right,
                bottom,
            } => {
                let mut x0 = start.x;
                let mut x1 = start.right();
                let mut y1 = start.bottom();
                if left {
                    x0 = shifted(
                        x0,
                        dx,
                        surface.x,
                        x1.saturating_sub(MIN_FLOATING_WIDTH).max(surface.x),
                    );
                }
                if right {
                    x1 = shifted(
                        x1,
                        dx,
                        (x0 + MIN_FLOATING_WIDTH).min(surface.right()),
                        surface.right(),
                    );
                }
                if bottom {
                    y1 = shifted(
                        y1,
                        dy,
                        (start.y + MIN_FLOATING_HEIGHT).min(surface.bottom()),
                        surface.bottom(),
                    );
                }
                Rect::new(x0, start.y, x1 - x0, y1 - start.y)
            }
        }
    }
}

impl ClientShellState {
    /// Topmost pane whose frame contains `point`.
    pub(super) fn pane_hit_at(&self, point: (u16, u16)) -> Option<&PaneHit> {
        self.hits.panes.iter().find(|hit| contains(hit.rect, point))
    }

    /// Whether `point` belongs to a pane drawn above the pane `pane_id`.
    pub(super) fn pane_covered_at(&self, pane_id: &str, point: (u16, u16)) -> bool {
        self.pane_hit_at(point)
            .is_some_and(|top| top.pane_id != pane_id)
    }

    /// Handle a left press on a floating pane's frame. Returns false when the
    /// press is not on a floating frame and should take the normal pane path.
    pub(super) fn press_floating_border(
        &mut self,
        hit: &PaneHit,
        point: (u16, u16),
        outcome: &mut ClientShellInput,
    ) -> bool {
        let Some(floating) = hit.floating else {
            return false;
        };
        let Some(press) = floating_border_press(hit, floating.pinned, point) else {
            return false;
        };
        tracing::debug!(pane = %hit.pane_id, ?press, "floating pane frame pressed");
        self.push_endpoint_method(
            crate::api::schema::Method::PaneFocus(crate::api::schema::PaneTarget {
                pane_id: hit.pane_id.clone(),
            }),
            outcome,
        );
        match press {
            FloatingBorderPress::Pin => self.push_endpoint_method(
                crate::api::schema::Method::PanePin(crate::api::schema::PanePinParams {
                    pane_id: Some(hit.pane_id.clone()),
                    mode: crate::api::schema::FloatingFlagMode::Toggle,
                }),
                outcome,
            ),
            FloatingBorderPress::Drag(kind) => {
                self.chrome_drag = Some(ClientChromeDrag::FloatingPane(FloatingDrag::new(
                    hit.pane_id.clone(),
                    kind,
                    point,
                    hit.rect,
                )));
            }
        }
        true
    }

    /// Send the dragged floating pane's new geometry. Intermediate updates are
    /// throttled; the final one on release always goes out when it changed.
    pub(super) fn drag_floating_pane(
        &mut self,
        point: (u16, u16),
        finished: bool,
        outcome: &mut ClientShellInput,
    ) {
        let surface = self.hits.pane_surface;
        let Some(ClientChromeDrag::FloatingPane(drag)) = self.chrome_drag.as_mut() else {
            return;
        };
        let rect = drag.rect_at(point, surface);
        let now = std::time::Instant::now();
        let throttled = !finished
            && drag
                .last_sent_at
                .is_some_and(|last| now.duration_since(last) < FLOATING_DRAG_SEND_INTERVAL);
        if !drag.needs_send(rect) || throttled {
            return;
        }
        drag.last_sent = Some(rect);
        drag.last_sent_at = Some(now);
        let pane_id = drag.pane_id.clone();
        self.push_endpoint_method(
            crate::api::schema::Method::PaneFloatingGeometry(
                crate::api::schema::PaneFloatingGeometryParams {
                    pane_id,
                    geometry: floating_geometry(rect, surface),
                },
            ),
            outcome,
        );
    }
}

/// `value + delta`, clamped to `min..=max`. `max` below `min` pins to `min`.
fn shifted(value: u16, delta: i32, min: u16, max: u16) -> u16 {
    let max = max.max(min);
    let moved = (i32::from(value) + delta).clamp(i32::from(min), i32::from(max));
    u16::try_from(moved).unwrap_or(min)
}

pub(super) fn floating_geometry(rect: Rect, surface: Rect) -> crate::api::schema::FloatingGeometry {
    let rect = FloatingRect::from_cells(rect, surface);
    crate::api::schema::FloatingGeometry {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(rect: Rect) -> PaneHit {
        PaneHit {
            rect,
            inner_rect: Rect::new(rect.x + 1, rect.y + 1, rect.width - 2, rect.height - 2),
            scrollbar_rect: None,
            scroll: None,
            pane_id: "pane_2".into(),
            popup: false,
            floating: Some(crate::protocol::ClientShellFloatingPane { pinned: false }),
            mouse_reporting: false,
            sgr_pixel_mouse: false,
            pixel_width: 0,
            pixel_height: 0,
        }
    }

    #[test]
    fn border_presses_map_to_move_resize_and_pin() {
        let pane = hit(Rect::new(10, 5, 40, 12));
        assert_eq!(floating_border_press(&pane, false, (20, 8)), None);
        assert_eq!(
            floating_border_press(&pane, false, (20, 5)),
            Some(FloatingBorderPress::Drag(FloatingDragKind::Move))
        );
        let pin = pin_button_rect(pane.rect, false).unwrap();
        assert_eq!(
            floating_border_press(&pane, false, (pin.x, pin.y)),
            Some(FloatingBorderPress::Pin)
        );
        assert_eq!(
            floating_border_press(&pane, false, (49, 16)),
            Some(FloatingBorderPress::Drag(FloatingDragKind::Resize {
                left: false,
                right: true,
                bottom: true,
            }))
        );
        assert_eq!(
            floating_border_press(&pane, false, (10, 9)),
            Some(FloatingBorderPress::Drag(FloatingDragKind::Resize {
                left: true,
                right: false,
                bottom: false,
            }))
        );
    }

    #[test]
    fn move_drag_follows_pointer_and_stays_inside_surface() {
        let surface = Rect::new(0, 1, 100, 40);
        let drag = FloatingDrag::new(
            "pane_2".into(),
            FloatingDragKind::Move,
            (20, 5),
            Rect::new(10, 5, 40, 12),
        );
        assert_eq!(drag.rect_at((25, 7), surface), Rect::new(15, 7, 40, 12));
        assert_eq!(drag.rect_at((0, 0), surface), Rect::new(0, 1, 40, 12));
        assert_eq!(drag.rect_at((99, 60), surface), Rect::new(60, 29, 40, 12));
    }

    #[test]
    fn resize_drag_moves_only_grabbed_edges_and_keeps_minimum_size() {
        let surface = Rect::new(0, 0, 100, 40);
        let start = Rect::new(10, 5, 40, 12);
        let corner = FloatingDrag::new(
            "pane_2".into(),
            FloatingDragKind::Resize {
                left: false,
                right: true,
                bottom: true,
            },
            (49, 16),
            start,
        );
        assert_eq!(corner.rect_at((59, 20), surface), Rect::new(10, 5, 50, 16));
        let shrunk = corner.rect_at((0, 0), surface);
        assert_eq!(shrunk.width, MIN_FLOATING_WIDTH);
        assert_eq!(shrunk.height, MIN_FLOATING_HEIGHT);

        let left = FloatingDrag::new(
            "pane_2".into(),
            FloatingDragKind::Resize {
                left: true,
                right: false,
                bottom: false,
            },
            (10, 9),
            start,
        );
        assert_eq!(left.rect_at((4, 9), surface), Rect::new(4, 5, 46, 12));
        assert_eq!(left.rect_at((99, 9), surface).right(), start.right());
    }

    #[test]
    fn geometry_is_relative_to_the_pane_surface() {
        let surface = Rect::new(20, 1, 100, 40);
        let geometry = floating_geometry(Rect::new(40, 11, 50, 20), surface);
        assert_eq!(
            (geometry.x, geometry.y, geometry.width, geometry.height),
            (200, 250, 500, 500)
        );
    }
}
