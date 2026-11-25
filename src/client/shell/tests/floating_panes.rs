use super::*;

const FLOATING: SurfaceRect = SurfaceRect {
    x: 20,
    y: 3,
    width: 40,
    height: 10,
};

fn surface_pane(pane_id: &str, rect: SurfaceRect, focused: bool) -> PaneSurfacePane {
    PaneSurfacePane {
        pane_id: pane_id.into(),
        content_revision: 0,
        rect,
        inner_rect: SurfaceRect {
            x: rect.x + 1,
            y: rect.y + 1,
            width: rect.width - 2,
            height: rect.height - 2,
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

/// A tiled split with a floating pane above both halves, listed topmost first.
fn floating_state() -> ClientShellState {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    let mut projected = snapshot();
    let mut floating = projected.panes[0].clone();
    floating.pane_id = "pane_2".into();
    floating.focused = true;
    floating.floating = Some(crate::protocol::ClientShellFloatingPane { pinned: false });
    projected.panes[0].focused = false;
    projected.panes.push(floating);
    projected.focused_pane_id = Some("pane_2".into());
    state.set_snapshot(Box::new(projected));

    let mut pane_surface = surface();
    let full = SurfaceRect {
        x: 0,
        y: 0,
        width: 80,
        height: 19,
    };
    pane_surface.panes = vec![
        surface_pane("pane_2", FLOATING, true),
        surface_pane("pane_1", full, false),
    ];
    pane_surface.splits.push(PaneSurfaceSplit {
        direction: PaneSurfaceSplitDirection::Horizontal,
        pos: 40,
        area: full,
        hit_rect: SurfaceRect {
            x: 40,
            y: 0,
            width: 1,
            height: 19,
        },
        path: vec![false],
    });
    state.set_pane_surface(pane_surface);
    state.compose(106, 20).expect("floating pane surface");
    state
}

fn left_mouse(kind: MouseEventKind, column: u16, row: u16) -> RawInputEvent {
    RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::empty(),
    })
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

#[test]
fn floating_hits_lead_the_hit_list_and_cover_lower_panes() {
    let state = floating_state();
    let floating = state.hits.panes[0].clone();
    assert_eq!(floating.pane_id, "pane_2");
    assert!(floating.floating.is_some());
    assert!(state.hits.panes[1].floating.is_none());

    let inside = (floating.inner_rect.x + 1, floating.inner_rect.y + 1);
    assert_eq!(
        state.pane_hit_at(inside).map(|hit| hit.pane_id.as_str()),
        Some("pane_2")
    );
    assert!(state.pane_covered_at("pane_1", inside));
    assert!(!state.pane_covered_at("pane_1", (state.hits.pane_surface.x, 0)));
}

#[test]
fn pressing_a_split_under_a_floating_pane_focuses_the_floating_pane() {
    let mut state = floating_state();
    let split = state.hits.pane_splits[0].clone();
    let floating = state.hits.panes[0].clone();
    let row = floating.inner_rect.y + 1;
    assert!(super::super::contains(split.hit_rect, (split.pos, row)));

    let outcome = state.handle_raw_events(vec![left_mouse(
        MouseEventKind::Down(MouseButton::Left),
        split.pos,
        row,
    )]);
    assert!(!matches!(
        state.chrome_drag,
        Some(ClientChromeDrag::PaneSplit { .. })
    ));
    assert!(methods(&outcome).iter().any(|method| matches!(
        method,
        crate::api::schema::Method::PaneFocus(target) if target.pane_id == "pane_2"
    )));
}

#[test]
fn dragging_the_title_border_moves_the_floating_pane() {
    let mut state = floating_state();
    let floating = state.hits.panes[0].clone();
    let surface = state.hits.pane_surface;
    let start = (floating.rect.x + 4, floating.rect.y);

    let press = state.handle_raw_events(vec![left_mouse(
        MouseEventKind::Down(MouseButton::Left),
        start.0,
        start.1,
    )]);
    assert!(matches!(
        methods(&press)[..],
        [crate::api::schema::Method::PaneFocus(_)]
    ));
    assert!(matches!(
        state.chrome_drag,
        Some(ClientChromeDrag::FloatingPane(_))
    ));

    let drag = state.handle_raw_events(vec![left_mouse(
        MouseEventKind::Drag(MouseButton::Left),
        start.0 + 6,
        start.1 + 2,
    )]);
    let expected = crate::workspace::FloatingRect::from_cells(
        Rect::new(
            floating.rect.x + 6,
            floating.rect.y + 2,
            floating.rect.width,
            floating.rect.height,
        ),
        surface,
    );
    let [crate::api::schema::Method::PaneFloatingGeometry(params)] = methods(&drag)[..] else {
        panic!("drag should send floating geometry");
    };
    assert_eq!(params.pane_id, "pane_2");
    assert_eq!(
        (params.geometry.x, params.geometry.y),
        (expected.x, expected.y)
    );

    let release = state.handle_raw_events(vec![left_mouse(
        MouseEventKind::Up(MouseButton::Left),
        start.0 + 6,
        start.1 + 2,
    )]);
    assert!(
        methods(&release).is_empty(),
        "unchanged release resends nothing"
    );
    assert!(state.chrome_drag.is_none());
}

#[test]
fn releasing_a_resize_drag_sends_the_final_geometry() {
    let mut state = floating_state();
    let floating = state.hits.panes[0].clone();
    let corner = (floating.rect.right() - 1, floating.rect.bottom() - 1);
    state.handle_raw_events(vec![left_mouse(
        MouseEventKind::Down(MouseButton::Left),
        corner.0,
        corner.1,
    )]);
    let release = state.handle_raw_events(vec![left_mouse(
        MouseEventKind::Up(MouseButton::Left),
        corner.0 + 4,
        corner.1 + 2,
    )]);
    let [crate::api::schema::Method::PaneFloatingGeometry(params)] = methods(&release)[..] else {
        panic!("release should send the final floating geometry");
    };
    let resized = Rect::new(
        floating.rect.x,
        floating.rect.y,
        floating.rect.width + 4,
        floating.rect.height + 2,
    );
    let expected = crate::workspace::FloatingRect::from_cells(resized, state.hits.pane_surface);
    assert_eq!(
        (params.geometry.width, params.geometry.height),
        (expected.width, expected.height)
    );
}

#[test]
fn clicking_the_pin_button_toggles_the_pin() {
    let mut state = floating_state();
    let floating = state.hits.panes[0].clone();
    let pin = crate::workspace::floating::pin_button_rect(floating.rect, false).unwrap();
    let outcome = state.handle_raw_events(vec![left_mouse(
        MouseEventKind::Down(MouseButton::Left),
        pin.x + 1,
        pin.y,
    )]);
    assert!(methods(&outcome).iter().any(|method| matches!(
        method,
        crate::api::schema::Method::PanePin(params)
            if params.pane_id.as_deref() == Some("pane_2")
                && params.mode == crate::api::schema::FloatingFlagMode::Toggle
    )));
    assert!(state.chrome_drag.is_none());
}

#[test]
fn floating_keybindings_use_the_floating_endpoint_methods() {
    let mut state = floating_state();
    let method = |state: &mut ClientShellState, action| {
        state
            .endpoint_method_for_action(action)
            .expect("floating action maps to a method")
    };
    assert!(matches!(
        method(&mut state, crate::input::KeybindAction::ToggleFloating),
        crate::api::schema::Method::TabFloatingSet(params)
            if params.tab_id.as_deref() == Some("tab_1") && params.create_if_empty
    ));
    assert!(matches!(
        method(&mut state, crate::input::KeybindAction::NewFloatingPane),
        crate::api::schema::Method::PaneFloatingCreate(params) if params.focus
    ));
    assert!(matches!(
        method(&mut state, crate::input::KeybindAction::ToggleFloatingPin),
        crate::api::schema::Method::PanePin(params)
            if params.pane_id.as_deref() == Some("pane_2")
    ));
    assert!(matches!(
        method(&mut state, crate::input::KeybindAction::MoveFloatingPaneLeft),
        crate::api::schema::Method::PaneFloatingMove(params)
            if params.direction == crate::api::schema::PaneDirection::Left
    ));

    let mut tiled = state.snapshot.as_deref().cloned().unwrap();
    tiled.focused_pane_id = Some("pane_1".into());
    state.set_snapshot(Box::new(tiled));
    assert!(state
        .endpoint_method_for_action(crate::input::KeybindAction::MoveFloatingPaneLeft)
        .is_none());
}

#[test]
fn pane_menu_offers_float_embed_and_pin_by_pane_kind() {
    let mut state = floating_state();
    let labels = |state: &ClientShellState| match state.overlay.as_ref() {
        Some(ClientShellOverlay::ContextMenu(menu)) => menu
            .items()
            .into_iter()
            .map(|item| item.label.into_owned())
            .collect::<Vec<_>>(),
        _ => panic!("pane menu should be open"),
    };

    state.open_pane_context_menu("pane_2".into(), 30, 5);
    let floating = labels(&state);
    assert!(floating.contains(&"Embed pane".to_owned()));
    assert!(floating.contains(&"Pin floating pane".to_owned()));
    assert!(floating.contains(&"New floating pane".to_owned()));

    state.open_pane_context_menu("pane_1".into(), 2, 2);
    let only_tiled = labels(&state);
    assert!(
        !only_tiled.contains(&"Make pane floating".to_owned()),
        "the only tiled pane of a tab stays tiled"
    );
    assert!(only_tiled.contains(&"New floating pane".to_owned()));
    assert!(!only_tiled.iter().any(|label| label.contains("Pin")));

    let mut projected = state.snapshot.as_deref().cloned().unwrap();
    let mut second = projected.panes[0].clone();
    second.pane_id = "pane_3".into();
    projected.panes.push(second);
    state.set_snapshot(Box::new(projected));
    state.open_pane_context_menu("pane_1".into(), 2, 2);
    assert!(labels(&state).contains(&"Make pane floating".to_owned()));
}

#[test]
fn pane_menu_new_floating_pane_opens_one_in_the_clicked_tab() {
    let mut state = floating_state();
    state.open_pane_context_menu("pane_1".into(), 2, 2);
    let index = match state.overlay.as_ref() {
        Some(ClientShellOverlay::ContextMenu(menu)) => menu
            .items()
            .iter()
            .position(|item| item.label == "New floating pane")
            .expect("menu item"),
        _ => panic!("pane menu should be open"),
    };
    let mut outcome = ClientShellInput::default();
    state.activate_context_menu_item(index, &mut outcome);
    assert!(methods(&outcome).iter().any(|method| matches!(
        method,
        crate::api::schema::Method::PaneFloatingCreate(params)
            if params.tab_id.as_deref() == Some("tab_1") && params.focus
    )));
}
