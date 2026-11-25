use super::*;

fn state_with_focused_pane(width: u16, height: u16) -> ClientShellState {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(snapshot()));
    let mut pane_surface = surface();
    let pane = &mut pane_surface.panes[0];
    pane.rect = SurfaceRect {
        x: 0,
        y: 0,
        width,
        height,
    };
    pane.inner_rect = SurfaceRect {
        x: 1,
        y: 1,
        width: width - 2,
        height: height - 2,
    };
    state.set_pane_surface(pane_surface);
    state.compose(106, 40).expect("pane surface");
    state
}

fn new_pane_split(state: &mut ClientShellState) -> crate::api::schema::PaneSplitParams {
    match state.endpoint_method_for_action(crate::input::KeybindAction::NewPane) {
        Some(crate::api::schema::Method::PaneSplit(params)) => params,
        other => panic!("new pane should split the focused pane, got {other:?}"),
    }
}

#[test]
fn new_pane_splits_a_wide_focused_pane_side_by_side() {
    let mut state = state_with_focused_pane(80, 30);
    let params = new_pane_split(&mut state);
    assert_eq!(params.target_pane_id.as_deref(), Some("pane_1"));
    assert_eq!(params.direction, crate::api::schema::SplitDirection::Right);
    assert!(params.focus);
}

#[test]
fn new_pane_stacks_below_a_tall_focused_pane() {
    let mut state = state_with_focused_pane(40, 30);
    let params = new_pane_split(&mut state);
    assert_eq!(params.target_pane_id.as_deref(), Some("pane_1"));
    assert_eq!(params.direction, crate::api::schema::SplitDirection::Down);
}
