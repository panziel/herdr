use super::*;
use std::time::{Duration, Instant};

fn animated_state(status: AgentStatus) -> ClientShellState {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    let mut projected = snapshot();
    projected.agents.push(ClientShellAgent {
        pane_id: "pane_1".into(),
        workspace_id: "ws_1".into(),
        tab_id: "tab_1".into(),
        name: Some("claude".into()),
        display_agent: None,
        agent: Some("claude".into()),
        title: None,
        terminal_title: None,
        terminal_title_stripped: None,
        agent_status: status,
        state_change_seq: 1,
        state_labels: Vec::new(),
        tokens: Vec::new(),
        focused: true,
    });
    if status == AgentStatus::Done {
        projected.agents[0].pane_id = "pane_background".into();
        projected.agents[0].focused = false;
        let mut running = projected.clone();
        running.agents[0].agent_status = AgentStatus::Working;
        state.set_snapshot(Box::new(running));
        projected.revision += 1;
        projected.agents[0].state_change_seq += 1;
    }
    let mut pane_surface = surface();
    pane_surface.projection_revision = projected.revision;
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(pane_surface);
    state.compose(106, 30).unwrap();
    state
}

#[test]
fn working_animation_patches_only_visible_status_cells_and_keeps_cursor() {
    let mut state = animated_state(AgentStatus::Working);
    let before = state.compose(106, 30).unwrap();
    let rect = state.hits.agents[0].0;
    let deadline = state.agent_animation.deadline().unwrap();
    assert!(state
        .tick_agent_animation(deadline - Duration::from_millis(1))
        .is_empty());
    let rows = state.tick_agent_animation(deadline);
    assert_eq!(rows.len(), 1);
    assert!(contains(rect, (rows[0].x, rows[0].y)));
    assert_eq!(rows[0].cells.len(), 1);
    assert_eq!(rows[0].cells[0].symbol, "◓");
    let after = apply_composed_surface_patch(
        &before,
        ClientComposedSurfacePatch {
            rows,
            cursor: before.cursor.clone(),
        },
    )
    .unwrap();
    assert_eq!(after.cursor, before.cursor);
    assert_eq!(
        after
            .cells
            .iter()
            .zip(&before.cells)
            .filter(|(a, b)| a != b)
            .count(),
        1
    );

    state.sidebar_collapsed = true;
    state.compose(106, 3).unwrap();
    assert!(state.hits.agent_indicators.is_empty());
    assert!(state.agent_animation.deadline().is_none());
    assert!(state
        .tick_agent_animation(deadline + Duration::from_secs(1))
        .is_empty());
}

#[test]
fn blocked_animation_pulses_slowly_and_static_states_do_not_schedule_updates() {
    let mut state = animated_state(AgentStatus::Blocked);
    let now = state.agent_animation.deadline().unwrap();
    for tick in 0..4 {
        assert!(state
            .tick_agent_animation(now + Duration::from_millis(tick * 120))
            .is_empty());
    }
    let rows = state.tick_agent_animation(now + Duration::from_millis(480));
    assert!(!rows.is_empty());
    assert!(rows
        .iter()
        .all(|row| row.cells[0].modifier & Modifier::DIM.bits() != 0));
    assert!(rows.iter().any(|row| row.cells[0].symbol == "!"));
    for status in [AgentStatus::Idle, AgentStatus::Done, AgentStatus::Unknown] {
        let mut state = animated_state(status);
        assert!(state.agent_animation.deadline().is_none());
        assert!(state
            .tick_agent_animation(Instant::now() + Duration::from_secs(2))
            .is_empty());
    }
}

#[test]
fn unfocused_client_does_not_animate_agents() {
    let mut state = animated_state(AgentStatus::Working);
    let deadline = state.agent_animation.deadline().unwrap();
    state.outer_focused = Some(false);
    assert!(state.tick_agent_animation(deadline).is_empty());
    assert!(state.agent_animation.deadline().is_none());
}

#[test]
fn agent_indicators_distinguish_every_state_in_expanded_and_collapsed_lists() {
    for style in [
        crate::config::StatusIndicatorStyle::Dots,
        crate::config::StatusIndicatorStyle::Symbols,
    ] {
        for (status, symbol, badge, animated) in [
            (AgentStatus::Working, "◐", "running", true),
            (AgentStatus::Blocked, "!", "input", true),
            (AgentStatus::Done, "✓", "done", false),
            (AgentStatus::Idle, "⏾", "idle", false),
            (AgentStatus::Unknown, "?", "unknown", false),
        ] {
            let mut state = animated_state(status);
            state.config.status_indicators = style;
            for collapsed in [false, true] {
                state.sidebar_collapsed = collapsed;
                let frame = state.compose(106, 30).unwrap();
                let indicator = state
                    .hits
                    .agent_indicators
                    .iter()
                    .find(|hit| hit.spinner)
                    .unwrap();
                let index = usize::from(indicator.rect.y) * usize::from(frame.width)
                    + usize::from(indicator.rect.x);
                assert_eq!(frame.cells[index].symbol, symbol);
                assert_eq!(
                    frame.cells[index].fg,
                    crate::protocol::color_to_u32(status_color(status, &state.config.palette))
                );
                assert_eq!(unicode_width::UnicodeWidthStr::width(symbol), 1);
                assert_eq!(state.agent_animation.deadline().is_some(), animated);
                if !collapsed {
                    let badge_rect = state
                        .hits
                        .agent_indicators
                        .iter()
                        .find(|hit| !hit.spinner)
                        .unwrap()
                        .rect;
                    let start = usize::from(badge_rect.y) * usize::from(frame.width)
                        + usize::from(badge_rect.x);
                    let text: String = frame.cells[start..start + usize::from(badge_rect.width)]
                        .iter()
                        .map(|cell| cell.symbol.as_str())
                        .collect();
                    assert_eq!(text.trim(), badge);
                    assert_eq!(
                        frame.cells[start].bg,
                        frame.cells[start - 1].bg,
                        "status text must not paint its own background"
                    );
                }
            }
        }
    }
}
