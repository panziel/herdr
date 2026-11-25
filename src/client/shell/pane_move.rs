// 把窗格移到同一工作区的另一个标签页（move pane to tab）：右键菜单、边框按钮和
// 拖到标签栏三个入口共用这里的候选列表与请求逻辑。
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PaneMoveTabChoice {
    pub(super) tab_id: String,
    pub(super) label: String,
}

impl ClientShellState {
    /// Tabs of the pane's workspace other than its own, in tab bar order.
    pub(super) fn pane_move_tab_choices(&self, pane_id: &str) -> Vec<PaneMoveTabChoice> {
        let Some(snapshot) = self.snapshot.as_deref() else {
            return Vec::new();
        };
        let Some(pane) = snapshot.panes.iter().find(|pane| pane.pane_id == pane_id) else {
            return Vec::new();
        };
        snapshot
            .tabs
            .iter()
            .filter(|tab| tab.workspace_id == pane.workspace_id && tab.tab_id != pane.tab_id)
            .map(|tab| PaneMoveTabChoice {
                tab_id: tab.tab_id.clone(),
                label: super::render::tab_label(tab),
            })
            .collect()
    }

    pub(super) fn open_pane_move_menu(&mut self, pane_id: String, x: u16, y: u16) {
        let tabs = self.pane_move_tab_choices(&pane_id);
        if tabs.is_empty() {
            return;
        }
        self.overlay = Some(ClientShellOverlay::ContextMenu(ClientContextMenuOverlay {
            target: ClientContextMenuTarget::PaneMoveTab { pane_id, tabs },
            x,
            y,
            highlighted: 0,
        }));
    }

    /// The server leaves zoomed tabs unchanged on `pane.move`, so both ends are
    /// unzoomed first; requests from one client run in order.
    pub(super) fn request_pane_move_to_tab(
        &mut self,
        pane_id: String,
        tab_id: String,
        outcome: &mut ClientShellInput,
    ) {
        use crate::api::schema::{
            Method, PaneMoveDestination, PaneMoveParams, PaneZoomMode, PaneZoomParams,
            SplitDirection,
        };

        let Some(snapshot) = self.snapshot.as_deref() else {
            return;
        };
        let Some(source_tab_id) = snapshot
            .panes
            .iter()
            .find(|pane| pane.pane_id == pane_id)
            .map(|pane| pane.tab_id.clone())
        else {
            return;
        };
        if source_tab_id == tab_id {
            return;
        }
        let tab_is_zoomed = |tab_id: &str| {
            snapshot
                .tabs
                .iter()
                .any(|tab| tab.tab_id == tab_id && tab.zoomed)
        };
        let mut unzoom_targets = Vec::new();
        if tab_is_zoomed(&source_tab_id) {
            unzoom_targets.push(pane_id.clone());
        }
        if tab_is_zoomed(&tab_id) {
            unzoom_targets.extend(
                snapshot
                    .panes
                    .iter()
                    .find(|pane| pane.tab_id == tab_id && pane.floating.is_none())
                    .map(|pane| pane.pane_id.clone()),
            );
        }
        tracing::info!(
            pane = %pane_id,
            from_tab = %source_tab_id,
            to_tab = %tab_id,
            unzoom = unzoom_targets.len(),
            "requesting pane move to tab"
        );
        for target in unzoom_targets {
            self.push_endpoint_method(
                Method::PaneZoom(PaneZoomParams {
                    pane_id: Some(target),
                    mode: PaneZoomMode::Off,
                }),
                outcome,
            );
        }
        self.push_endpoint_method(
            Method::PaneMove(PaneMoveParams {
                pane_id,
                destination: PaneMoveDestination::Tab {
                    tab_id,
                    target_pane_id: None,
                    split: SplitDirection::Right,
                    ratio: None,
                },
                focus: true,
            }),
            outcome,
        );
    }

    /// Tab of the tab bar under `point` that `pane_id` can be dropped into.
    pub(super) fn pane_drop_tab_at(&self, pane_id: &str, point: (u16, u16)) -> Option<String> {
        let tab_id = self
            .hits
            .tabs
            .iter()
            .find(|(rect, _)| contains(*rect, point))
            .map(|(_, tab_id)| tab_id)?;
        self.pane_move_tab_choices(pane_id)
            .into_iter()
            .find(|choice| &choice.tab_id == tab_id)
            .map(|choice| choice.tab_id)
    }
}
