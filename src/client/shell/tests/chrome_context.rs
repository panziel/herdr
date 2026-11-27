use super::*;

#[test]
fn workspace_without_branch_keeps_its_second_row_and_full_hit_area() {
    let mut projected = snapshot();
    projected.workspaces[0].branch = None;
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(surface());
    let frame = state.compose(106, 24).unwrap();
    let hit = &state.hits.workspaces[0];
    assert_eq!(hit.rect.height, 2);
    let buffer = frame.to_ratatui_buffer().unwrap();
    let second_row: String = (hit.rect.x..hit.rect.right())
        .map(|x| buffer[(x, hit.rect.y + 1)].symbol())
        .collect();
    assert!(
        second_row.contains('无') && second_row.contains('分') && second_row.contains('支'),
        "{second_row:?}"
    );
}

#[test]
fn tab_chips_use_powerline_edges_and_descriptive_default_names() {
    let mut projected = snapshot();
    let mut second = projected.tabs[0].clone();
    second.tab_id = "tab_2".into();
    second.number = 2;
    second.label = "2".into();
    second.custom_label = true;
    second.focused = false;
    projected.tabs.push(second);
    let mut custom = projected.tabs[0].clone();
    custom.tab_id = "tab_3".into();
    custom.number = 3;
    custom.label = "build".into();
    custom.custom_label = true;
    custom.focused = false;
    projected.tabs.push(custom);
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(surface());
    let frame = state.compose(106, 20).unwrap();
    let buffer = frame.to_ratatui_buffer().unwrap();
    for ((rect, _), label) in state.hits.tabs.iter().zip(["Tab #1", "Tab #2", "build"]) {
        let text: String = (rect.x..rect.right())
            .map(|x| buffer[(x, rect.y)].symbol())
            .collect();
        assert!(text.contains(label), "{text:?}");
    }
    for (index, (rect, _)) in state.hits.tabs.iter().enumerate() {
        assert_eq!(buffer[(rect.x, rect.y)].symbol(), "");
        assert_eq!(
            buffer[(rect.right() - 1, rect.y)].symbol(),
            "",
            "tab {index} should have a right powerline edge"
        );
    }
    let active = state.hits.tabs[0].0;
    assert_eq!(
        buffer[(active.x, active.y)].style().bg,
        Some(state.config.palette.accent)
    );
    assert_eq!(
        buffer[(active.x, active.y)].style().fg,
        Some(state.config.palette.panel_bg)
    );
}

#[test]
fn tab_titles_use_one_style_for_default_and_custom_names() {
    let mut projected = snapshot();
    let mut auto = projected.tabs[0].clone();
    auto.tab_id = "tab_2".into();
    auto.number = 2;
    auto.label = "2".into();
    auto.custom_label = false;
    auto.focused = false;
    projected.tabs.push(auto);
    let mut custom = projected.tabs[0].clone();
    custom.tab_id = "tab_3".into();
    custom.number = 3;
    custom.label = "build".into();
    custom.custom_label = true;
    custom.focused = false;
    projected.tabs.push(custom);
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(surface());
    let frame = state.compose(106, 20).unwrap();
    let buffer = frame.to_ratatui_buffer().unwrap();
    let body_style = |tab_id: &str| {
        let (rect, _) = state
            .hits
            .tabs
            .iter()
            .find(|(_, id)| id == tab_id)
            .expect("tab hit");
        buffer[(rect.x + 1, rect.y)].style()
    };
    let auto_style = body_style("tab_2");
    let custom_style = body_style("tab_3");
    assert_eq!(auto_style.fg, custom_style.fg);
    assert_eq!(auto_style.add_modifier, custom_style.add_modifier);
    assert_eq!(auto_style.fg, Some(state.config.palette.overlay0));
    assert!(!auto_style.add_modifier.contains(Modifier::BOLD));
    assert!(!auto_style.add_modifier.contains(Modifier::DIM));
}

#[test]
fn auto_named_tab_shows_tab_number_even_when_label_is_stale_position() {
    let mut projected = snapshot();
    projected.tabs[0].number = 5;
    projected.tabs[0].label = "2".into();
    projected.tabs[0].custom_label = false;
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(surface());
    let frame = state.compose(106, 20).unwrap();
    let buffer = frame.to_ratatui_buffer().unwrap();
    let (rect, _) = state.hits.tabs[0];
    let text: String = (rect.x..rect.right())
        .map(|x| buffer[(x, rect.y)].symbol())
        .collect();
    assert!(text.contains("Tab #5"), "{text:?}");
}

#[test]
fn powerline_tabs_join_the_body_with_a_left_notch_and_right_arrow() {
    use ratatui::style::Color;

    fn visible_colors(cell: &ratatui::buffer::Cell) -> (Color, Color) {
        let foreground = match cell.fg {
            Color::Reset => Color::Rgb(192, 197, 205),
            color => color,
        };
        let background = match cell.bg {
            Color::Reset => Color::Rgb(30, 45, 59),
            color => color,
        };
        if cell.modifier.contains(Modifier::REVERSED) {
            (background, foreground)
        } else {
            (foreground, background)
        }
    }

    let mut terminal = crate::app::state::Palette::terminal();
    terminal.accent = Color::Rgb(80, 196, 70);
    let default_palette = ClientShellConfig::from_config(&Config::default()).palette;
    for palette in [default_palette, terminal] {
        for focused in 0..4 {
            let mut projected = snapshot();
            projected.tabs = (0..4)
                .map(|index| ClientShellTab {
                    tab_id: format!("tab_{}", index + 1),
                    number: index + 1,
                    label: (index + 1).to_string(),
                    focused: index == focused,
                    ..projected.tabs[0].clone()
                })
                .collect();
            projected.focused_tab_id = Some(projected.tabs[focused].tab_id.clone());
            let mut state =
                ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
            state.config.palette = palette.clone();
            state.set_snapshot(Box::new(projected));
            state.set_pane_surface(surface());

            let frame = state.compose(106, 20).expect("powerline tab bar");
            let buffer = frame.to_ratatui_buffer().expect("tab bar frame");
            let bar = visible_colors(&buffer[(state.hits.new_tab.x, 0)]).1;
            assert_eq!(state.hits.tabs.len(), 4);
            for (index, (rect, _)) in state.hits.tabs.iter().enumerate() {
                let body = visible_colors(&buffer[(rect.x + 1, rect.y)]).1;
                assert_ne!(body, bar, "tab {index} needs a filled body");
                if index == focused {
                    assert_eq!(body, palette.accent);
                }
                let leading = &buffer[(rect.x, rect.y)];
                let trailing = &buffer[(rect.right() - 1, rect.y)];
                assert_eq!(leading.symbol(), "");
                assert_eq!(trailing.symbol(), "");
                assert_eq!(visible_colors(leading), (bar, body), "left notch");
                assert_eq!(visible_colors(trailing), (body, bar), "right arrow");
                for x in rect.x + 1..rect.right() - 1 {
                    assert_eq!(visible_colors(&buffer[(x, rect.y)]).1, body);
                }
                if let Some((next, _)) = state.hits.tabs.get(index + 1) {
                    assert_eq!(rect.right(), next.x, "adjacent arrow and notch");
                }
            }
        }
    }
}

#[test]
fn opening_new_tab_overlay_preserves_focused_tab_title_contrast() {
    let mut projected = snapshot();
    let mut second = projected.tabs[0].clone();
    second.tab_id = "tab_2".into();
    second.number = 2;
    second.label = "2".into();
    second.focused = false;
    projected.tabs.push(second);

    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(surface());
    let mut open = ClientShellInput::default();
    state.record_binding(
        crate::input::KeybindMatch::Action(crate::input::KeybindAction::NewTab),
        &mut open,
    );

    let frame = state.compose(106, 20).expect("new tab overlay");
    let buffer = frame.to_ratatui_buffer().expect("overlay frame");
    let focused = state
        .hits
        .tabs
        .iter()
        .find(|(_, tab_id)| tab_id == "tab_1")
        .map(|(rect, _)| *rect)
        .expect("focused tab hit");
    let inactive = state
        .hits
        .tabs
        .iter()
        .find(|(_, tab_id)| tab_id == "tab_2")
        .map(|(rect, _)| *rect)
        .expect("inactive tab hit");
    let focused_title = (focused.x + 1..focused.right() - 1)
        .find(|&x| buffer[(x, focused.y)].symbol() == "T")
        .expect("focused tab title");
    let inactive_title = (inactive.x + 1..inactive.right() - 1)
        .find(|&x| buffer[(x, inactive.y)].symbol() == "T")
        .expect("inactive tab title");
    assert!(!buffer[(focused_title, focused.y)]
        .modifier
        .contains(Modifier::DIM));
    for x in [focused.x, focused.right() - 1] {
        assert!(!buffer[(x, focused.y)].modifier.contains(Modifier::DIM));
    }
    assert!(!buffer[(inactive_title, inactive.y)]
        .modifier
        .contains(Modifier::DIM));
    for (rect, _) in &state.hits.tabs {
        for x in rect.x..rect.right() {
            assert!(!buffer[(x, rect.y)].modifier.contains(Modifier::DIM));
        }
    }
}

#[test]
fn opening_new_tab_overlay_keeps_config_diagnostic_style_uniform() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(snapshot()));
    state.set_pane_surface(surface());
    let diagnostic = "client + endpoint: config.toml invalid; using defaults; herdr config check";
    state.config_diagnostic = Some(diagnostic.into());
    state.record_binding(
        crate::input::KeybindMatch::Action(crate::input::KeybindAction::NewTab),
        &mut ClientShellInput::default(),
    );

    let frame = state
        .compose(80, 24)
        .expect("diagnostic with new tab overlay");
    let buffer = frame.to_ratatui_buffer().expect("diagnostic frame");
    let (start, y) = cell_symbol_position(&frame, buffer.area, "client + endpoint:");
    let style = buffer[(start, y)].style();
    for x in start..start + diagnostic.len() as u16 {
        assert_eq!(buffer[(x, y)].style(), style, "diagnostic column {x}");
    }
}

#[test]
fn tab_overflow_controls_scroll_the_client_owned_tab_bar() {
    let mut snapshot = snapshot();
    snapshot.tabs.extend((2..=8).map(|number| ClientShellTab {
        tab_id: format!("tab_{number}"),
        workspace_id: "ws_1".into(),
        number,
        label: number.to_string(),
        custom_label: false,
        zoomed: false,
        focused: false,
        agent_status: AgentStatus::Idle,
    }));
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(snapshot));
    state.set_pane_surface(surface());
    state.compose(80, 20).expect("overflow tab bar");

    assert!(state.hits.tab_scroll_right.width > 0);
    let scroll_right = state.hits.tab_scroll_right;
    let outcome =
        state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: scroll_right.x + 1,
            row: scroll_right.y,
            modifiers: KeyModifiers::empty(),
        })]);
    assert!(outcome.repaint);
    assert_eq!(state.tab_scroll, 1);

    let mut update = state.snapshot.as_deref().expect("snapshot").clone();
    update.focused_tab_id = Some("tab_8".into());
    for tab in &mut update.tabs {
        tab.focused = tab.tab_id == "tab_8";
    }
    state.set_snapshot(Box::new(update));
    state.compose(80, 20).expect("focused overflow tab");
    assert!(state.hits.tabs.iter().any(|(_, tab_id)| tab_id == "tab_8"));

    state.compose(300, 20).expect("tabs without overflow");
    assert_eq!(state.tab_scroll, 0);
    assert_eq!(state.hits.tabs.len(), 8);
    state.compose(80, 20).expect("focused tab after narrowing");
    assert!(state.hits.tabs.iter().any(|(_, tab_id)| tab_id == "tab_8"));
}

#[test]
fn focused_last_overflow_tab_shows_its_full_label() {
    let mut projected = snapshot();
    let labels = [
        "1",
        "Laiza Portfolio Site",
        "linkedin posts",
        "update cv",
        "nvim",
        "brother",
        "day organiser",
        "nvim test",
    ];
    projected.tabs = labels
        .iter()
        .enumerate()
        .map(|(index, label)| ClientShellTab {
            tab_id: format!("tab_{}", index + 1),
            workspace_id: "ws_1".into(),
            number: index + 1,
            label: (*label).into(),
            custom_label: index > 0,
            zoomed: false,
            focused: index == 7,
            agent_status: AgentStatus::Idle,
        })
        .collect();
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    for number in [8, 7, 8] {
        let tab_id = format!("tab_{number}");
        projected.focused_tab_id = Some(tab_id.clone());
        projected.workspaces[0].active_tab_id = tab_id.clone();
        projected.panes[0].tab_id = tab_id.clone();
        for tab in &mut projected.tabs {
            tab.focused = tab.tab_id == tab_id;
        }
        state.set_snapshot(Box::new(projected.clone()));
        state.set_pane_surface(surface());
        let frame = state
            .compose(133, 20)
            .expect("reporter's overflowing strip");
        assert_eq!(
            state.hits.new_tab.right() - state.hits.tab_scroll_left.x,
            107
        );
        let rect = state
            .hits
            .tabs
            .iter()
            .find(|(_, id)| id == &tab_id)
            .expect("focused tab")
            .0;
        let text = (rect.x..rect.right())
            .map(|x| {
                frame.cells[(rect.y * frame.width + x) as usize]
                    .symbol
                    .as_str()
            })
            .collect::<String>();
        assert!(
            text.contains(labels[number - 1]),
            "focused tab rendered as {text:?}, rect={rect:?}"
        );
    }

    // Manual scrolling must be able to reveal the rest of a partially drawn last tab.
    let end_scroll = state.tab_scroll;
    let left = state.hits.tab_scroll_left;
    state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: left.x + 1,
        row: left.y,
        modifiers: KeyModifiers::empty(),
    })]);
    let frame = state.compose(133, 20).expect("manual scroll left");
    assert_eq!(state.tab_scroll, end_scroll - 1);
    let right = state.hits.tab_scroll_right;
    assert_eq!(
        frame.cells[(right.y * frame.width + right.x + 1) as usize].fg,
        crate::protocol::color_to_u32(state.config.palette.overlay1),
        "right arrow stays enabled while the final tab is clipped"
    );
    state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: right.x + 1,
        row: right.y,
        modifiers: KeyModifiers::empty(),
    })]);
    let frame = state.compose(133, 20).expect("manual scroll right");
    assert_eq!(state.tab_scroll, end_scroll);
    assert!(frame_rows(&frame)[0].contains("nvim test"));
    assert_eq!(
        frame.cells[(right.y * frame.width + right.x + 1) as usize].fg,
        crate::protocol::color_to_u32(state.config.palette.overlay0),
        "right arrow dims at the useful scroll limit"
    );
}

#[test]
fn focused_workspace_change_reveals_new_workspace_in_full_sidebar() {
    let mut initial = snapshot();
    let template = initial.workspaces[0].clone();
    initial.workspaces = (1..=12)
        .map(|number| ClientShellWorkspace {
            workspace_id: format!("ws_{number}"),
            number,
            label: format!("space-{number}"),
            branch: None,
            focused: number == 1,
            ..template.clone()
        })
        .collect();

    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(initial));
    state.set_pane_surface(surface());
    state.compose(106, 20).expect("full sidebar");
    assert!(state.hits.workspace_max_scroll > 0);
    assert!(state
        .hits
        .workspaces
        .iter()
        .all(|hit| hit.workspace_id != "ws_12"));

    let mut update = state.snapshot.as_deref().expect("snapshot").clone();
    update.revision = 2;
    update.focused_workspace_id = Some("ws_12".into());
    for workspace in &mut update.workspaces {
        workspace.focused = workspace.workspace_id == "ws_12";
    }
    let mut updated_surface = surface();
    updated_surface.projection_revision = 2;
    state.set_snapshot(Box::new(update));
    state.set_pane_surface(updated_surface);
    state.compose(106, 2).expect("zero-height workspace body");
    assert!(state.reveal_focused_workspace);
    state.compose(106, 20).expect("updated full sidebar");

    assert!(state
        .hits
        .workspaces
        .iter()
        .any(|hit| hit.workspace_id == "ws_12"));
}

#[test]
fn client_owned_sidebar_dividers_resize_live() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(snapshot()));
    state.set_pane_surface(surface());
    state.compose(106, 30).expect("expanded sidebar");
    assert!(state.hits.machines.is_empty());
    let workspace_body = state.hits.workspace_body;
    let needless_scroll =
        state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: workspace_body.x,
            row: workspace_body.y,
            modifiers: KeyModifiers::empty(),
        })]);
    assert_eq!(state.hits.workspace_max_scroll, 0);
    assert_eq!(state.workspace_scroll, 0);
    assert!(!needless_scroll.repaint);
    let width_divider = state.hits.sidebar_divider;
    state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: width_divider.x,
        row: width_divider.y + 2,
        modifiers: KeyModifiers::empty(),
    })]);
    let resize =
        state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: 31,
            row: width_divider.y + 2,
            modifiers: KeyModifiers::empty(),
        })]);
    assert_eq!(state.sidebar_width, 32);
    assert!(state.sidebar_width_manual);
    assert!(resize.repaint);
    assert!(resize.resize);
    let waiting_frame = state.compose(106, 30).expect("waiting for resized surface");
    let waiting_text: String = waiting_frame
        .cells
        .iter()
        .map(|cell| cell.symbol.as_str())
        .collect();
    assert!(
        waiting_text.contains(" Spaces"),
        "local sidebar must keep spaces while resizing: {waiting_text}"
    );
    assert!(!waiting_text.contains(" Machines"));
    assert!(!waiting_text.contains("Select a connected machine"));
    assert!(!waiting_text.contains("LIVE"));
    assert!(waiting_frame.cursor.is_none());
    assert!(state.pane_surface.is_none());
    assert!(state.hits.panes.is_empty());
    assert!(state.hits.pane_splits.is_empty());
    assert!(state.hits.machines.is_empty());
    assert_eq!(state.hits.sidebar_divider.x, 31);
    assert_eq!(state.hits.workspaces[0].workspace_id, "ws_1");

    let next_resize =
        state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: 32,
            row: width_divider.y + 2,
            modifiers: KeyModifiers::empty(),
        })]);
    assert!(next_resize.resize);
    state.compose(106, 30).expect("continued resize");
    assert_eq!(state.hits.sidebar_divider.x, 32);
    state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: 32,
        row: width_divider.y + 2,
        modifiers: KeyModifiers::empty(),
    })]);
    assert!(state.chrome_drag.is_none());

    state.set_pane_surface(surface());
    let recovered_frame = state.compose(106, 30).expect("resized sidebar");
    let recovered_text: String = recovered_frame
        .cells
        .iter()
        .map(|cell| cell.symbol.as_str())
        .collect();
    assert!(recovered_text.contains(" Spaces"));
    assert!(recovered_text.contains("LIVE"));
    assert!(!state.hits.panes.is_empty());
    let section_divider = state.hits.sidebar_section_divider;
    state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: section_divider.x + 2,
        row: section_divider.y,
        modifiers: KeyModifiers::empty(),
    })]);
    let split = state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: section_divider.x + 2,
        row: 20,
        modifiers: KeyModifiers::empty(),
    })]);
    assert!(state.sidebar_section_split > 0.6);
    assert!(split.repaint);
    assert!(!split.resize);
}

#[test]
fn context_menus_capture_stable_targets_and_route_actions() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(snapshot()));
    state.set_pane_surface(surface());
    state.compose(106, 20).expect("composed frame");

    let workspace = state.hits.workspaces[0].rect;
    let open_workspace_menu =
        state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Right),
            column: workspace.x + 2,
            row: workspace.y,
            modifiers: KeyModifiers::empty(),
        })]);
    assert!(open_workspace_menu.actions.is_empty());
    assert!(matches!(
        state.overlay,
        Some(ClientShellOverlay::ContextMenu(ClientContextMenuOverlay {
            target: ClientContextMenuTarget::Workspace { ref workspace_id, .. },
            ..
        })) if workspace_id == "ws_1"
    ));
    let workspace_items = match state.overlay.as_ref() {
        Some(ClientShellOverlay::ContextMenu(menu)) => menu.items(),
        _ => panic!("workspace context menu"),
    };
    assert!(workspace_items
        .iter()
        .any(|item| item.action == ClientContextMenuAction::NewWorktree));
    state.compose(106, 20).expect("workspace context menu");
    let rename = state.hits.context_menu_rows[0].0;
    state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: rename.x + 1,
        row: rename.y,
        modifiers: KeyModifiers::empty(),
    })]);
    assert!(matches!(
        state.overlay,
        Some(ClientShellOverlay::Rename(ClientRenameOverlay {
            target: ClientRenameTarget::Workspace { ref workspace_id },
            ..
        })) if workspace_id == "ws_1"
    ));

    state.overlay = None;
    state.compose(106, 20).expect("composed frame");
    let pane = state.hits.panes[0].rect;
    state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Right),
        column: pane.x + 1,
        row: pane.y,
        modifiers: KeyModifiers::empty(),
    })]);
    state.compose(106, 20).expect("pane context menu");
    let split_index = match state.overlay.as_ref() {
        Some(ClientShellOverlay::ContextMenu(menu)) => menu
            .items()
            .iter()
            .position(|item| item.action == ClientContextMenuAction::SplitRight)
            .expect("split right item"),
        _ => panic!("pane context menu"),
    };
    let split = state.hits.context_menu_rows[split_index].0;
    let outcome =
        state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: split.x + 1,
            row: split.y,
            modifiers: KeyModifiers::empty(),
        })]);
    let [ClientShellAction::Endpoint { request, .. }] = &outcome.actions[..] else {
        panic!("pane split context action should use endpoint API");
    };
    assert!(matches!(
        &request.method,
        crate::api::schema::Method::PaneSplit(params)
            if params.target_pane_id.as_deref() == Some("pane_1")
                && params.direction == crate::api::schema::SplitDirection::Right
    ));
}

#[test]
fn global_menu_opens_from_sidebar_and_routes_client_actions() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(snapshot()));
    state.set_pane_surface(surface());
    state.compose(106, 30).expect("shell frame");
    let launcher = state.hits.global_launcher;
    assert_ne!(launcher, Rect::default());

    let open = state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: launcher.x,
        row: launcher.y,
        modifiers: KeyModifiers::empty(),
    })]);
    assert!(open.repaint);
    let menu = state.compose(106, 30).expect("global menu");
    let text = menu
        .cells
        .chunks(menu.width as usize)
        .map(|row| {
            row.iter()
                .map(|cell| cell.symbol.as_str())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("settings"));
    assert!(text.contains("keybinds"));
    assert!(text.contains("reload config"));
    assert!(text.contains("detach"));

    let keybinds = state.hits.global_menu_rows[1].0;
    let help = state.handle_raw_events(vec![RawInputEvent::Mouse(crossterm::event::MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: keybinds.x,
        row: keybinds.y,
        modifiers: KeyModifiers::empty(),
    })]);
    assert!(help.actions.is_empty());
    assert!(matches!(state.overlay, Some(ClientShellOverlay::Help(_))));

    state.overlay = Some(ClientShellOverlay::GlobalMenu(ClientGlobalMenuOverlay {
        highlighted: 3,
    }));
    let detach = state.handle_input_bytes(b"\r");
    assert!(detach.detach);
    assert!(state.overlay.is_none());
}

#[test]
fn new_tab_overlay_owns_text_cursor_and_submits_public_api_request() {
    let mut config = Config::default();
    config.ui.prompt_new_tab_name = true;
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&config));
    state.set_snapshot(Box::new(snapshot()));
    state.set_pane_surface(surface());
    let mut open = ClientShellInput::default();
    state.record_binding(
        crate::input::KeybindMatch::Action(crate::input::KeybindAction::NewTab),
        &mut open,
    );
    assert!(open.actions.is_empty());
    let frame = state.compose(106, 20).expect("new tab overlay");
    let text = frame
        .cells
        .chunks(frame.width as usize)
        .map(|row| {
            row.iter()
                .map(|cell| cell.symbol.as_str())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("new tab"));
    assert!(text.contains("save"));
    let restored = frame.to_ratatui_buffer().expect("overlay frame");
    assert!(!restored
        .cell((29, 6))
        .expect("overlay title cell")
        .modifier
        .contains(Modifier::DIM));
    assert!(frame.cursor.as_ref().is_some_and(|cursor| cursor.visible));

    assert!(state.handle_input_bytes(b"logs").actions.is_empty());
    let create = state.handle_input_bytes(b"\r");
    let [ClientShellAction::Endpoint { request, .. }] = &create.actions[..] else {
        panic!("new tab save should use endpoint API");
    };
    assert!(matches!(
        &request.method,
        crate::api::schema::Method::TabCreate(params)
            if params.workspace_id.as_deref() == Some("ws_1")
                && params.label.as_deref() == Some("logs")
    ));
    assert!(state.overlay.is_none());
}

#[test]
fn new_tab_overlay_interior_click_keeps_it_open() {
    let mut config = Config::default();
    config.ui.prompt_new_tab_name = true;
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&config));
    state.set_snapshot(Box::new(snapshot()));
    state.set_pane_surface(surface());
    let mut open = ClientShellInput::default();
    state.record_binding(
        crate::input::KeybindMatch::Action(crate::input::KeybindAction::NewTab),
        &mut open,
    );
    state.compose(106, 20).expect("new tab overlay");
    let popup = state.hits.overlay_popup;
    state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: popup.x + 2,
        row: popup.y + 1,
        modifiers: KeyModifiers::empty(),
    })]);
    assert!(matches!(state.overlay, Some(ClientShellOverlay::Rename(_))));
}

#[test]
fn new_tab_overlay_cancel_and_outside_clicks_dismiss() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(snapshot()));
    state.set_pane_surface(surface());
    let mut open = ClientShellInput::default();
    state.record_binding(
        crate::input::KeybindMatch::Action(crate::input::KeybindAction::NewTab),
        &mut open,
    );
    state.compose(106, 20).expect("new tab overlay");
    let cancel = state.hits.overlay_cancel;
    state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: cancel.x,
        row: cancel.y,
        modifiers: KeyModifiers::empty(),
    })]);
    assert!(state.overlay.is_none());

    let mut open = ClientShellInput::default();
    state.record_binding(
        crate::input::KeybindMatch::Action(crate::input::KeybindAction::NewTab),
        &mut open,
    );
    state.compose(106, 20).expect("new tab overlay");
    state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 0,
        row: 0,
        modifiers: KeyModifiers::empty(),
    })]);
    assert!(state.overlay.is_none());
}

#[test]
fn close_confirmation_error_becomes_client_owned_overlay_and_stable_workspace_close() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(snapshot()));
    state.set_pane_surface(surface());
    let mut close = ClientShellInput::default();
    // Exercise the server error fallback directly: last-pane keybindings now
    // confirm locally before issuing a request.
    state.push_endpoint_method(
        crate::api::schema::Method::PaneClose(crate::api::schema::PaneTarget {
            pane_id: "pane_1".into(),
        }),
        &mut close,
    );
    let [ClientShellAction::Endpoint { request, .. }] = &close.actions[..] else {
        panic!("pane close should use endpoint API");
    };
    let request_id = request.id.clone();
    assert!(
        state
            .handle_endpoint_result(
                "boot-1",
                &request_id,
                Err(ClientShellEndpointError {
                    code: Some("confirmation_required".into()),
                    message: "confirmation required".into(),
                }),
            )
            .0
    );
    let frame = state.compose(106, 20).expect("confirmation overlay");
    let text = frame
        .cells
        .chunks(frame.width as usize)
        .map(|row| {
            row.iter()
                .map(|cell| cell.symbol.as_str())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("Close workspace?"));
    assert!(text.contains("1 pane"));

    let confirm = state.handle_input_bytes(b"\r");
    let [ClientShellAction::Endpoint { request, .. }] = &confirm.actions[..] else {
        panic!("confirmation should use endpoint API");
    };
    assert!(matches!(
        &request.method,
        crate::api::schema::Method::WorkspaceClose(params)
            if params.workspace_id == "ws_1" && !params.close_group
    ));
}

#[test]
fn compact_card_is_three_rows_with_accent_border_and_no_fill() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.config.spaces.card_style = SidebarCardStyle::Compact;
    state.set_snapshot(Box::new(snapshot()));
    state.set_pane_surface(surface());
    let frame = state.compose(106, 24).unwrap();
    let buffer = frame.to_ratatui_buffer().unwrap();
    let hit = state.hits.workspaces[0].rect;
    assert_eq!(hit.height, 3);
    assert_eq!(buffer[(hit.x, hit.y)].symbol(), "╭");
    assert_eq!(buffer[(hit.right() - 1, hit.y)].symbol(), "╮");
    assert_eq!(buffer[(hit.x, hit.bottom() - 1)].symbol(), "╰");
    assert_eq!(buffer[(hit.right() - 1, hit.bottom() - 1)].symbol(), "╯");
    assert_eq!(
        buffer[(hit.x, hit.y)].style().fg,
        Some(state.config.palette.accent)
    );
    let border_row: String = (hit.x..hit.right())
        .map(|x| buffer[(x, hit.y)].symbol())
        .collect();
    let name_row: String = (hit.x..hit.right())
        .map(|x| buffer[(x, hit.y + 1)].symbol())
        .collect();
    assert!(name_row.contains("client-shell"), "{name_row:?}");
    assert!(border_row.contains("main"), "{border_row:?}");
    // 分支标题带左边距压在上边框上，不顶着圆角。
    assert!(border_row.starts_with("╭── main "), "{border_row:?}");
    assert!(!name_row.contains("main"), "{name_row:?}");
    let name_x = (hit.x + 1..hit.right() - 1)
        .find(|&x| buffer[(x, hit.y + 1)].symbol() == "c")
        .expect("name cell");
    assert_ne!(
        buffer[(name_x, hit.y + 1)].style().bg,
        Some(state.config.palette.active_row_bg)
    );
}

#[test]
fn compact_card_without_branch_omits_placeholder() {
    let mut projected = snapshot();
    projected.workspaces[0].branch = None;
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.config.spaces.card_style = SidebarCardStyle::Compact;
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(surface());
    let frame = state.compose(106, 24).unwrap();
    let buffer = frame.to_ratatui_buffer().unwrap();
    let hit = state.hits.workspaces[0].rect;
    assert_eq!(hit.height, 3);
    let border_row: String = (hit.x..hit.right())
        .map(|x| buffer[(x, hit.y)].symbol())
        .collect();
    let name_row: String = (hit.x..hit.right())
        .map(|x| buffer[(x, hit.y + 1)].symbol())
        .collect();
    assert!(name_row.contains("client-shell"), "{name_row:?}");
    assert!(!name_row.contains("无分支"), "{name_row:?}");
    assert!(!border_row.contains("无分支"), "{border_row:?}");
    assert!(
        border_row.chars().all(|c| c == '╭' || c == '╮' || c == '─'),
        "{border_row:?}"
    );
    assert!(!name_row.contains('·'), "{name_row:?}");
}

#[test]
fn full_card_with_branch_is_four_rows() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.config.spaces.card_style = SidebarCardStyle::Full;
    state.set_snapshot(Box::new(snapshot()));
    state.set_pane_surface(surface());
    let frame = state.compose(106, 24).unwrap();
    let buffer = frame.to_ratatui_buffer().unwrap();
    let hit = state.hits.workspaces[0].rect;
    assert_eq!(hit.height, 4);
    let name_row: String = (hit.x..hit.right())
        .map(|x| buffer[(x, hit.y + 1)].symbol())
        .collect();
    let branch_row: String = (hit.x..hit.right())
        .map(|x| buffer[(x, hit.y + 2)].symbol())
        .collect();
    assert!(name_row.contains("client-shell"), "{name_row:?}");
    assert!(branch_row.contains("main"), "{branch_row:?}");
}

#[test]
fn full_card_without_branch_degrades_to_three_rows() {
    let mut projected = snapshot();
    projected.workspaces[0].branch = None;
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.config.spaces.card_style = SidebarCardStyle::Full;
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(surface());
    let frame = state.compose(106, 24).unwrap();
    let buffer = frame.to_ratatui_buffer().unwrap();
    let hit = state.hits.workspaces[0].rect;
    assert_eq!(hit.height, 3);
    let name_row: String = (hit.x..hit.right())
        .map(|x| buffer[(x, hit.y + 1)].symbol())
        .collect();
    assert!(!name_row.contains("无分支"), "{name_row:?}");
}

#[test]
fn unselected_card_border_uses_overlay0() {
    let mut projected = snapshot();
    let mut second = projected.workspaces[0].clone();
    second.workspace_id = "ws_2".into();
    second.label = "other".into();
    second.focused = false;
    projected.workspaces.push(second);
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.config.spaces.card_style = SidebarCardStyle::Compact;
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(surface());
    let frame = state.compose(106, 24).unwrap();
    let buffer = frame.to_ratatui_buffer().unwrap();
    let focused = state.hits.workspaces[0].rect;
    let inactive = state.hits.workspaces[1].rect;
    assert_eq!(
        buffer[(focused.x, focused.y)].style().fg,
        Some(state.config.palette.accent)
    );
    assert_eq!(
        buffer[(inactive.x, inactive.y)].style().fg,
        Some(state.config.palette.overlay0)
    );
}

#[test]
fn indented_worktree_child_is_own_card_without_tree_prefix() {
    let mut projected = snapshot();
    projected.workspaces[0].worktree = Some(crate::protocol::ClientShellWorktree {
        key: "repo".into(),
        label: "repo".into(),
        is_linked_worktree: false,
    });
    let mut child = projected.workspaces[0].clone();
    child.workspace_id = "ws_2".into();
    child.label = "feature-ws".into();
    child.branch = Some("worktree/feature".into());
    child.custom_label = false;
    child.focused = false;
    child.worktree = Some(crate::protocol::ClientShellWorktree {
        key: "repo".into(),
        label: "repo".into(),
        is_linked_worktree: true,
    });
    projected.workspaces.push(child);
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.config.spaces.card_style = SidebarCardStyle::Compact;
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(surface());
    let frame = state.compose(106, 24).unwrap();
    let buffer = frame.to_ratatui_buffer().unwrap();
    let child_hit = state.hits.workspaces[1].rect;
    assert!(state.hits.workspaces[1].indented);
    assert_eq!(child_hit.height, 3);
    let name_row: String = (child_hit.x..child_hit.right())
        .map(|x| buffer[(x, child_hit.y + 1)].symbol())
        .collect();
    assert!(name_row.contains("feature"), "{name_row:?}");
    assert!(!name_row.contains("├─"), "{name_row:?}");
    assert!(!name_row.contains("└─"), "{name_row:?}");
}

#[test]
fn workspace_card_hover_recolors_unfocused_border() {
    let mut projected = snapshot();
    let mut second = projected.workspaces[0].clone();
    second.workspace_id = "ws_2".into();
    second.label = "other".into();
    second.focused = false;
    projected.workspaces.push(second);
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.config.spaces.card_style = SidebarCardStyle::Compact;
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(surface());
    state.compose(106, 24).unwrap();
    let inactive = state.hits.workspaces[1].rect;
    let focused = state.hits.workspaces[0].rect;
    let mouse = MouseEvent {
        kind: MouseEventKind::Moved,
        column: inactive.x + 1,
        row: inactive.y + 1,
        modifiers: KeyModifiers::NONE,
    };
    assert!(
        state
            .handle_raw_events(vec![RawInputEvent::Mouse(mouse)])
            .repaint
    );
    let hovered = state.compose(106, 24).unwrap();
    let idx = |x: u16, y: u16| usize::from(y) * usize::from(hovered.width) + usize::from(x);
    assert_eq!(
        hovered.cells[idx(inactive.x, inactive.y)].fg,
        crate::protocol::color_to_u32(state.config.palette.hover_border_color())
    );
    assert_eq!(
        hovered.cells[idx(focused.x, focused.y)].fg,
        crate::protocol::color_to_u32(state.config.palette.accent)
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
    let restored = state.compose(106, 24).unwrap();
    assert_eq!(
        restored.cells[idx(inactive.x, inactive.y)].fg,
        crate::protocol::color_to_u32(state.config.palette.overlay0)
    );
}

#[test]
fn plain_workspace_list_does_not_hover() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(snapshot()));
    state.set_pane_surface(surface());
    state.compose(106, 24).unwrap();
    let hit = state.hits.workspaces[0].rect;
    state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Moved,
        column: hit.x + 1,
        row: hit.y,
        modifiers: KeyModifiers::NONE,
    })]);
    assert!(state.hovered_card.is_none());
}

#[test]
fn context_menu_edges_split_background_wide_characters_cleanly() {
    let palette = ClientShellConfig::from_config(&Config::default()).palette;
    let menu = ClientContextMenuOverlay {
        target: ClientContextMenuTarget::Tab {
            tab_id: "tab_1".into(),
            workspace_id: "ws_1".into(),
        },
        x: 35,
        y: 8,
        highlighted: 0,
    };
    let mut buffer = Buffer::empty(Rect::new(0, 0, 106, 40));
    for y in 8..20 {
        buffer.set_string(34, y, "果", Style::default());
    }

    let rendered = render::render_context_menu(&mut buffer, &menu, &palette).expect("menu");

    let area = rendered.area;
    assert_eq!(area.x, 35);
    for y in area.top()..area.bottom() {
        assert_eq!(
            buffer[(34, y)].symbol(),
            " ",
            "a wide character left of the menu at row {y} would cover its edge"
        );
        assert_ne!(buffer[(area.x, y)].symbol(), " ", "menu edge at row {y}");
    }
}
