//! 浮动窗格（floating pane）的 API 处理：显示/隐藏浮动层、新建、浮出/嵌入、固定（pin）、移动和改变大小。

use std::path::PathBuf;

use crate::api::schema::{
    EventData, EventEnvelope, EventKind, FloatingFlagMode, FloatingLayerMode,
    PaneFloatingCreateParams, PaneFloatingGeometryParams, PaneFloatingMoveParams,
    PaneFloatingSetParams, PaneInfo, PanePinParams, ResponseResult, TabFloatingParams,
    TabFloatingResult,
};
use crate::app::App;
use crate::layout::{NavDirection, PaneId};
use crate::workspace::FloatingRect;

use super::panes::pane_not_found;
use super::responses::{encode_error, encode_success};

/// Keyboard moves shift by a few cells; columns are narrower than rows.
const DEFAULT_MOVE_COLUMNS: u16 = 4;
const DEFAULT_MOVE_ROWS: u16 = 2;

impl App {
    pub(super) fn handle_tab_floating_set(
        &mut self,
        id: String,
        params: TabFloatingParams,
    ) -> String {
        let target = match params.tab_id.as_deref() {
            Some(tab_id) => self.parse_tab_id(tab_id),
            None => self.state.active.and_then(|ws_idx| {
                let ws = self.state.workspaces.get(ws_idx)?;
                Some((ws_idx, ws.active_tab_index()))
            }),
        };
        let Some((ws_idx, tab_idx)) = target else {
            return encode_error(id, "tab_not_found", "tab not found");
        };
        let Some(tab) = self.floating_tab(ws_idx, tab_idx) else {
            return encode_error(id, "tab_not_found", "tab not found");
        };
        let show = match params.mode {
            FloatingLayerMode::Toggle => !tab.floating.is_visible(),
            FloatingLayerMode::Show => true,
            FloatingLayerMode::Hide => false,
        };
        let previous_focus = self.state.current_pane_focus_target();
        let mut created_pane = None;
        let changed = if show && tab.floating.is_empty() {
            if params.create_if_empty {
                match self.create_floating_pane(ws_idx, tab_idx, None, true, Vec::new()) {
                    Ok(pane) => {
                        created_pane = Some(Box::new(pane));
                        true
                    }
                    Err(message) => {
                        return encode_error(id, "pane_floating_create_failed", message)
                    }
                }
            } else {
                false
            }
        } else {
            self.state.workspaces[ws_idx].tabs[tab_idx].set_floating_visible(show)
        };

        let Some(tab) = self.floating_tab(ws_idx, tab_idx) else {
            return encode_error(id, "tab_not_found", "tab not found");
        };
        let visible = tab.floating.is_visible();
        let focused = tab.focused_pane();
        tracing::info!(
            ws_idx,
            tab_idx,
            visible,
            changed,
            "floating layer visibility set"
        );
        // Creating the first pane already recorded focus, saved and announced the layout.
        if changed && created_pane.is_none() {
            self.state
                .record_pane_focus_change(previous_focus, ws_idx, focused);
            self.state.mark_session_dirty();
            self.schedule_session_save();
            self.emit_layout_updated_event(ws_idx, tab_idx);
        }
        self.state.mode = crate::app::Mode::Terminal;
        let (Some(tab_id), Some(focused_pane_id)) = (
            self.public_tab_id(ws_idx, tab_idx),
            self.public_pane_id(ws_idx, focused),
        ) else {
            return encode_error(id, "tab_not_found", "tab not found");
        };
        encode_success(
            id,
            ResponseResult::TabFloating {
                floating: TabFloatingResult {
                    tab_id,
                    changed,
                    visible,
                    focused_pane_id,
                    created_pane,
                },
            },
        )
    }

    pub(super) fn handle_pane_floating_create(
        &mut self,
        id: String,
        params: PaneFloatingCreateParams,
    ) -> String {
        let target = match params.tab_id.as_deref() {
            Some(tab_id) => self.parse_tab_id(tab_id),
            None => self.state.active.and_then(|ws_idx| {
                let ws = self.state.workspaces.get(ws_idx)?;
                Some((ws_idx, ws.active_tab_index()))
            }),
        };
        let Some((ws_idx, tab_idx)) = target else {
            return encode_error(id, "tab_not_found", "tab not found");
        };
        let extra_env = match super::env::normalize_launch_env(params.env) {
            Ok(env) => env,
            Err((code, message)) => return encode_error(id, &code, message),
        };
        match self.create_floating_pane(ws_idx, tab_idx, params.cwd, params.focus, extra_env) {
            Ok(pane) => encode_success(id, ResponseResult::PaneInfo { pane }),
            Err(message) => encode_error(id, "pane_floating_create_failed", message),
        }
    }

    pub(super) fn handle_pane_floating_set(
        &mut self,
        id: String,
        params: PaneFloatingSetParams,
    ) -> String {
        let Some((ws_idx, tab_idx, pane_id)) = self.resolve_tab_pane(params.pane_id.as_deref())
        else {
            return pane_not_found(id, params.pane_id.as_deref().unwrap_or_default());
        };
        let tab = &self.state.workspaces[ws_idx].tabs[tab_idx];
        let floating = match params.mode {
            FloatingFlagMode::Toggle => !tab.is_floating(pane_id),
            FloatingFlagMode::On => true,
            FloatingFlagMode::Off => false,
        };
        let previous_focus = self.state.current_pane_focus_target();
        let tab = &mut self.state.workspaces[ws_idx].tabs[tab_idx];
        let changed = if floating == tab.is_floating(pane_id) {
            false
        } else if floating {
            if !tab.float_pane(pane_id) {
                return encode_error(
                    id,
                    "pane_floating_unavailable",
                    "the last tiled pane of a tab cannot float",
                );
            }
            true
        } else {
            tab.embed_pane(pane_id)
        };
        if changed {
            tracing::info!(
                pane = pane_id.raw(),
                floating,
                "pane floating state changed"
            );
            self.state.switch_workspace_tab(ws_idx, tab_idx);
            self.state
                .record_pane_focus_change(previous_focus, ws_idx, pane_id);
            self.state.mode = crate::app::Mode::Terminal;
        }
        self.finish_floating_pane_change(id, ws_idx, tab_idx, pane_id, changed)
    }

    pub(super) fn handle_pane_pin(&mut self, id: String, params: PanePinParams) -> String {
        let Some((ws_idx, tab_idx, pane_id)) = self.resolve_tab_pane(params.pane_id.as_deref())
        else {
            return pane_not_found(id, params.pane_id.as_deref().unwrap_or_default());
        };
        let layer = &mut self.state.workspaces[ws_idx].tabs[tab_idx].floating;
        let Some(current) = layer.get(pane_id).map(|floating| floating.pinned) else {
            return encode_error(id, "pane_not_floating", "pane is not floating");
        };
        let pinned = match params.mode {
            FloatingFlagMode::Toggle => !current,
            FloatingFlagMode::On => true,
            FloatingFlagMode::Off => false,
        };
        let changed = layer.set_pinned(pane_id, pinned);
        if changed {
            tracing::info!(pane = pane_id.raw(), pinned, "floating pane pin changed");
        }
        self.finish_floating_pane_change(id, ws_idx, tab_idx, pane_id, changed)
    }

    pub(super) fn handle_pane_floating_move(
        &mut self,
        id: String,
        params: PaneFloatingMoveParams,
    ) -> String {
        let Some((ws_idx, tab_idx, pane_id)) = self.resolve_tab_pane(params.pane_id.as_deref())
        else {
            return pane_not_found(id, params.pane_id.as_deref().unwrap_or_default());
        };
        let direction: NavDirection = params.direction.into();
        let cells = params.cells.unwrap_or(match direction {
            NavDirection::Left | NavDirection::Right => DEFAULT_MOVE_COLUMNS,
            NavDirection::Up | NavDirection::Down => DEFAULT_MOVE_ROWS,
        });
        let area = self.state.new_pane_area();
        let layer = &mut self.state.workspaces[ws_idx].tabs[tab_idx].floating;
        let Some(rect) = layer.get(pane_id).map(|floating| floating.rect) else {
            return encode_error(id, "pane_not_floating", "pane is not floating");
        };
        let changed = layer.set_rect(pane_id, rect.moved(direction, cells, area));
        self.finish_floating_pane_change(id, ws_idx, tab_idx, pane_id, changed)
    }

    pub(super) fn handle_pane_floating_geometry(
        &mut self,
        id: String,
        params: PaneFloatingGeometryParams,
    ) -> String {
        let Some((ws_idx, tab_idx, pane_id)) = self.resolve_tab_pane(Some(&params.pane_id)) else {
            return pane_not_found(id, &params.pane_id);
        };
        let geometry = params.geometry;
        let layer = &mut self.state.workspaces[ws_idx].tabs[tab_idx].floating;
        if !layer.contains(pane_id) {
            return encode_error(id, "pane_not_floating", "pane is not floating");
        }
        let rect = FloatingRect::new(geometry.x, geometry.y, geometry.width, geometry.height);
        let changed = layer.set_rect(pane_id, rect);
        self.finish_floating_pane_change(id, ws_idx, tab_idx, pane_id, changed)
    }

    /// Grow or shrink a floating pane for `pane.resize`. `amount` is the share
    /// of the tab area the far edge moves by, matching split resizes.
    pub(super) fn resize_floating_pane(
        &mut self,
        ws_idx: usize,
        tab_idx: usize,
        pane_id: PaneId,
        direction: NavDirection,
        amount: f32,
    ) -> bool {
        let area = self.state.new_pane_area();
        let length = match direction {
            NavDirection::Left | NavDirection::Right => area.width,
            NavDirection::Up | NavDirection::Down => area.height,
        };
        let cells = (f32::from(length) * amount).round().max(1.0) as u16;
        let Some(layer) = self
            .state
            .workspaces
            .get_mut(ws_idx)
            .and_then(|ws| ws.tabs.get_mut(tab_idx))
            .map(|tab| &mut tab.floating)
        else {
            return false;
        };
        let Some(rect) = layer.get(pane_id).map(|floating| floating.rect) else {
            return false;
        };
        let changed = layer.set_rect(pane_id, rect.resized(direction, cells, area));
        if changed {
            self.state.mark_session_dirty();
        }
        changed
    }

    fn create_floating_pane(
        &mut self,
        ws_idx: usize,
        tab_idx: usize,
        cwd: Option<String>,
        focus: bool,
        extra_env: Vec<(String, String)>,
    ) -> Result<PaneInfo, String> {
        let (rows, cols) = self
            .state
            .new_pane_size(crate::ui::NewPanePlacement::Floating { ws_idx, tab_idx });
        let cwd = cwd.map(PathBuf::from).or_else(|| {
            let focused = self.floating_tab(ws_idx, tab_idx)?.focused_pane();
            let follow_cwd = self.launch_cwd_for_pane_in_workspace(ws_idx, focused);
            Some(self.resolve_new_terminal_cwd(follow_cwd))
        });
        let default_shell = self.state.default_shell.clone();
        let scrollback_limit_bytes = self.state.pane_scrollback_limit_bytes;
        let host_terminal_theme = self.state.host_terminal_theme;
        let host_terminal_appearance = self.pane_host_terminal_appearance();
        let shell_config = crate::pane::PaneShellConfig::new(&default_shell, self.state.shell_mode);
        let previous_focus = self.state.current_pane_focus_target();
        let new_pane = self
            .state
            .workspaces
            .get_mut(ws_idx)
            .and_then(|ws| {
                ws.create_floating_pane(
                    tab_idx,
                    focus,
                    rows,
                    cols,
                    cwd,
                    scrollback_limit_bytes,
                    host_terminal_theme,
                    host_terminal_appearance,
                    shell_config,
                    extra_env,
                )
            })
            .ok_or_else(|| "tab not found".to_owned())?
            .map_err(|err| err.to_string())?;
        let pane_id = new_pane.pane_id;
        tracing::info!(
            pane = pane_id.raw(),
            ws_idx,
            tab_idx,
            "floating pane created"
        );
        if focus {
            self.state.switch_workspace_tab(ws_idx, tab_idx);
            self.state
                .record_pane_focus_change(previous_focus, ws_idx, pane_id);
            self.state.mode = crate::app::Mode::Terminal;
        }
        self.terminal_runtimes
            .insert(new_pane.terminal.id.clone(), new_pane.runtime);
        self.state.remove_alias_shadowed_by_new_pane(pane_id);
        self.state
            .terminals
            .insert(new_pane.terminal.id.clone(), new_pane.terminal);
        self.state.mark_session_dirty();
        self.schedule_session_save();
        let pane = self
            .pane_info(ws_idx, pane_id)
            .ok_or_else(|| "created floating pane disappeared".to_owned())?;
        self.emit_event(EventEnvelope {
            event: EventKind::PaneCreated,
            data: EventData::PaneCreated { pane: pane.clone() },
        });
        self.emit_layout_updated_event(ws_idx, tab_idx);
        Ok(pane)
    }

    fn finish_floating_pane_change(
        &mut self,
        id: String,
        ws_idx: usize,
        tab_idx: usize,
        pane_id: PaneId,
        changed: bool,
    ) -> String {
        if changed {
            self.state.mark_session_dirty();
            self.schedule_session_save();
            self.emit_layout_updated_event(ws_idx, tab_idx);
        }
        match self.pane_info(ws_idx, pane_id) {
            Some(pane) => encode_success(id, ResponseResult::PaneInfo { pane }),
            None => encode_error(id, "pane_not_found", "pane not found"),
        }
    }

    /// Resolve an optional public pane id (default: the focused pane) to its tab.
    fn resolve_tab_pane(&self, pane_id: Option<&str>) -> Option<(usize, usize, PaneId)> {
        let (ws_idx, pane_id) = self.resolve_optional_pane(pane_id)?;
        let tab_idx = self.state.workspaces[ws_idx].find_tab_index_for_pane(pane_id)?;
        Some((ws_idx, tab_idx, pane_id))
    }

    fn floating_tab(&self, ws_idx: usize, tab_idx: usize) -> Option<&crate::workspace::Tab> {
        self.state.workspaces.get(ws_idx)?.tabs.get(tab_idx)
    }
}

#[cfg(test)]
mod tests {
    use crate::api::schema::{
        FloatingFlagMode, FloatingGeometry, FloatingLayerMode, PaneDirection,
        PaneFloatingGeometryParams, PaneFloatingMoveParams, PaneFloatingSetParams, PanePinParams,
        PaneResizeParams, ResponseResult, SuccessResponse, TabFloatingParams,
    };
    use crate::app::App;
    use crate::config::Config;
    use crate::layout::PaneId;
    use crate::workspace::Workspace;

    fn app_with_floating_pane() -> (App, PaneId, PaneId) {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &Config::default(),
            crate::app::AppPolicy::TEST,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        let mut workspace = Workspace::test_new("floating");
        let root = workspace.tabs[0].root_pane;
        let floating = workspace.test_add_floating(true);
        app.state.workspaces = vec![workspace];
        app.state.active = Some(0);
        app.state.ensure_test_terminals();
        crate::ui::compute_view_with_runtime_registry(
            &mut app.state,
            &crate::terminal::TerminalRuntimeRegistry::new(),
            ratatui::layout::Rect::new(0, 0, 100, 40),
        );
        (app, root, floating)
    }

    fn success(response: &str) -> ResponseResult {
        serde_json::from_str::<SuccessResponse>(response)
            .unwrap_or_else(|_| panic!("expected success, got {response}"))
            .result
    }

    fn error_code(response: &str) -> String {
        let value: serde_json::Value = serde_json::from_str(response).unwrap();
        value["error"]["code"]
            .as_str()
            .unwrap_or_default()
            .to_owned()
    }

    fn floating_rect(app: &App, pane_id: PaneId) -> crate::workspace::FloatingRect {
        app.state.workspaces[0].tabs[0]
            .floating
            .get(pane_id)
            .expect("floating pane")
            .rect
    }

    #[test]
    fn tab_floating_set_toggles_the_layer_and_focus() {
        let (mut app, root, floating) = app_with_floating_pane();
        let floating_public = app.public_pane_id(0, floating).unwrap();
        let root_public = app.public_pane_id(0, root).unwrap();

        let ResponseResult::TabFloating { floating: result } =
            success(&app.handle_tab_floating_set("hide".into(), TabFloatingParams::default()))
        else {
            panic!("expected tab floating result");
        };
        assert!(result.changed && !result.visible);
        assert_eq!(result.focused_pane_id, root_public);
        assert_eq!(app.state.workspaces[0].focused_pane_id(), Some(root));

        let ResponseResult::TabFloating { floating: result } =
            success(&app.handle_tab_floating_set(
                "show".into(),
                TabFloatingParams {
                    mode: FloatingLayerMode::Show,
                    ..TabFloatingParams::default()
                },
            ))
        else {
            panic!("expected tab floating result");
        };
        assert!(result.changed && result.visible);
        assert_eq!(result.focused_pane_id, floating_public);
        assert!(result.created_pane.is_none());
        assert_eq!(
            app.state
                .previous_pane_focus
                .as_ref()
                .map(|focus| focus.pane_id),
            Some(root)
        );
        app.state.workspaces[0].assert_invariants_for_test();
    }

    #[test]
    fn showing_an_empty_layer_without_create_is_a_no_op() {
        let (mut app, root, floating) = app_with_floating_pane();
        app.state.workspaces[0].tabs[0].floating.remove(floating);
        app.state.workspaces[0].tabs[0].panes.remove(&floating);
        app.state.workspaces[0]
            .public_pane_numbers
            .remove(&floating);

        let ResponseResult::TabFloating { floating: result } =
            success(&app.handle_tab_floating_set("show".into(), TabFloatingParams::default()))
        else {
            panic!("expected tab floating result");
        };
        assert!(!result.changed && !result.visible);
        assert_eq!(app.state.workspaces[0].focused_pane_id(), Some(root));
    }

    #[test]
    fn pin_requires_a_floating_pane_and_reports_the_new_state() {
        let (mut app, root, floating) = app_with_floating_pane();
        let root_public = app.public_pane_id(0, root).unwrap();
        assert_eq!(
            error_code(&app.handle_pane_pin(
                "tiled".into(),
                PanePinParams {
                    pane_id: Some(root_public),
                    mode: FloatingFlagMode::Toggle,
                },
            )),
            "pane_not_floating"
        );

        let ResponseResult::PaneInfo { pane } =
            success(&app.handle_pane_pin("pin".into(), PanePinParams::default()))
        else {
            panic!("expected pane info");
        };
        assert!(pane.floating.is_some_and(|floating| floating.pinned));
        assert!(app.state.workspaces[0].tabs[0]
            .floating
            .get(floating)
            .is_some_and(|pane| pane.pinned));
    }

    #[test]
    fn floating_set_embeds_and_refuses_to_float_the_last_tiled_pane() {
        let (mut app, root, floating) = app_with_floating_pane();
        let root_public = app.public_pane_id(0, root).unwrap();
        assert_eq!(
            error_code(&app.handle_pane_floating_set(
                "last".into(),
                PaneFloatingSetParams {
                    pane_id: Some(root_public),
                    mode: FloatingFlagMode::On,
                },
            )),
            "pane_floating_unavailable"
        );

        let ResponseResult::PaneInfo { pane } = success(
            &app.handle_pane_floating_set("embed".into(), PaneFloatingSetParams::default()),
        ) else {
            panic!("expected pane info");
        };
        assert!(pane.floating.is_none());
        assert!(pane.focused);
        assert_eq!(
            app.state.workspaces[0].tabs[0].layout.pane_ids(),
            vec![root, floating]
        );
        app.state.workspaces[0].assert_invariants_for_test();
    }

    #[test]
    fn move_resize_and_geometry_change_only_floating_geometry() {
        let (mut app, _root, floating) = app_with_floating_pane();
        let floating_public = app.public_pane_id(0, floating).unwrap();
        let area = app.state.new_pane_area();
        let before = floating_rect(&app, floating).resolve(area);

        success(&app.handle_pane_floating_move(
            "move".into(),
            PaneFloatingMoveParams {
                pane_id: None,
                direction: PaneDirection::Right,
                cells: Some(3),
            },
        ));
        let moved = floating_rect(&app, floating).resolve(area);
        assert_eq!(moved.x, before.x + 3);
        assert_eq!(moved.width, before.width);

        let ResponseResult::PaneResize { resize } = success(&app.handle_pane_resize(
            "resize".into(),
            PaneResizeParams {
                pane_id: Some(floating_public.clone()),
                direction: PaneDirection::Down,
                amount: Some(0.1),
            },
        )) else {
            panic!("expected resize result");
        };
        assert!(resize.changed);
        assert!(floating_rect(&app, floating).resolve(area).height > moved.height);

        success(&app.handle_pane_floating_geometry(
            "geometry".into(),
            PaneFloatingGeometryParams {
                pane_id: floating_public,
                geometry: FloatingGeometry {
                    x: 100,
                    y: 100,
                    width: 400,
                    height: 300,
                },
            },
        ));
        assert_eq!(
            floating_rect(&app, floating),
            crate::workspace::FloatingRect::new(100, 100, 400, 300)
        );
        app.state.workspaces[0].assert_invariants_for_test();
    }

    #[test]
    fn focus_direction_moves_between_floating_panes() {
        let (mut app, _root, left) = app_with_floating_pane();
        let right = app.state.workspaces[0].test_add_floating(true);
        let layer = &mut app.state.workspaces[0].tabs[0].floating;
        layer.set_rect(left, crate::workspace::FloatingRect::new(0, 200, 300, 300));
        layer.set_rect(
            right,
            crate::workspace::FloatingRect::new(600, 200, 300, 300),
        );
        let right_public = app.public_pane_id(0, right).unwrap();

        let response = app.handle_pane_focus_direction(
            "focus".into(),
            crate::api::schema::PaneFocusDirectionParams {
                pane_id: Some(right_public),
                direction: PaneDirection::Left,
            },
        );
        let ResponseResult::PaneFocusDirection { focus } = success(&response) else {
            panic!("expected focus result");
        };
        assert!(focus.changed);
        assert_eq!(app.state.workspaces[0].focused_pane_id(), Some(left));
    }
}
