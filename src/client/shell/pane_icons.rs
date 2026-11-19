// 窗格边框标题里的智能体图标（agent icon）：在服务端画好的 " 名称  状态 " 标题中，
// 于名称前插入正在运行的智能体图标。图标能否显示取决于宿主终端，只有客户端知道，
// 所以在客户端合成帧时改写边框行，服务端渲染的窗格表面保持与终端无关。
use super::*;
use crate::client::agent_icons::{AgentIconImage, AGENT_ICON_COLUMNS};

/// 服务端把标题画在窗格左上角后第 3 格，以一个空格开头。
const TITLE_OFFSET: u16 = 3;
/// 插入的宽度：图标加其后的一个空格。
const INSERT_WIDTH: u16 = AGENT_ICON_COLUMNS + 1;

fn horizontal_border(symbol: &str) -> bool {
    matches!(symbol, "─" | "┬" | "┴" | "┼")
}

pub(super) fn render_pane_title_icons(
    frame: &mut FrameData,
    hits: &mut ShellHitMap,
    snapshot: &ClientShellSnapshot,
    config: &ClientShellConfig,
) {
    if !config.agent_icons_active() {
        return;
    }
    let tone = crate::client::agent_icons::tone_for_palette(&config.palette);
    for index in 0..hits.panes.len() {
        let hit = &hits.panes[index];
        let Some(image) = snapshot
            .agents
            .iter()
            .find(|agent| agent.pane_id == hit.pane_id)
            .and_then(|agent| agent.agent.as_deref())
            .and_then(crate::detect::parse_agent_label)
            .and_then(|agent| AgentIconImage::for_agent(agent, tone))
        else {
            continue;
        };
        // 命中表按从上到下排列，排在前面的浮动窗格会盖住本窗格的边框。
        if insert_title_icon(frame, hit, &hits.panes[..index], image)
            && !hits.agent_icon_images.contains(&image)
        {
            hits.agent_icon_images.push(image);
        }
    }
}

/// 把标题从名称起右移 `INSERT_WIDTH` 格，并在空出的位置写入占位符。
/// 标题后至少要有 `INSERT_WIDTH` 格空边框可让出，否则不画，避免截断名称、状态或浮动窗格的 pin 按钮。
fn insert_title_icon(
    frame: &mut FrameData,
    hit: &PaneHit,
    covering: &[PaneHit],
    image: AgentIconImage,
) -> bool {
    let rect = hit.rect;
    let y = rect.y;
    if hit.inner_rect.y <= y || y >= frame.height {
        return false;
    }
    let right = rect.right().min(frame.width);
    let width = usize::from(frame.width);
    let row = usize::from(y) * width;
    let symbol = |x: u16| frame.cells[row + usize::from(x)].symbol.as_str();
    let title_start = rect.x.saturating_add(TITLE_OFFSET);
    if title_start.saturating_add(1) >= right
        || !(rect.x + 1..title_start).all(|x| horizontal_border(symbol(x)))
        || symbol(title_start) != " "
    {
        return false;
    }
    let Some(title_end) = (title_start + 1..right).find(|x| horizontal_border(symbol(*x))) else {
        return false;
    };
    let shifted_end = title_end.saturating_add(INSERT_WIDTH);
    if shifted_end >= right || !(title_end..shifted_end).all(|x| horizontal_border(symbol(x))) {
        return false;
    }
    if covering
        .iter()
        .any(|cover| (title_start..shifted_end).any(|x| contains(cover.rect, (x, y))))
    {
        return false;
    }

    let name_start = row + usize::from(title_start) + 1;
    frame.cells[name_start..row + usize::from(shifted_end)].rotate_right(usize::from(INSERT_WIDTH));
    let separator = frame.cells[row + usize::from(title_start)].clone();
    let foreground = crate::protocol::color_to_u32(image.foreground());
    for (offset, symbol) in image.placeholder_cells().into_iter().enumerate() {
        let cell = &mut frame.cells[name_start + offset];
        *cell = separator.clone();
        cell.symbol = symbol;
        cell.fg = foreground;
        cell.modifier = 0;
    }
    frame.cells[name_start + usize::from(AGENT_ICON_COLUMNS)] = separator;
    true
}
