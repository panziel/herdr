use super::*;

const PANE_WIDTH: u16 = 40;
const PANE_HEIGHT: u16 = 6;

fn bordered_lines(title: &str) -> Vec<String> {
    let inner = usize::from(PANE_WIDTH) - 2;
    let mut top = format!("┌─{title}");
    let fill = inner.saturating_sub(title.chars().count() + 1);
    top.push_str(&"─".repeat(fill));
    top.push('┐');
    let mut lines = vec![top];
    for _ in 1..PANE_HEIGHT - 1 {
        lines.push(format!("│{}│", " ".repeat(inner)));
    }
    lines.push(format!("└{}┘", "─".repeat(inner)));
    lines
}

fn bordered_pane(pane_id: &str, x: u16, focused: bool) -> PaneSurfacePane {
    PaneSurfacePane {
        pane_id: pane_id.into(),
        content_revision: 0,
        rect: SurfaceRect {
            x,
            y: 0,
            width: PANE_WIDTH,
            height: PANE_HEIGHT,
        },
        inner_rect: SurfaceRect {
            x: x + 1,
            y: 1,
            width: PANE_WIDTH - 2,
            height: PANE_HEIGHT - 2,
        },
        scrollbar_rect: None,
        scroll: None,
        focused,
        mouse_reporting: false,
        sgr_pixel_mouse: false,
        alternate_screen_active: false,
        pixel_width: 0,
        pixel_height: 0,
    }
}

/// Two bordered panes side by side in a surface frame.
fn bordered_surface(left_title: &str) -> PaneSurfaceFrame {
    let left = bordered_lines(left_title);
    let right = bordered_lines(" shell ");
    let lines = left
        .iter()
        .zip(&right)
        .map(|(left, right)| format!("{left}{right}"))
        .collect::<Vec<_>>();
    let buffer = Buffer::with_lines(lines.iter().map(String::as_str));
    let mut pane_surface = surface();
    pane_surface.frame = FrameData::from_ratatui_buffer_with_hyperlinks(&buffer, None, &[]);
    pane_surface.panes = vec![
        bordered_pane("pane_1", 0, true),
        bordered_pane("pane_2", PANE_WIDTH, false),
    ];
    pane_surface
}

fn pane(pane_id: &str, tab_id: &str, focused: bool) -> ClientShellPane {
    ClientShellPane {
        pane_id: pane_id.into(),
        workspace_id: "ws_1".into(),
        tab_id: tab_id.into(),
        label: None,
        cwd: Some("/repo".into()),
        foreground_cwd: Some("/repo".into()),
        focused,
        right_click_passthrough: false,
        floating: None,
    }
}

/// tab_1 holds pane_1 and pane_2; tab_2 ("build") holds pane_3.
fn two_tab_snapshot() -> ClientShellSnapshot {
    let mut projected = snapshot();
    let mut build = projected.tabs[0].clone();
    build.tab_id = "tab_2".into();
    build.number = 2;
    build.label = "build".into();
    build.custom_label = true;
    build.focused = false;
    projected.tabs.push(build);
    projected.panes = vec![
        pane("pane_1", "tab_1", true),
        pane("pane_2", "tab_1", false),
        pane("pane_3", "tab_2", false),
    ];
    projected
}

fn state_with(projected: ClientShellSnapshot, left_title: &str) -> ClientShellState {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(bordered_surface(left_title));
    state.compose(106, 20).expect("bordered pane surface");
    state
}

fn mouse(kind: MouseEventKind, column: u16, row: u16) -> RawInputEvent {
    RawInputEvent::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::empty(),
    })
}

fn click(state: &mut ClientShellState, rect: Rect) -> ClientShellInput {
    state.handle_raw_events(vec![mouse(
        MouseEventKind::Down(MouseButton::Left),
        rect.x,
        rect.y,
    )])
}

fn methods(outcome: &ClientShellInput) -> Vec<&crate::api::schema::Method> {
    outcome
        .actions
        .iter()
        .filter_map(|action| match action {
            ClientShellAction::Endpoint { request, .. } => Some(&request.method),
            _ => None,
        })
        .collect()
}

fn top_border(frame: &FrameData, rect: Rect) -> String {
    let row = usize::from(rect.y) * usize::from(frame.width);
    (rect.x..rect.right())
        .map(|x| frame.cells[row + usize::from(x)].symbol.as_str())
        .collect()
}

fn button(
    state: &ClientShellState,
    pane_id: &str,
    action: super::super::pane_buttons::PaneButtonAction,
) -> Rect {
    state
        .hits
        .pane_buttons
        .iter()
        .find(|button| button.pane_id == pane_id && button.action == action)
        .map(|button| button.rect)
        .unwrap_or_else(|| panic!("{action:?} button on {pane_id}"))
}

fn assert_moves_pane_1_to_build(method: &crate::api::schema::Method) {
    let crate::api::schema::Method::PaneMove(params) = method else {
        panic!("expected pane.move, got {method:?}");
    };
    assert_eq!(params.pane_id, "pane_1");
    assert!(params.focus);
    assert_eq!(
        params.destination,
        crate::api::schema::PaneMoveDestination::Tab {
            tab_id: "tab_2".into(),
            target_pane_id: None,
            split: crate::api::schema::SplitDirection::Right,
            ratio: None,
        }
    );
}

fn context_menu_labels(state: &ClientShellState) -> Vec<String> {
    match state.overlay.as_ref() {
        Some(ClientShellOverlay::ContextMenu(menu)) => menu
            .items()
            .into_iter()
            .map(|item| item.label.into_owned())
            .collect(),
        _ => panic!("context menu should be open"),
    }
}

#[test]
fn border_buttons_sit_before_the_top_right_corner() {
    let mut state = state_with(two_tab_snapshot(), " shell ");
    let frame = state.compose(106, 20).expect("frame");
    let rect = state.hits.panes[0].rect;
    let border = top_border(&frame, rect);
    assert!(border.starts_with("┌─ shell ─"), "{border}");
    assert!(border.ends_with("─[->]─[+]─[x]─┐"), "{border}");
    assert_eq!(
        state
            .hits
            .pane_buttons
            .iter()
            .filter(|button| button.pane_id == "pane_1")
            .count(),
        3
    );
}

#[test]
fn border_buttons_only_offer_actions_that_apply() {
    let mut projected = two_tab_snapshot();
    projected.tabs.truncate(1);
    projected.panes.truncate(1);
    let mut state = state_with(projected, " shell ");
    let frame = state.compose(106, 20).expect("frame");
    let rect = state.hits.panes[0].rect;
    let border = top_border(&frame, rect);
    assert!(border.ends_with("─[x]─┐"), "{border}");
    assert!(!border.contains("[->]") && !border.contains("[+]"));
}

#[test]
fn border_buttons_never_overwrite_a_long_title() {
    let title = format!(" {} ", "t".repeat(usize::from(PANE_WIDTH) - 8));
    let mut state = state_with(two_tab_snapshot(), &title);
    let frame = state.compose(106, 20).expect("frame");
    let rect = state.hits.panes[0].rect;
    assert!(top_border(&frame, rect).contains(&title));
    assert!(!state
        .hits
        .pane_buttons
        .iter()
        .any(|button| button.pane_id == "pane_1"));
}

#[test]
fn close_and_zoom_buttons_send_their_pane_methods() {
    use super::super::pane_buttons::PaneButtonAction;

    let mut state = state_with(two_tab_snapshot(), " shell ");
    let target = button(&state, "pane_1", PaneButtonAction::Close);
    let close = click(&mut state, target);
    assert!(matches!(
        methods(&close)[..],
        [crate::api::schema::Method::PaneClose(target)] if target.pane_id == "pane_1"
    ));

    state.compose(106, 20).expect("frame");
    let target = button(&state, "pane_1", PaneButtonAction::ToggleZoom);
    let zoom = click(&mut state, target);
    assert!(matches!(
        methods(&zoom)[..],
        [crate::api::schema::Method::PaneZoom(params)]
            if params.pane_id.as_deref() == Some("pane_1")
                && params.mode == crate::api::schema::PaneZoomMode::Toggle
    ));
    assert!(
        state.chrome_drag.is_none(),
        "buttons do not start a pane drag"
    );
}

#[test]
fn move_button_lists_other_tabs_and_moves_the_pane() {
    use super::super::pane_buttons::PaneButtonAction;

    let mut state = state_with(two_tab_snapshot(), " shell ");
    let target = button(&state, "pane_1", PaneButtonAction::MoveToTab);
    let open = click(&mut state, target);
    assert!(methods(&open).is_empty());
    assert_eq!(context_menu_labels(&state), vec!["build".to_owned()]);

    state.compose(106, 20).expect("move menu");
    let choice = state.hits.context_menu_rows[0].0;
    let moved = click(&mut state, choice);
    let [method] = methods(&moved)[..] else {
        panic!("one pane.move request");
    };
    assert_moves_pane_1_to_build(method);
    assert!(state.overlay.is_none());
}

#[test]
fn pane_menu_moves_the_pane_to_a_chosen_tab() {
    let mut state = state_with(two_tab_snapshot(), " shell ");
    state.open_pane_context_menu("pane_1".into(), 2, 2);
    let index = context_menu_labels(&state)
        .iter()
        .position(|label| label == "Move to tab...")
        .expect("move to tab item");
    let mut outcome = ClientShellInput::default();
    state.activate_context_menu_item(index, &mut outcome);
    assert!(methods(&outcome).is_empty());
    assert_eq!(context_menu_labels(&state), vec!["build".to_owned()]);

    let mut outcome = ClientShellInput::default();
    state.activate_context_menu_item(0, &mut outcome);
    let [method] = methods(&outcome)[..] else {
        panic!("one pane.move request");
    };
    assert_moves_pane_1_to_build(method);
}

#[test]
fn pane_menu_hides_move_when_the_workspace_has_one_tab() {
    let mut projected = two_tab_snapshot();
    projected.tabs.truncate(1);
    projected.panes.truncate(2);
    let mut state = state_with(projected, " shell ");
    state.open_pane_context_menu("pane_1".into(), 2, 2);
    assert!(!context_menu_labels(&state)
        .iter()
        .any(|label| label == "Move to tab..."));
}

#[test]
fn moving_out_of_a_zoomed_tab_unzooms_it_first() {
    let mut projected = two_tab_snapshot();
    projected.tabs[0].zoomed = true;
    projected.tabs[1].zoomed = true;
    let mut state = state_with(projected, " shell ");
    let frame = state.compose(106, 20).expect("frame");
    assert!(top_border(&frame, state.hits.panes[0].rect).contains("[-]"));

    let mut outcome = ClientShellInput::default();
    state.request_pane_move_to_tab("pane_1".into(), "tab_2".into(), &mut outcome);
    let [crate::api::schema::Method::PaneZoom(source), crate::api::schema::Method::PaneZoom(target), moved] =
        methods(&outcome)[..]
    else {
        panic!("unzoom both tabs, then move");
    };
    assert_eq!(source.pane_id.as_deref(), Some("pane_1"));
    assert_eq!(target.pane_id.as_deref(), Some("pane_3"));
    assert_eq!(source.mode, crate::api::schema::PaneZoomMode::Off);
    assert_eq!(target.mode, crate::api::schema::PaneZoomMode::Off);
    assert_moves_pane_1_to_build(moved);
}

#[test]
fn dragging_a_pane_border_onto_another_tab_moves_the_pane() {
    let mut state = state_with(two_tab_snapshot(), " shell ");
    let border = state.hits.panes[0].rect;
    let press = state.handle_raw_events(vec![mouse(
        MouseEventKind::Down(MouseButton::Left),
        border.x + 1,
        border.y,
    )]);
    assert!(matches!(
        methods(&press)[..],
        [crate::api::schema::Method::PaneFocus(_)]
    ));

    let (own_tab, build_tab) = {
        let tab = |id: &str| {
            state
                .hits
                .tabs
                .iter()
                .find(|(_, tab_id)| tab_id == id)
                .map(|(rect, _)| *rect)
                .expect("tab hit")
        };
        (tab("tab_1"), tab("tab_2"))
    };
    state.handle_raw_events(vec![mouse(
        MouseEventKind::Drag(MouseButton::Left),
        own_tab.x + 1,
        own_tab.y,
    )]);
    assert!(matches!(
        state.chrome_drag,
        Some(ClientChromeDrag::PaneMove {
            target_tab_id: None,
            ..
        })
    ));

    state.handle_raw_events(vec![mouse(
        MouseEventKind::Drag(MouseButton::Left),
        build_tab.x + 1,
        build_tab.y,
    )]);
    assert!(matches!(
        state.chrome_drag,
        Some(ClientChromeDrag::PaneMove { target_tab_id: Some(ref tab_id), target_pane_id: None, .. })
            if tab_id == "tab_2"
    ));
    let frame = state.compose(106, 20).expect("drag frame");
    let highlighted = &frame.cells
        [usize::from(build_tab.y) * usize::from(frame.width) + usize::from(build_tab.x + 1)];
    assert_ne!(
        highlighted.modifier & Modifier::UNDERLINED.bits(),
        0,
        "the drop target tab is underlined"
    );

    let release = state.handle_raw_events(vec![mouse(
        MouseEventKind::Up(MouseButton::Left),
        build_tab.x + 1,
        build_tab.y,
    )]);
    let [method] = methods(&release)[..] else {
        panic!("one pane.move request");
    };
    assert_moves_pane_1_to_build(method);
    assert!(state.chrome_drag.is_none());
}
