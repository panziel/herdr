use std::collections::HashMap;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::Line,
    widgets::{Paragraph, Widget},
};

use super::*;

pub(super) struct AgentRow {
    pub(super) pane_id: String,
    pub(super) status: crate::api::schema::AgentStatus,
    pub(super) focused: bool,
    pub(super) rows: Vec<Vec<crate::ui::ResolvedToken>>,
    pub(super) card: AgentCardContent,
}

/// Fixed content of an Agents card; card styles ignore the `rows` template.
pub(super) struct AgentCardContent {
    pub(super) label: String,
    /// 所属 space（workspace）名，compact 卡片画在上边框。
    pub(super) space: String,
    pub(super) machine: Option<String>,
    pub(super) tab: Option<String>,
}

impl AgentCardContent {
    /// full 卡片的位置行：`machine · space · tab`。
    fn location(&self) -> String {
        [
            self.machine.as_deref(),
            Some(self.space.as_str()),
            self.tab.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ")
    }

    /// compact 卡片名称后的上下文：`machine · tab`（space 已画在上边框）。
    fn context(&self) -> String {
        [self.machine.as_deref(), self.tab.as_deref()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" · ")
    }
}

pub(super) fn agent_row_lines(row: &AgentRow, style: SidebarCardStyle) -> usize {
    match style {
        SidebarCardStyle::Plain => row.rows.len().max(1),
        SidebarCardStyle::Compact => 3,
        SidebarCardStyle::Full => 4,
    }
}

pub(super) fn ordered_agent_pane_ids(
    snapshot: &ClientShellSnapshot,
    sort: crate::config::AgentPanelSortConfig,
) -> Vec<String> {
    if snapshot.agent_view_label.is_some() {
        return snapshot
            .agent_order
            .iter()
            .filter(|pane_id| {
                snapshot
                    .agents
                    .iter()
                    .any(|agent| agent.pane_id == pane_id.as_str())
            })
            .cloned()
            .collect();
    }
    let mut agents = snapshot.agents.iter().collect::<Vec<_>>();
    if sort == crate::config::AgentPanelSortConfig::Priority {
        agents.sort_by_key(|agent| {
            (
                std::cmp::Reverse(status_priority(agent.agent_status)),
                std::cmp::Reverse(agent.state_change_seq),
            )
        });
    }
    agents
        .into_iter()
        .map(|agent| agent.pane_id.clone())
        .collect()
}

pub(super) fn render_agent_panel(
    buffer: &mut Buffer,
    area: Rect,
    snapshot: &ClientShellSnapshot,
    config: &ClientShellConfig,
    agent_scroll: &mut usize,
    hits: &mut ShellHitMap,
) {
    if !render_agent_panel_header(
        buffer,
        area,
        snapshot.agent_view_label.as_deref(),
        config,
        hits,
    ) {
        return;
    }

    let rows = agent_rows(snapshot, config, None);
    render_agent_list(
        buffer,
        area,
        &rows,
        snapshot
            .agent_view_label
            .as_ref()
            .map(|_| " no matching agents"),
        config,
        agent_scroll,
        hits,
        |row| agent_row_lines(row, config.agents.card_style),
        |buffer, rect, row, hits| {
            hits.agents.push((rect, row.pane_id.clone()));
            render_agent_row(buffer, rect, row, config, hits);
        },
    );
}

pub(super) fn render_agent_panel_header(
    buffer: &mut Buffer,
    area: Rect,
    agent_view_label: Option<&str>,
    config: &ClientShellConfig,
    hits: &mut ShellHitMap,
) -> bool {
    if area.height == 0 {
        return false;
    }
    put_text(
        buffer,
        area.x,
        area.y,
        area.width,
        &"─".repeat(area.width as usize),
        Style::default().fg(config.palette.surface_dim),
    );
    if area.height < 2 {
        return false;
    }
    put_text(
        buffer,
        area.x,
        area.y + 1,
        area.width,
        " agents",
        Style::default()
            .fg(config.palette.overlay0)
            .add_modifier(Modifier::BOLD),
    );
    let sort_label = agent_view_label.unwrap_or(match config.agent_panel_sort {
        crate::config::AgentPanelSortConfig::Spaces => "grouped",
        crate::config::AgentPanelSortConfig::Priority => "priority",
    });
    let sort_width = display_width(sort_label).min(area.width as usize) as u16;
    let sort_rect = Rect::new(
        area.right().saturating_sub(sort_width),
        area.y + 1,
        sort_width,
        1,
    );
    hits.agent_sort_toggle = if config.mouse_capture && agent_view_label.is_none() {
        sort_rect
    } else {
        Rect::default()
    };
    put_text(
        buffer,
        sort_rect.x,
        sort_rect.y,
        sort_rect.width,
        sort_label,
        Style::default()
            .fg(if agent_view_label.is_some() {
                config.palette.accent
            } else {
                config.palette.overlay0
            })
            .add_modifier(Modifier::BOLD),
    );
    true
}

pub(super) fn render_agent_list<T>(
    buffer: &mut Buffer,
    area: Rect,
    rows: &[T],
    empty_message: Option<&str>,
    config: &ClientShellConfig,
    agent_scroll: &mut usize,
    hits: &mut ShellHitMap,
    row_lines: impl Fn(&T) -> usize,
    mut render_row: impl FnMut(&mut Buffer, Rect, &T, &mut ShellHitMap),
) {
    let body = Rect::new(
        area.x,
        area.y.saturating_add(3),
        area.width,
        area.height.saturating_sub(3),
    );
    hits.agent_body = body;
    if body.is_empty() || rows.is_empty() {
        *agent_scroll = 0;
        if let Some(message) = empty_message.filter(|_| !body.is_empty()) {
            put_text(
                buffer,
                body.x,
                body.y,
                body.width,
                message,
                Style::default()
                    .fg(config.palette.overlay0)
                    .add_modifier(Modifier::DIM),
            );
        }
        return;
    }

    let row_heights = rows
        .iter()
        .map(|row| row_lines(row).max(1).min(u16::MAX as usize) as u16)
        .collect::<Vec<_>>();
    let gaps = rows
        .iter()
        .enumerate()
        .map(|(index, _)| {
            if index + 1 < rows.len() {
                config.agents.row_gap
            } else {
                0
            }
        })
        .collect::<Vec<_>>();
    let metrics =
        super::scroll::list_scroll_metrics(&row_heights, &gaps, body.height, *agent_scroll);
    hits.agent_max_scroll = metrics.max_offset_from_bottom;
    hits.agent_scroll_metrics = Some(metrics);
    *agent_scroll = metrics
        .max_offset_from_bottom
        .saturating_sub(metrics.offset_from_bottom);
    let show_scrollbar = metrics.max_offset_from_bottom > 0 && body.width > 1;
    let content_width = body.width.saturating_sub(u16::from(show_scrollbar));
    let mut y = body.y;
    for (index, row) in rows.iter().enumerate().skip(*agent_scroll) {
        let height = row_heights[index].min(body.height);
        if y.saturating_add(height) > body.bottom() {
            break;
        }
        let rect = Rect::new(body.x, y, content_width, height);
        render_row(buffer, rect, row, hits);
        y = y
            .saturating_add(height)
            .saturating_add(if index + 1 < rows.len() {
                config.agents.row_gap
            } else {
                0
            });
    }

    if show_scrollbar {
        let track = Rect::new(body.right().saturating_sub(1), body.y, 1, body.height);
        hits.agent_scrollbar = track;
        super::scroll::render_list_scrollbar(buffer, track, metrics, &config.palette);
    }
}

pub(super) fn agent_rows(
    snapshot: &ClientShellSnapshot,
    config: &ClientShellConfig,
    machine: Option<&str>,
) -> Vec<AgentRow> {
    ordered_agent_pane_ids(snapshot, config.agent_panel_sort)
        .into_iter()
        .filter_map(|pane_id| agent_row(snapshot, &pane_id, config, machine))
        .collect()
}

pub(super) fn agent_row(
    snapshot: &ClientShellSnapshot,
    pane_id: &str,
    config: &ClientShellConfig,
    machine: Option<&str>,
) -> Option<AgentRow> {
    let agent = snapshot
        .agents
        .iter()
        .find(|agent| agent.pane_id == pane_id)?;
    let workspace = snapshot
        .workspaces
        .iter()
        .find(|workspace| workspace.workspace_id == agent.workspace_id)?;
    let tab = snapshot.tabs.iter().find(|tab| tab.tab_id == agent.tab_id);
    let pane = snapshot
        .panes
        .iter()
        .find(|pane| pane.pane_id == agent.pane_id);
    let tab_count = snapshot
        .tabs
        .iter()
        .filter(|candidate| candidate.workspace_id == agent.workspace_id)
        .count();
    let tab_label = tab
        .filter(|tab| tab_count > 1 || tab.custom_label)
        .map(|tab| tab.label.as_str());
    let agent_label = agent
        .display_agent
        .as_deref()
        .or(agent.name.as_deref())
        .or(agent.agent.as_deref())
        .or(agent.title.as_deref());
    let labels = agent
        .state_labels
        .iter()
        .cloned()
        .collect::<HashMap<_, _>>();
    let tokens = agent.tokens.iter().cloned().collect::<HashMap<_, _>>();
    let state_text = labels
        .get(status_text(agent.agent_status))
        .map(String::as_str)
        .unwrap_or_else(|| sidebar_status_text(agent.agent_status));
    let canonical_agent = agent
        .agent
        .as_deref()
        .and_then(crate::detect::parse_agent_label);
    let pane_label = agent
        .title
        .as_deref()
        .or_else(|| pane.and_then(|pane| pane.label.as_deref()));
    let card = AgentCardContent {
        label: agent_label.or(pane_label).unwrap_or("agent").to_owned(),
        space: workspace.label.clone(),
        machine: machine.map(str::to_owned),
        tab: tab_label.map(str::to_owned),
    };
    let rows = crate::ui::sidebar_agent_rows(
        &config.agents,
        crate::ui::AgentTokenContext {
            machine,
            workspace: &workspace.label,
            tab: tab_label,
            pane: pane_label,
            agent_label,
            terminal_title: agent.terminal_title.as_deref(),
            terminal_title_stripped: agent.terminal_title_stripped.as_deref(),
            canonical_agent,
            tokens: &tokens,
        },
        state_text,
    );
    Some(AgentRow {
        pane_id: agent.pane_id.clone(),
        status: agent.agent_status,
        focused: agent.focused,
        rows,
        card,
    })
}

pub(super) fn render_agent_row(
    buffer: &mut Buffer,
    rect: Rect,
    row: &AgentRow,
    config: &ClientShellConfig,
    hits: &mut ShellHitMap,
) {
    match config.agents.card_style {
        SidebarCardStyle::Plain => render_plain_agent_row(buffer, rect, row, config, hits),
        style => render_agent_card(buffer, rect, row, config, style, hits),
    }
}

fn render_plain_agent_row(
    buffer: &mut Buffer,
    rect: Rect,
    row: &AgentRow,
    config: &ClientShellConfig,
    hits: &mut ShellHitMap,
) {
    let palette = &config.palette;
    let row_style = if row.focused {
        Style::default().bg(palette.active_row_bg)
    } else {
        Style::default()
    };
    let name_style = if row.focused {
        Style::default()
            .fg(palette.text)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(palette.subtext0)
            .add_modifier(Modifier::BOLD)
    };
    let status_style = Style::default().fg(status_color(row.status, palette));
    let secondary = Style::default().fg(palette.overlay0);
    let icon = (
        super::agent_animation::indicator_icon(row.status, config.status_indicators),
        Style::default().fg(status_color(row.status, palette)),
    );
    let rows = if row.rows.is_empty() {
        vec![vec![crate::ui::ResolvedToken {
            kind: crate::ui::ResolvedTokenKind::StateIcon,
            style: Default::default(),
        }]]
    } else {
        row.rows.clone()
    };
    for (index, tokens) in rows.iter().take(rect.height as usize).enumerate() {
        let indent = if index == 0 { 1 } else { 3 };
        let badge = (rect.width >= 20
            && tokens.len() == 1
            && matches!(tokens[0].kind, crate::ui::ResolvedTokenKind::Agent(_)))
        .then_some(status_badge_text(row.status));
        // One cell of gap before the status text and one cell of right margin.
        let badge_width = badge.map_or(0, |text| text.len() as u16 + 2);
        let y = rect.y + index as u16;
        let mut spans = vec![ratatui::text::Span::raw(" ".repeat(indent))];
        spans.extend(crate::ui::resolved_token_spans(
            tokens,
            icon,
            status_style,
            name_style,
            secondary,
            secondary,
            palette,
            rect.width.saturating_sub(indent as u16 + badge_width) as usize,
            |offset| {
                let x = rect
                    .x
                    .saturating_add((indent + offset).min(u16::MAX as usize) as u16);
                if x < rect.right() {
                    hits.agent_indicators
                        .push(super::agent_animation::AgentIndicator {
                            rect: Rect::new(x, y, 1, 1),
                            status: row.status,
                            spinner: true,
                        });
                }
            },
        ));
        Paragraph::new(Line::from(spans))
            .style(row_style)
            .render(Rect::new(rect.x, y, rect.width, 1), buffer);
        if row.focused {
            buffer[(rect.x, y)]
                .set_symbol("▎")
                .set_style(Style::default().fg(palette.accent));
        }
        if let Some(badge) = badge {
            let width = badge.len() as u16;
            let badge_rect = Rect::new(rect.right().saturating_sub(width + 1), y, width, 1);
            buffer.set_stringn(badge_rect.x, y, badge, width as usize, status_style);
            hits.agent_indicators
                .push(super::agent_animation::AgentIndicator {
                    rect: badge_rect,
                    status: row.status,
                    spinner: false,
                });
        }
    }
}

/// compact 卡片把 space 名画在上边框，内容保持一行（`icon name · 机器/标签页  state`）；
/// full 卡片把位置（`machine · space · tab`）放到单独一行。
pub(super) fn render_agent_card(
    buffer: &mut Buffer,
    rect: Rect,
    row: &AgentRow,
    config: &ClientShellConfig,
    style: SidebarCardStyle,
    hits: &mut ShellHitMap,
) {
    if !super::sidebar_card::card_fits(rect) {
        return;
    }
    let palette = &config.palette;
    let border = if row.focused {
        palette.accent
    } else {
        palette.overlay0
    };
    super::sidebar_card::draw_card_border(buffer, rect, border);
    if style == SidebarCardStyle::Compact {
        super::sidebar_card::draw_card_border_title(buffer, rect, &row.card.space, border);
    }

    let name_style = Style::default()
        .fg(if row.focused {
            palette.text
        } else {
            palette.subtext0
        })
        .add_modifier(Modifier::BOLD);
    let secondary_style = Style::default().fg(if row.focused {
        palette.mauve
    } else {
        palette.overlay0
    });
    let status_style = Style::default().fg(status_color(row.status, palette));
    let inner_left = rect.x + 2;
    let inner_right = rect.right().saturating_sub(2);
    let y = rect.y + 1;

    let state_text = status_badge_text(row.status);
    let state_width = display_width(state_text) as u16;
    let show_state = inner_right.saturating_sub(inner_left) >= state_width + MIN_CARD_NAME_WIDTH;
    let text_right = if show_state {
        inner_right.saturating_sub(state_width + 1)
    } else {
        inner_right
    };

    let icon = super::agent_animation::indicator_icon(row.status, config.status_indicators);
    let mut x = super::render::put_segment(buffer, inner_left, y, text_right, icon, status_style);
    if x > inner_left {
        hits.agent_indicators
            .push(super::agent_animation::AgentIndicator {
                rect: Rect::new(inner_left, y, 1, 1),
                status: row.status,
                spinner: true,
            });
    }
    x = super::render::put_segment(buffer, x, y, text_right, " ", name_style);
    x = super::render::put_segment(buffer, x, y, text_right, &row.card.label, name_style);
    if style == SidebarCardStyle::Compact {
        let context = row.card.context();
        if !context.is_empty() {
            x = super::render::put_segment(
                buffer,
                x,
                y,
                text_right,
                " · ",
                Style::default().fg(palette.overlay0),
            );
            super::render::put_segment(buffer, x, y, text_right, &context, secondary_style);
        }
    }
    if show_state {
        let state_rect = Rect::new(inner_right - state_width, y, state_width, 1);
        super::render::put_segment(
            buffer,
            state_rect.x,
            y,
            inner_right,
            state_text,
            status_style,
        );
        hits.agent_indicators
            .push(super::agent_animation::AgentIndicator {
                rect: state_rect,
                status: row.status,
                spinner: false,
            });
    }
    if style == SidebarCardStyle::Full && rect.height >= 4 {
        let location = row.card.location();
        super::render::put_segment(
            buffer,
            inner_left,
            y + 1,
            inner_right,
            &location,
            secondary_style,
        );
    }
}

/// Below this many cells left for icon and name, the card drops the status text.
const MIN_CARD_NAME_WIDTH: u16 = 8;

fn put_text(buffer: &mut Buffer, x: u16, y: u16, width: u16, text: &str, style: Style) {
    for (offset, character) in text.chars().take(width as usize).enumerate() {
        if let Some(cell) = buffer.cell_mut((x + offset as u16, y)) {
            cell.set_char(character).set_style(style);
        }
    }
}

fn display_width(text: &str) -> usize {
    unicode_width::UnicodeWidthStr::width(text)
}

pub(super) fn status_badge_text(status: crate::api::schema::AgentStatus) -> &'static str {
    use crate::api::schema::AgentStatus;
    match status {
        AgentStatus::Working => "running",
        AgentStatus::Blocked => "input",
        AgentStatus::Done => "done",
        AgentStatus::Idle => "idle",
        AgentStatus::Unknown => "unknown",
    }
}

fn sidebar_status_text(status: crate::api::schema::AgentStatus) -> &'static str {
    use crate::api::schema::AgentStatus;
    match status {
        AgentStatus::Blocked => "blocked",
        AgentStatus::Done => "done",
        AgentStatus::Working => "working",
        AgentStatus::Idle => "idle",
        AgentStatus::Unknown => "unknown",
    }
}
