use super::*;
use crate::client::agent_icons::{AgentIconImage, AgentIconTone};

const PLACEHOLDER: char = '\u{10EEEE}';

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

fn icon_state(style: SidebarCardStyle, host: bool) -> ClientShellState {
    let config = ClientShellConfig::from_config(&Config::default()).with_agent_icon_host(host);
    let mut state = ClientShellState::new(config);
    state.config.agents.card_style = style;
    let mut projected = snapshot();
    projected.agents = vec![
        agent("pane_1", "claude", AgentStatus::Working, true),
        agent("pane_2", "droid", AgentStatus::Idle, false),
    ];
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(surface());
    state
}

fn cell(frame: &FrameData, x: u16, y: u16) -> &crate::protocol::CellData {
    &frame.cells[usize::from(y) * usize::from(frame.width) + usize::from(x)]
}

/// 返回该行第一个占位符单元格的列号。
fn placeholder_column(frame: &FrameData, rect: Rect, y: u16) -> Option<u16> {
    (rect.x..rect.right()).find(|x| cell(frame, *x, y).symbol.starts_with(PLACEHOLDER))
}

fn claude_icon() -> AgentIconImage {
    AgentIconImage::for_agent(crate::detect::Agent::Claude, AgentIconTone::Dark).unwrap()
}

fn expected_fg(image: AgentIconImage) -> u32 {
    let glyph = crate::client::agent_icons::AgentIconSlot::Image(image).glyph();
    crate::protocol::color_to_u32(glyph.style.fg.unwrap())
}

#[test]
fn compact_card_draws_icon_between_status_and_name() {
    let mut state = icon_state(SidebarCardStyle::Compact, true);
    let frame = state.compose(106, 30).unwrap();
    let rect = state.hits.agents[0].0;
    let y = rect.y + 1;
    let x = placeholder_column(&frame, rect, y).expect("claude icon placeholder");

    assert_eq!(cell(&frame, x - 1, y).symbol, " ");
    assert_eq!(cell(&frame, x - 2, y).symbol, "◐");
    assert_eq!(cell(&frame, x, y).symbol, "\u{10EEEE}\u{0305}\u{0305}");
    assert_eq!(cell(&frame, x + 1, y).symbol, "\u{10EEEE}\u{0305}\u{030D}");
    assert_eq!(cell(&frame, x, y).fg, expected_fg(claude_icon()));
    assert_eq!(cell(&frame, x + 1, y).fg, expected_fg(claude_icon()));
    let name: String = (x + 2..x + 9)
        .map(|x| cell(&frame, x, y).symbol.as_str())
        .collect();
    assert_eq!(name, " claude");
    assert_eq!(frame.agent_icons, vec![claude_icon()]);
}

#[test]
fn agents_without_bundled_icons_keep_names_aligned() {
    let mut state = icon_state(SidebarCardStyle::Compact, true);
    let frame = state.compose(106, 30).unwrap();
    let rows = frame_rows(&frame);
    let claude = state.hits.agents[0].0;
    let droid = state.hits.agents[1].0;
    let name_column = |rect: Rect, name: &str| {
        let line: Vec<&str> = (rect.x..rect.right())
            .map(|x| cell(&frame, x, rect.y + 1).symbol.as_str())
            .collect();
        (0..line.len()).find(|start| line[*start..].concat().starts_with(name))
    };
    assert_eq!(name_column(claude, "claude"), name_column(droid, "droid"));
    assert!(name_column(droid, "droid").is_some());
    assert!(placeholder_column(&frame, droid, droid.y + 1).is_none());
    assert!(!rows[usize::from(droid.y + 1)].contains(PLACEHOLDER));
}

#[test]
fn plain_rows_place_icon_before_agent_name_and_keep_status_badge() {
    let mut state = icon_state(SidebarCardStyle::Plain, true);
    let frame = state.compose(106, 30).unwrap();
    let rect = state.hits.agents[0].0;
    let y = rect.y + 1;
    let x = placeholder_column(&frame, rect, y).expect("icon on the agent row");
    assert_eq!(x, rect.x + 3);
    let line: String = (x + 2..rect.right())
        .map(|x| cell(&frame, x, y).symbol.as_str())
        .collect();
    assert!(line.starts_with(" claude"), "{line:?}");
    assert!(line.trim_end().ends_with("running"), "{line:?}");
    assert_eq!(frame.agent_icons, vec![claude_icon()]);
}

#[test]
fn icons_are_omitted_when_disabled_or_unsupported() {
    for (style, host, enabled) in [
        (SidebarCardStyle::Compact, false, true),
        (SidebarCardStyle::Compact, true, false),
        (SidebarCardStyle::Plain, false, true),
        (SidebarCardStyle::Plain, true, false),
    ] {
        let mut state = icon_state(style, host);
        state.config.agents.icons = enabled;
        let frame = state.compose(106, 30).unwrap();
        assert!(
            frame
                .cells
                .iter()
                .all(|cell| !cell.symbol.starts_with(PLACEHOLDER)),
            "{style:?} host={host} enabled={enabled}"
        );
        assert!(frame.agent_icons.is_empty());
        let rect = state.hits.agents[0].0;
        let line: String = (rect.x..rect.right())
            .map(|x| cell(&frame, x, rect.y + 1).symbol.as_str())
            .collect();
        assert!(line.contains(" claude"), "{line:?}");
    }
}

const PANE_TITLE: &str = "╭── claude  ◐ running ───────╮";

fn bordered_pane(pane_id: &str, rect: SurfaceRect) -> PaneSurfacePane {
    let mut pane = surface().panes.remove(0);
    pane.pane_id = pane_id.into();
    pane.rect = rect;
    pane.inner_rect = SurfaceRect {
        x: rect.x + 1,
        y: rect.y + 1,
        width: rect.width - 2,
        height: rect.height - 2,
    };
    pane
}

fn titled_surface(title: &str) -> PaneSurfaceFrame {
    let width = unicode_width::UnicodeWidthStr::width(title);
    let body = format!("│{}│", " ".repeat(width - 2));
    let bottom = format!("╰{}╯", "─".repeat(width - 2));
    let buffer = Buffer::with_lines([title, body.as_str(), body.as_str(), bottom.as_str()]);
    let mut surface = surface();
    surface.frame = FrameData::from_ratatui_buffer_with_hyperlinks(&buffer, None, &[]);
    surface.panes = vec![bordered_pane(
        "pane_1",
        SurfaceRect {
            x: 0,
            y: 0,
            width: width as u16,
            height: 4,
        },
    )];
    surface
}

fn pane_title_state(host: bool, surface: PaneSurfaceFrame) -> ClientShellState {
    let mut state = icon_state(SidebarCardStyle::Plain, host);
    state.set_pane_surface(surface);
    state
}

fn pane_title_row(state: &ClientShellState, frame: &FrameData) -> Vec<String> {
    let rect = state.hits.panes[0].rect;
    (rect.x..rect.right())
        .map(|x| cell(frame, x, rect.y).symbol.clone())
        .collect()
}

#[test]
fn pane_title_shows_the_running_agent_icon_before_its_name() {
    let mut state = pane_title_state(true, titled_surface(PANE_TITLE));
    let frame = state.compose(106, 30).unwrap();
    let row = pane_title_row(&state, &frame);
    let rect = state.hits.panes[0].rect;

    assert_eq!(row[..4].concat(), "╭── ");
    assert_eq!(row[4], "\u{10EEEE}\u{0305}\u{0305}");
    assert_eq!(row[5], "\u{10EEEE}\u{0305}\u{030D}");
    assert_eq!(
        cell(&frame, rect.x + 4, rect.y).fg,
        expected_fg(claude_icon())
    );
    assert_eq!(row[6..].concat(), " claude  ◐ running ────╮");
    assert_eq!(row.len(), unicode_width::UnicodeWidthStr::width(PANE_TITLE));
    assert!(frame.agent_icons.contains(&claude_icon()));
}

#[test]
fn pane_title_keeps_its_text_when_the_icon_does_not_fit_or_is_unsupported() {
    let crowded = "╭── claude  ◐ running ──╮";
    for (host, title) in [(true, crowded), (false, PANE_TITLE)] {
        let mut state = pane_title_state(host, titled_surface(title));
        let frame = state.compose(106, 30).unwrap();
        assert_eq!(
            pane_title_row(&state, &frame).concat(),
            title,
            "host={host}"
        );
    }
}

#[test]
fn pane_title_without_a_known_agent_icon_is_unchanged() {
    let mut state = pane_title_state(true, titled_surface(PANE_TITLE));
    let mut projected = snapshot();
    projected.agents = vec![agent("pane_1", "droid", AgentStatus::Working, true)];
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(titled_surface(PANE_TITLE));
    let frame = state.compose(106, 30).unwrap();
    assert_eq!(pane_title_row(&state, &frame).concat(), PANE_TITLE);
}

#[test]
fn pane_title_covered_by_a_floating_pane_is_left_alone() {
    let mut surface = titled_surface(PANE_TITLE);
    let cover = bordered_pane(
        "pane_float",
        SurfaceRect {
            x: 8,
            y: 0,
            width: 6,
            height: 3,
        },
    );
    surface.panes.insert(0, cover);
    let mut state = pane_title_state(true, surface);
    let frame = state.compose(106, 30).unwrap();
    let rect = state.hits.panes[1].rect;
    let row: String = (rect.x..rect.right())
        .map(|x| cell(&frame, x, rect.y).symbol.as_str())
        .collect();
    assert_eq!(row, PANE_TITLE);
}
