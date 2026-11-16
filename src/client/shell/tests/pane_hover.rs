use super::*;

fn hover_state(focused: bool) -> ClientShellState {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    let mut projected = snapshot();
    projected.focused_pane_id = focused.then(|| "pane_1".into());
    let mut pane_surface = surface();
    let mut buffer = Buffer::with_lines(["╭──────────╮", "│ ─color   │", "╰──────────╯"]);
    buffer.set_style(
        buffer.area,
        Style::default().fg(if focused {
            state.config.palette.accent
        } else {
            state.config.palette.overlay0
        }),
    );
    pane_surface.frame = FrameData::from_ratatui_buffer_with_hyperlinks(&buffer, None, &[]);
    pane_surface.panes[0].rect = SurfaceRect {
        x: 0,
        y: 0,
        width: 12,
        height: 3,
    };
    pane_surface.panes[0].inner_rect = SurfaceRect {
        x: 1,
        y: 1,
        width: 10,
        height: 1,
    };
    pane_surface.panes[0].focused = focused;
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(pane_surface);
    state.compose(106, 20).unwrap();
    state
}

#[test]
fn hover_changes_only_unfocused_border_and_repaints_only_on_transition() {
    let mut state = hover_state(false);
    let before = state.compose(106, 20).unwrap();
    let hit = state.hits.panes[0].clone();
    let mouse = MouseEvent {
        kind: MouseEventKind::Moved,
        column: hit.inner_rect.x + 1,
        row: hit.inner_rect.y,
        modifiers: KeyModifiers::NONE,
    };
    let outcome = state.handle_raw_events(vec![RawInputEvent::Mouse(mouse)]);
    assert!(outcome.repaint);
    assert!(outcome.actions.is_empty());
    assert!(outcome.requests.is_empty());
    let hovered = state.compose(106, 20).unwrap();
    let index = |x: u16, y: u16| usize::from(y) * usize::from(hovered.width) + usize::from(x);
    assert_eq!(
        hovered.cells[index(hit.rect.x, hit.rect.y)].fg,
        crate::protocol::color_to_u32(state.config.palette.hover_border_color())
    );
    assert_eq!(
        hovered.cells[index(hit.inner_rect.x + 1, hit.inner_rect.y)],
        before.cells[index(hit.inner_rect.x + 1, hit.inner_rect.y)]
    );
    assert_eq!(state.snapshot.as_ref().unwrap().focused_pane_id, None);
    for (before, after) in before.cells.iter().zip(&hovered.cells) {
        let mut expected = before.clone();
        expected.fg = after.fg;
        assert_eq!(&expected, after);
    }

    assert!(
        !state
            .handle_raw_events(vec![RawInputEvent::Mouse(mouse)])
            .repaint
    );
    let leave = MouseEvent {
        column: 0,
        row: 0,
        ..mouse
    };
    assert!(
        state
            .handle_raw_events(vec![RawInputEvent::Mouse(leave)])
            .repaint
    );
    let restored = state.compose(106, 20).unwrap();
    assert_eq!(restored.cells, before.cells);
}

#[test]
fn focused_pane_does_not_receive_hover_style() {
    let mut state = hover_state(true);
    let before = state.compose(106, 20).unwrap();
    let hit = state.hits.panes[0].clone();
    let outcome = state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Moved,
        column: hit.inner_rect.x,
        row: hit.inner_rect.y,
        modifiers: KeyModifiers::NONE,
    })]);
    assert!(state.hovered_pane_id.is_none());
    assert!(!outcome.repaint);
    let after = state.compose(106, 20).unwrap();
    assert_eq!(before.cells, after.cells);
    assert_eq!(
        after.cells[usize::from(hit.rect.y) * usize::from(after.width) + usize::from(hit.rect.x)]
            .fg,
        crate::protocol::color_to_u32(state.config.palette.accent)
    );
}
