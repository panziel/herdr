//! 自动拆分（new pane）：参照 zellij 的 new pane，沿聚焦窗格在屏幕上较长的一边拆分，
//! 宽大于高时左右并排，否则上下堆叠。

use super::state::PaneHit;
use crate::api::schema::SplitDirection;

/// Terminal cells are usually about twice as tall as they are wide. Used when the host
/// terminal has not reported its cell pixel size.
const DEFAULT_CELL_HEIGHT_TO_WIDTH: u64 = 2;

/// Compares on-screen height and width instead of cell counts, because a cell is taller
/// than it is wide. Ties split side by side, matching zellij.
pub(super) fn auto_split_direction(hit: &PaneHit) -> SplitDirection {
    let rows = u64::from(hit.rect.height);
    let cols = u64::from(hit.rect.width);
    let inner_rows = u64::from(hit.inner_rect.height);
    let inner_cols = u64::from(hit.inner_rect.width);
    let pixel_width = u64::from(hit.pixel_width);
    let pixel_height = u64::from(hit.pixel_height);

    // pixel_width/pixel_height describe the inner rect, so the cell aspect ratio is
    // (pixel_height / inner_rows) / (pixel_width / inner_cols). Cross-multiplied to stay in
    // integers.
    let (visual_height, visual_width) =
        if pixel_width > 0 && pixel_height > 0 && inner_rows > 0 && inner_cols > 0 {
            (
                rows * pixel_height * inner_cols,
                cols * pixel_width * inner_rows,
            )
        } else {
            (rows * DEFAULT_CELL_HEIGHT_TO_WIDTH, cols)
        };

    if visual_height > visual_width {
        SplitDirection::Down
    } else {
        SplitDirection::Right
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    fn hit(width: u16, height: u16, pixel_width: u32, pixel_height: u32) -> PaneHit {
        PaneHit {
            rect: Rect::new(0, 0, width, height),
            inner_rect: Rect::new(1, 1, width - 2, height - 2),
            scrollbar_rect: None,
            scroll: None,
            pane_id: "pane_1".into(),
            popup: false,
            floating: None,
            mouse_reporting: false,
            sgr_pixel_mouse: false,
            pixel_width,
            pixel_height,
        }
    }

    #[test]
    fn wide_pane_splits_side_by_side_without_pixel_size() {
        assert_eq!(
            auto_split_direction(&hit(80, 30, 0, 0)),
            SplitDirection::Right
        );
    }

    #[test]
    fn tall_pane_stacks_without_pixel_size() {
        assert_eq!(
            auto_split_direction(&hit(40, 30, 0, 0)),
            SplitDirection::Down
        );
    }

    #[test]
    fn equal_visual_size_splits_side_by_side() {
        assert_eq!(
            auto_split_direction(&hit(60, 30, 0, 0)),
            SplitDirection::Right
        );
    }

    #[test]
    fn reported_cell_size_overrides_the_default_ratio() {
        // 3:1 cells make a 50x20 pane visually 60 tall by 50 wide.
        let (inner_cols, inner_rows) = (48, 18);
        let tall_cells = hit(50, 20, inner_cols * 10, inner_rows * 30);
        assert_eq!(auto_split_direction(&tall_cells), SplitDirection::Down);

        // Square cells make the same pane clearly wider than tall.
        let square_cells = hit(50, 20, inner_cols * 10, inner_rows * 10);
        assert_eq!(auto_split_direction(&square_cells), SplitDirection::Right);
    }
}
