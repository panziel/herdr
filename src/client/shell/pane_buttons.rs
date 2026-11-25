// 窗格边框按钮（pane border buttons）：顶部边框右端的「移到其他标签页」「缩放切换」「关闭」。
// 按钮在客户端合成帧时写进服务端画好的边框行，绘制与命中区域出自同一份布局，
// 所以无论服务端版本如何，画出来的按钮都点得到，点得到的位置也一定画了按钮。
use super::*;
use crate::workspace::floating::pin_button_rect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PaneButtonAction {
    MoveToTab,
    ToggleZoom,
    Close,
}

#[derive(Clone, Debug)]
pub(super) struct PaneButtonHit {
    pub(super) rect: Rect,
    pub(super) pane_id: String,
    pub(super) action: PaneButtonAction,
}

/// ASCII only: ambiguous-width symbols render double-width in CJK terminals
/// and would shift the hit area away from the drawn label.
fn button_label(action: PaneButtonAction, zoomed: bool) -> &'static str {
    match action {
        PaneButtonAction::MoveToTab => "[->]",
        PaneButtonAction::ToggleZoom if zoomed => "[-]",
        PaneButtonAction::ToggleZoom => "[+]",
        PaneButtonAction::Close => "[x]",
    }
}

/// Buttons that do something for `pane_id`, left to right.
fn pane_buttons(
    snapshot: &ClientShellSnapshot,
    pane_id: &str,
) -> Vec<(PaneButtonAction, &'static str)> {
    let Some(pane) = snapshot.panes.iter().find(|pane| pane.pane_id == pane_id) else {
        return Vec::new();
    };
    let Some(tab) = snapshot.tabs.iter().find(|tab| tab.tab_id == pane.tab_id) else {
        return Vec::new();
    };
    let has_other_tab = snapshot
        .tabs
        .iter()
        .any(|other| other.workspace_id == tab.workspace_id && other.tab_id != tab.tab_id);
    let tab_pane_count = snapshot
        .panes
        .iter()
        .filter(|other| other.tab_id == tab.tab_id)
        .count();
    let mut buttons = Vec::with_capacity(3);
    if has_other_tab {
        buttons.push(PaneButtonAction::MoveToTab);
    }
    if tab_pane_count > 1 {
        buttons.push(PaneButtonAction::ToggleZoom);
    }
    buttons.push(PaneButtonAction::Close);
    buttons
        .into_iter()
        .map(|action| (action, button_label(action, tab.zoomed)))
        .collect()
}

pub(super) fn render_pane_buttons(
    frame: &mut FrameData,
    hits: &mut ShellHitMap,
    snapshot: &ClientShellSnapshot,
    config: &ClientShellConfig,
) {
    if !config.mouse_capture {
        return;
    }
    let mut placed = Vec::new();
    for index in 0..hits.panes.len() {
        let hit = &hits.panes[index];
        if hit.popup {
            continue;
        }
        let buttons = pane_buttons(snapshot, &hit.pane_id);
        // 命中表按从上到下排列，排在前面的浮动窗格会盖住本窗格的边框。
        placed.extend(place_buttons(frame, hit, &hits.panes[..index], &buttons));
    }
    hits.pane_buttons = placed;
}

/// Buttons end one border cell before the corner, or before a floating pane's
/// pin toggle. They are only drawn over plain `─` cells with one more `─` to
/// their left, so the title, split junctions and covering floating panes stay
/// intact.
fn place_buttons(
    frame: &mut FrameData,
    hit: &PaneHit,
    covering: &[PaneHit],
    buttons: &[(PaneButtonAction, &'static str)],
) -> Vec<PaneButtonHit> {
    let rect = hit.rect;
    let y = rect.y;
    if buttons.is_empty() || hit.inner_rect.y <= y || y >= frame.height {
        return Vec::new();
    }
    let end = hit
        .floating
        .and_then(|floating| pin_button_rect(rect, floating.pinned))
        .map_or(rect.right().saturating_sub(2), |pin| {
            pin.x.saturating_sub(1)
        })
        .min(frame.width);
    let width = buttons
        .iter()
        .map(|(_, label)| label.len() as u16)
        .sum::<u16>()
        .saturating_add(buttons.len() as u16 - 1);
    let Some(start) = end.checked_sub(width).filter(|start| *start > rect.x + 1) else {
        return Vec::new();
    };
    let row = usize::from(y) * usize::from(frame.width);
    // One extra `─` before the first button keeps it apart from the title.
    if !(start - 1..end).all(|x| frame.cells[row + usize::from(x)].symbol == "─")
        || covering
            .iter()
            .any(|cover| (start..end).any(|x| contains(cover.rect, (x, y))))
    {
        return Vec::new();
    }

    let mut x = start;
    let mut placed = Vec::with_capacity(buttons.len());
    for (action, label) in buttons {
        for (offset, symbol) in label.chars().enumerate() {
            frame.cells[row + usize::from(x) + offset].symbol = symbol.to_string();
        }
        let label_width = label.len() as u16;
        placed.push(PaneButtonHit {
            rect: Rect::new(x, y, label_width, 1),
            pane_id: hit.pane_id.clone(),
            action: *action,
        });
        x = x.saturating_add(label_width + 1);
    }
    placed
}

impl ClientShellState {
    pub(super) fn pane_button_at(&self, point: (u16, u16)) -> Option<PaneButtonHit> {
        let top_pane = self.pane_hit_at(point)?;
        self.hits
            .pane_buttons
            .iter()
            .find(|button| button.pane_id == top_pane.pane_id && contains(button.rect, point))
            .cloned()
    }

    pub(super) fn activate_pane_button(
        &mut self,
        button: PaneButtonHit,
        outcome: &mut ClientShellInput,
    ) {
        use crate::api::schema::{Method, PaneZoomMode, PaneZoomParams};

        tracing::debug!(pane = %button.pane_id, action = ?button.action, "pane border button pressed");
        match button.action {
            PaneButtonAction::MoveToTab => {
                self.open_pane_move_menu(
                    button.pane_id,
                    button.rect.x,
                    button.rect.y.saturating_add(1),
                );
            }
            PaneButtonAction::ToggleZoom => self.push_endpoint_method(
                Method::PaneZoom(PaneZoomParams {
                    pane_id: Some(button.pane_id),
                    mode: PaneZoomMode::Toggle,
                }),
                outcome,
            ),
            PaneButtonAction::Close => self.request_pane_close(button.pane_id, outcome),
        }
        outcome.repaint = true;
    }
}
