use ratatui::{buffer::Buffer, layout::Rect};

use super::text::display_width;

/// Blanks every wide character that straddles a vertical edge of `rect`.
/// Call it before drawing an overlay into `rect`. Buffer diffs and frame
/// encoders skip the columns after a wide lead, so a lead left outside the
/// overlay would hide the overlay's edge glyph and keep painting across it.
pub(crate) fn split_wide_cells_at_vertical_edges(buf: &mut Buffer, rect: Rect) {
    let rect = rect.intersection(buf.area);
    for y in rect.top()..rect.bottom() {
        split_wide_cell_at(buf, rect.left(), y);
        split_wide_cell_at(buf, rect.right(), y);
    }
}

/// Blanks the wide character whose columns include both `edge - 1` and `edge`.
fn split_wide_cell_at(buf: &mut Buffer, edge: u16, y: u16) {
    let area = buf.area;
    if edge <= area.left() || edge >= area.right() {
        return;
    }
    // Spacer tails are empty cells, so the lead is the first non-empty cell
    // at or before `edge - 1`.
    let mut lead = edge - 1;
    while lead > area.left() && buf[(lead, y)].symbol().is_empty() {
        lead -= 1;
    }
    let span_end = usize::from(lead) + display_width(buf[(lead, y)].symbol());
    if span_end <= usize::from(edge) {
        return;
    }
    let span_end = u16::try_from(span_end).map_or(area.right(), |end| end.min(area.right()));
    for x in lead..span_end {
        buf[(x, y)].set_symbol(" ");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(cells: &[&str]) -> Buffer {
        let mut buf = Buffer::empty(Rect::new(0, 0, cells.len() as u16, 1));
        for (x, symbol) in cells.iter().enumerate() {
            buf[(x as u16, 0)].set_symbol(symbol);
        }
        buf
    }

    fn symbols(buf: &Buffer) -> Vec<&str> {
        buf.content().iter().map(|cell| cell.symbol()).collect()
    }

    #[test]
    fn blanks_a_wide_lead_whose_tail_enters_the_rect() {
        let mut buf = row(&["a", "果", "", "b", "c"]);
        split_wide_cells_at_vertical_edges(&mut buf, Rect::new(2, 0, 2, 1));
        assert_eq!(symbols(&buf), ["a", " ", " ", "b", "c"]);
    }

    #[test]
    fn blanks_a_tail_left_outside_the_rect() {
        let mut buf = row(&["a", "b", "果", "", "c"]);
        split_wide_cells_at_vertical_edges(&mut buf, Rect::new(1, 0, 2, 1));
        assert_eq!(symbols(&buf), ["a", "b", " ", " ", "c"]);
    }

    #[test]
    fn keeps_wide_characters_that_do_not_cross_an_edge() {
        let mut buf = row(&["果", "", "a", "果", "", "b"]);
        split_wide_cells_at_vertical_edges(&mut buf, Rect::new(2, 0, 1, 1));
        assert_eq!(symbols(&buf), ["果", "", "a", "果", "", "b"]);
    }
}
