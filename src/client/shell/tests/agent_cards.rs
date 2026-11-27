use super::*;

fn agent(pane_id: &str, name: &str, status: AgentStatus, focused: bool) -> ClientShellAgent {
    ClientShellAgent {
        pane_id: pane_id.into(),
        workspace_id: "ws_1".into(),
        tab_id: "tab_1".into(),
        name: Some(name.into()),
        display_agent: None,
        agent: Some(name.into()),
        title: None,
        terminal_title: None,
        terminal_title_stripped: None,
        agent_status: status,
        state_change_seq: 1,
        state_labels: Vec::new(),
        tokens: Vec::new(),
        focused,
    }
}

fn card_state(style: SidebarCardStyle) -> ClientShellState {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.config.agents.card_style = style;
    let mut projected = snapshot();
    projected.workspaces[0].label = "duo".into();
    projected.agents = vec![
        agent("pane_1", "kiro", AgentStatus::Working, true),
        agent("pane_2", "claude", AgentStatus::Idle, false),
    ];
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(surface());
    state
}

fn card_line(frame: &FrameData, rect: Rect, y: u16) -> String {
    frame_rows(frame)[usize::from(y)]
        .chars()
        .skip(usize::from(rect.x))
        .take(usize::from(rect.width))
        .collect()
}

fn cell(frame: &FrameData, x: u16, y: u16) -> &crate::protocol::CellData {
    &frame.cells[usize::from(y) * usize::from(frame.width) + usize::from(x)]
}

#[test]
fn compact_agent_card_puts_space_name_on_the_top_border() {
    let mut state = card_state(SidebarCardStyle::Compact);
    let frame = state.compose(106, 30).unwrap();
    let (rect, pane_id) = state.hits.agents[0].clone();
    assert_eq!(pane_id, "pane_1");
    assert_eq!(rect.height, 3);
    assert_eq!(cell(&frame, rect.x, rect.y).symbol, "╭");
    assert_eq!(
        cell(&frame, rect.right() - 1, rect.bottom() - 1).symbol,
        "╯"
    );
    let border_line = card_line(&frame, rect, rect.y);
    let line = card_line(&frame, rect, rect.y + 1);
    // space 名带左边距压在上边框上，不顶着圆角。
    assert!(border_line.starts_with("╭── duo "), "{border_line:?}");
    assert!(line.contains("◐ kiro"), "{line:?}");
    assert!(!line.contains("duo"), "{line:?}");
    assert!(
        line.trim_end_matches(['│', ' ']).ends_with("running"),
        "{line:?}"
    );
    assert!(state.hits.agents[1].0.y >= rect.bottom());
}

#[test]
fn compact_agent_card_keeps_tab_context_after_the_name() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.config.agents.card_style = SidebarCardStyle::Compact;
    let mut projected = snapshot();
    projected.workspaces[0].label = "duo".into();
    projected.tabs[0].label = "ci".into();
    projected.tabs[0].custom_label = true;
    projected.agents = vec![agent("pane_1", "kiro", AgentStatus::Working, true)];
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(surface());
    let frame = state.compose(106, 30).unwrap();
    let rect = state.hits.agents[0].0;
    let border_line = card_line(&frame, rect, rect.y);
    let line = card_line(&frame, rect, rect.y + 1);
    assert!(border_line.starts_with("╭── duo "), "{border_line:?}");
    assert!(line.contains("◐ kiro · ci"), "{line:?}");
    assert!(!line.contains("duo"), "{line:?}");
}

fn named_pane_state(style: SidebarCardStyle, pane_label: &str) -> ClientShellState {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.config.agents.card_style = style;
    state.sidebar_width = 40;
    state.sidebar_width_manual = true;
    let mut projected = snapshot();
    projected.workspaces[0].label = "duo".into();
    projected.tabs[0].label = "ci".into();
    projected.tabs[0].custom_label = true;
    projected.panes[0].label = Some(pane_label.into());
    projected.agents = vec![agent("pane_1", "kiro", AgentStatus::Working, true)];
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(surface());
    state
}

#[test]
fn compact_agent_card_shows_user_named_pane_after_tab() {
    let mut state = named_pane_state(SidebarCardStyle::Compact, "reviewer");
    let frame = state.compose(106, 30).unwrap();
    let rect = state.hits.agents[0].0;
    let line = card_line(&frame, rect, rect.y + 1);
    assert!(line.contains("◐ kiro · ci · reviewer"), "{line:?}");
}

#[test]
fn full_agent_card_shows_user_named_pane_in_location() {
    let mut state = named_pane_state(SidebarCardStyle::Full, "reviewer");
    let frame = state.compose(106, 30).unwrap();
    let rect = state.hits.agents[0].0;
    let location_line = card_line(&frame, rect, rect.y + 2);
    assert!(
        location_line.contains("duo · ci · reviewer"),
        "{location_line:?}"
    );
}

#[test]
fn agent_card_omits_pane_name_that_repeats_the_card_name() {
    let mut state = named_pane_state(SidebarCardStyle::Compact, "kiro");
    let frame = state.compose(106, 30).unwrap();
    let rect = state.hits.agents[0].0;
    let line = card_line(&frame, rect, rect.y + 1);
    assert!(line.contains("◐ kiro · ci "), "{line:?}");
    assert!(!line.contains("ci · kiro"), "{line:?}");
}

#[test]
fn full_agent_card_puts_location_on_its_own_row() {
    let mut state = card_state(SidebarCardStyle::Full);
    let frame = state.compose(106, 30).unwrap();
    let rect = state.hits.agents[0].0;
    assert_eq!(rect.height, 4);
    let name_line = card_line(&frame, rect, rect.y + 1);
    let location_line = card_line(&frame, rect, rect.y + 2);
    assert!(name_line.contains("◐ kiro"), "{name_line:?}");
    assert!(name_line.contains("running"), "{name_line:?}");
    assert!(!name_line.contains("duo"), "{name_line:?}");
    assert!(location_line.contains("duo"), "{location_line:?}");
}

#[test]
fn agent_card_highlights_focus_with_border_instead_of_fill() {
    let mut state = card_state(SidebarCardStyle::Compact);
    let frame = state.compose(106, 30).unwrap();
    let palette = &state.config.palette;
    let focused = state.hits.agents[0].0;
    let other = state.hits.agents[1].0;
    assert_eq!(
        cell(&frame, focused.x, focused.y).fg,
        crate::protocol::color_to_u32(palette.accent)
    );
    assert_eq!(
        cell(&frame, other.x, other.y).fg,
        crate::protocol::color_to_u32(palette.overlay0)
    );
    let active = crate::protocol::color_to_u32(palette.active_row_bg);
    assert!((focused.x..focused.right()).all(|x| cell(&frame, x, focused.y + 1).bg != active));
}

#[test]
fn agent_card_status_cells_still_animate() {
    let mut state = card_state(SidebarCardStyle::Full);
    state.compose(106, 30).unwrap();
    let rect = state.hits.agents[0].0;
    let indicators = state
        .hits
        .agent_indicators
        .iter()
        .filter(|indicator| contains(rect, (indicator.rect.x, indicator.rect.y)))
        .collect::<Vec<_>>();
    assert!(indicators
        .iter()
        .any(|indicator| indicator.spinner && indicator.rect.x == rect.x + 2));
    assert!(indicators.iter().any(|indicator| !indicator.spinner));
    assert!(state.agent_animation.deadline().is_some());
}

#[test]
fn agent_card_hover_recolors_unfocused_border() {
    let mut state = card_state(SidebarCardStyle::Compact);
    state.compose(106, 30).unwrap();
    let other = state.hits.agents[1].0;
    let mouse = MouseEvent {
        kind: MouseEventKind::Moved,
        column: other.x + 1,
        row: other.y + 1,
        modifiers: KeyModifiers::NONE,
    };
    assert!(
        state
            .handle_raw_events(vec![RawInputEvent::Mouse(mouse)])
            .repaint
    );
    let hovered = state.compose(106, 30).unwrap();
    assert_eq!(
        cell(&hovered, other.x, other.y).fg,
        crate::protocol::color_to_u32(state.config.palette.hover_border_color())
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
    let restored = state.compose(106, 30).unwrap();
    assert_eq!(
        cell(&restored, other.x, other.y).fg,
        crate::protocol::color_to_u32(state.config.palette.overlay0)
    );
}

#[test]
fn plain_agent_list_keeps_token_rows_and_does_not_hover() {
    let mut state = card_state(SidebarCardStyle::Plain);
    state.compose(106, 30).unwrap();
    let rect = state.hits.agents[0].0;
    assert_eq!(rect.height, 2);
    state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Moved,
        column: rect.x + 1,
        row: rect.y,
        modifiers: KeyModifiers::NONE,
    })]);
    assert!(state.hovered_card.is_none());
}
