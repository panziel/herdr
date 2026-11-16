// 侧栏圆角卡片（card_style 的 compact/full）的边框绘制，供 Spaces 与 Agents 列表共用。
use super::render::put_text;
use super::*;

pub(super) const MIN_CARD_WIDTH: u16 = 4;
pub(super) const MIN_CARD_HEIGHT: u16 = 3;

pub(super) fn is_card_border_symbol(symbol: &str) -> bool {
    matches!(symbol, "│" | "─" | "╭" | "╮" | "╰" | "╯")
}

pub(super) fn card_fits(area: Rect) -> bool {
    area.width >= MIN_CARD_WIDTH && area.height >= MIN_CARD_HEIGHT
}

pub(super) fn draw_card_border(buffer: &mut Buffer, area: Rect, color: ratatui::style::Color) {
    if !card_fits(area) {
        return;
    }
    let style = Style::default().fg(color);
    let right = area.right() - 1;
    let last = area.bottom() - 1;
    put_text(buffer, area.x, area.y, 1, "╭", style);
    put_text(buffer, right, area.y, 1, "╮", style);
    put_text(buffer, area.x, last, 1, "╰", style);
    put_text(buffer, right, last, 1, "╯", style);
    for x in area.x + 1..right {
        put_text(buffer, x, area.y, 1, "─", style);
        put_text(buffer, x, last, 1, "─", style);
    }
    for y in area.y + 1..last {
        put_text(buffer, area.x, y, 1, "│", style);
        put_text(buffer, right, y, 1, "│", style);
    }
}

/// 标题压在上边框上，与窗格边框标题同一套排版：距左角 3 格起笔，
/// 文本自带首尾空格（`╭── label ──╮`），超长按显示宽度截断出 `…`。
pub(super) fn draw_card_border_title(
    buffer: &mut Buffer,
    area: Rect,
    label: &str,
    color: ratatui::style::Color,
) {
    if !card_fits(area) || label.is_empty() {
        return;
    }
    let start = area.x.saturating_add(3);
    let end = area.right().saturating_sub(1);
    let width = end.saturating_sub(start) as usize;
    // 剩不下 " x " 时宁可不画，避免标题贴角或残缺。
    if width < 3 {
        return;
    }
    let text = format!(" {} ", crate::ui::truncate_end(label, width - 2));
    put_text(
        buffer,
        start,
        area.y,
        width as u16,
        &text,
        Style::default().fg(color),
    );
}
