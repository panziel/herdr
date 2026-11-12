//! 浮动窗格（floating pane）：每个 tab 一层悬浮在平铺窗格之上的窗格。
//!
//! 浮动层不会拦截平铺窗格的输入。焦点回到平铺窗格时整层隐藏，只有已固定（pinned）
//! 的浮动窗格继续显示在最上层。位置和大小按 tab 区域的千分比保存，客户端尺寸变化时按比例跟随。

use ratatui::layout::Rect;

use crate::layout::{NavDirection, PaneId};

/// Denominator of [`FloatingRect`] coordinates.
pub(crate) const FLOATING_SCALE: u16 = 1000;
/// Smallest outer size a floating pane resolves to, so its frame, title and
/// pin button always fit around at least a few terminal cells.
pub(crate) const MIN_FLOATING_WIDTH: u16 = 16;
pub(crate) const MIN_FLOATING_HEIGHT: u16 = 5;

const DEFAULT_SIZE: u16 = 600;
const DEFAULT_ORIGIN: u16 = 200;
const CASCADE_STEP: u16 = 30;
const CASCADE_SLOTS: u16 = 6;

/// Geometry of a floating pane in thousandths of its tab area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloatingRect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Default for FloatingRect {
    fn default() -> Self {
        Self::cascaded(0)
    }
}

impl FloatingRect {
    pub(crate) fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        let width = width.clamp(1, FLOATING_SCALE);
        let height = height.clamp(1, FLOATING_SCALE);
        Self {
            x: x.min(FLOATING_SCALE - width),
            y: y.min(FLOATING_SCALE - height),
            width,
            height,
        }
    }

    /// Default rect for the `index`-th floating pane, offset so stacked panes
    /// do not hide each other's frames.
    pub(crate) fn cascaded(index: usize) -> Self {
        let slot = u16::try_from(index % usize::from(CASCADE_SLOTS)).unwrap_or(0);
        let offset = slot * CASCADE_STEP;
        Self::new(
            DEFAULT_ORIGIN + offset,
            DEFAULT_ORIGIN + offset,
            DEFAULT_SIZE,
            DEFAULT_SIZE,
        )
    }

    /// Cell rect inside `area`. The result never leaves `area` and respects the
    /// minimum floating size whenever `area` is large enough to hold it.
    pub(crate) fn resolve(self, area: Rect) -> Rect {
        let scale = |length: u16, permille: u16| -> u16 {
            let scaled = (u32::from(length) * u32::from(permille) + u32::from(FLOATING_SCALE / 2))
                / u32::from(FLOATING_SCALE);
            u16::try_from(scaled).unwrap_or(u16::MAX)
        };
        let width = scale(area.width, self.width)
            .max(MIN_FLOATING_WIDTH)
            .min(area.width);
        let height = scale(area.height, self.height)
            .max(MIN_FLOATING_HEIGHT)
            .min(area.height);
        let x = scale(area.width, self.x).min(area.width - width);
        let y = scale(area.height, self.y).min(area.height - height);
        Rect::new(area.x + x, area.y + y, width, height)
    }

    /// Inverse of [`Self::resolve`] for a cell rect drawn inside `area`.
    pub(crate) fn from_cells(rect: Rect, area: Rect) -> Self {
        let rect = rect.intersection(area);
        let ratio = |offset: u16, length: u16| -> u16 {
            if length == 0 {
                return 0;
            }
            let permille = (u32::from(offset) * u32::from(FLOATING_SCALE) + u32::from(length) / 2)
                / u32::from(length);
            u16::try_from(permille).unwrap_or(FLOATING_SCALE)
        };
        Self::new(
            ratio(rect.x.saturating_sub(area.x), area.width),
            ratio(rect.y.saturating_sub(area.y), area.height),
            ratio(rect.width, area.width),
            ratio(rect.height, area.height),
        )
    }

    /// Shift the resolved rect by `cells` toward `direction`, staying inside `area`.
    pub(crate) fn moved(self, direction: NavDirection, cells: u16, area: Rect) -> Self {
        let mut rect = self.resolve(area);
        match direction {
            NavDirection::Left => rect.x = rect.x.saturating_sub(cells).max(area.x),
            NavDirection::Right => {
                rect.x = rect
                    .x
                    .saturating_add(cells)
                    .min(area.right().saturating_sub(rect.width));
            }
            NavDirection::Up => rect.y = rect.y.saturating_sub(cells).max(area.y),
            NavDirection::Down => {
                rect.y = rect
                    .y
                    .saturating_add(cells)
                    .min(area.bottom().saturating_sub(rect.height));
            }
        }
        Self::from_cells(rect, area).with_size_of(self)
    }

    /// Grow (`Right`/`Down`) or shrink (`Left`/`Up`) the right or bottom edge
    /// of the resolved rect by `cells`. A grow that hits the area edge moves the
    /// opposite edge instead, so a pane at the border can still get bigger.
    pub(crate) fn resized(self, direction: NavDirection, cells: u16, area: Rect) -> Self {
        let mut rect = self.resolve(area);
        match direction {
            NavDirection::Left => {
                rect.width = rect.width.saturating_sub(cells).max(MIN_FLOATING_WIDTH);
            }
            NavDirection::Up => {
                rect.height = rect.height.saturating_sub(cells).max(MIN_FLOATING_HEIGHT);
            }
            NavDirection::Right => {
                let width = rect.width.saturating_add(cells).min(area.width);
                let overflow = (rect.x + width).saturating_sub(area.right());
                rect.x = rect.x.saturating_sub(overflow).max(area.x);
                rect.width = width;
            }
            NavDirection::Down => {
                let height = rect.height.saturating_add(cells).min(area.height);
                let overflow = (rect.y + height).saturating_sub(area.bottom());
                rect.y = rect.y.saturating_sub(overflow).max(area.y);
                rect.height = height;
            }
        }
        Self::from_cells(rect, area)
    }

    /// Keep this rect's size and take only the position from a converted rect,
    /// so moving never accumulates rounding drift in the size.
    fn with_size_of(self, original: Self) -> Self {
        Self::new(self.x, self.y, original.width, original.height)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloatingPane {
    pub id: PaneId,
    pub rect: FloatingRect,
    /// Pinned panes stay on top while the floating layer is hidden.
    pub pinned: bool,
}

/// Per-tab stack of floating panes plus whether the layer is shown.
///
/// A shown layer owns keyboard focus through its top pane; hiding the layer
/// hands focus back to the tiled layout. Pinned panes are drawn either way.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FloatingLayer {
    /// Bottom to top. The last entry is drawn above every other pane.
    panes: Vec<FloatingPane>,
    visible: bool,
}

impl FloatingLayer {
    pub(crate) fn from_saved(panes: Vec<FloatingPane>, visible: bool) -> Self {
        let visible = visible && !panes.is_empty();
        Self { panes, visible }
    }

    pub fn is_empty(&self) -> bool {
        self.panes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.panes.len()
    }

    /// Panes from bottom to top.
    pub fn panes(&self) -> &[FloatingPane] {
        &self.panes
    }

    pub fn contains(&self, id: PaneId) -> bool {
        self.panes.iter().any(|pane| pane.id == id)
    }

    pub fn get(&self, id: PaneId) -> Option<&FloatingPane> {
        self.panes.iter().find(|pane| pane.id == id)
    }

    pub fn top(&self) -> Option<&FloatingPane> {
        self.panes.last()
    }

    /// Whether every floating pane is shown and the top one has focus.
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn focused_pane(&self) -> Option<PaneId> {
        self.visible
            .then(|| self.top().map(|pane| pane.id))
            .flatten()
    }

    /// Panes drawn above the tiled layout, bottom to top.
    pub fn rendered_panes(&self) -> impl Iterator<Item = &FloatingPane> {
        self.panes
            .iter()
            .filter(move |pane| self.visible || pane.pinned)
    }

    pub fn is_rendered(&self, id: PaneId) -> bool {
        self.get(id).is_some_and(|pane| self.visible || pane.pinned)
    }

    /// Default rect for the next floating pane.
    pub(crate) fn next_rect(&self) -> FloatingRect {
        FloatingRect::cascaded(self.panes.len())
    }

    /// Add a pane. With `focus` it goes on top and the layer is shown;
    /// otherwise it slides under the focused pane so focus does not move.
    pub(crate) fn insert(&mut self, pane: FloatingPane, focus: bool) {
        self.panes.retain(|existing| existing.id != pane.id);
        if focus {
            self.panes.push(pane);
            self.visible = true;
        } else if self.visible {
            let below_top = self.panes.len().saturating_sub(1);
            self.panes.insert(below_top, pane);
        } else {
            self.panes.push(pane);
        }
    }

    pub(crate) fn remove(&mut self, id: PaneId) -> Option<FloatingPane> {
        let index = self.panes.iter().position(|pane| pane.id == id)?;
        let removed = self.panes.remove(index);
        self.visible &= !self.panes.is_empty();
        Some(removed)
    }

    /// Raise `id` to the top and show the layer with `id` focused.
    pub(crate) fn focus(&mut self, id: PaneId) -> bool {
        let Some(index) = self.panes.iter().position(|pane| pane.id == id) else {
            return false;
        };
        let pane = self.panes.remove(index);
        self.panes.push(pane);
        self.visible = true;
        true
    }

    /// Show every floating pane and focus the top one.
    pub(crate) fn show(&mut self) -> bool {
        self.visible = !self.panes.is_empty();
        self.visible
    }

    /// Hide unpinned panes and return keyboard focus to the tiled layout.
    pub(crate) fn hide(&mut self) {
        self.visible = false;
    }

    pub(crate) fn set_pinned(&mut self, id: PaneId, pinned: bool) -> bool {
        match self.panes.iter_mut().find(|pane| pane.id == id) {
            Some(pane) if pane.pinned != pinned => {
                pane.pinned = pinned;
                true
            }
            _ => false,
        }
    }

    pub(crate) fn set_rect(&mut self, id: PaneId, rect: FloatingRect) -> bool {
        match self.panes.iter_mut().find(|pane| pane.id == id) {
            Some(pane) if pane.rect != rect => {
                pane.rect = rect;
                true
            }
            _ => false,
        }
    }
}

/// Cells of the clickable pin toggle drawn at the right end of a floating
/// pane's top border, shared by the renderer and the client hit-test.
pub(crate) fn pin_button_rect(outer: Rect, pinned: bool) -> Option<Rect> {
    let width = u16::try_from(pin_button_label(pinned).len()).ok()?;
    if outer.width < width + 4 || outer.height < 2 {
        return None;
    }
    Some(Rect::new(outer.right() - width - 2, outer.y, width, 1))
}

/// ASCII only: ambiguous-width symbols render double-width in CJK terminals
/// and would shift the hit area away from the drawn label.
pub(crate) fn pin_button_label(pinned: bool) -> &'static str {
    if pinned {
        " pin [x] "
    } else {
        " pin [ ] "
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> Rect {
        Rect::new(10, 2, 100, 40)
    }

    fn pane(raw: u32) -> FloatingPane {
        FloatingPane {
            id: PaneId::from_raw(raw),
            rect: FloatingRect::default(),
            pinned: false,
        }
    }

    #[test]
    fn rect_resolves_inside_area_and_round_trips_through_cells() {
        let rect = FloatingRect::new(200, 250, 500, 500);
        let cells = rect.resolve(area());
        assert_eq!(cells, Rect::new(30, 12, 50, 20));
        assert_eq!(FloatingRect::from_cells(cells, area()), rect);
    }

    #[test]
    fn rect_resolution_respects_minimum_size_and_area_bounds() {
        let tiny = FloatingRect::new(990, 990, 1, 1).resolve(area());
        assert_eq!(tiny.width, MIN_FLOATING_WIDTH);
        assert_eq!(tiny.height, MIN_FLOATING_HEIGHT);
        assert!(tiny.right() <= area().right());
        assert!(tiny.bottom() <= area().bottom());

        let small_area = Rect::new(0, 0, 8, 3);
        let clamped = FloatingRect::default().resolve(small_area);
        assert_eq!(clamped, small_area);
    }

    #[test]
    fn cascaded_rects_are_offset_and_wrap() {
        assert_ne!(FloatingRect::cascaded(0), FloatingRect::cascaded(1));
        assert_eq!(
            FloatingRect::cascaded(0),
            FloatingRect::cascaded(usize::from(CASCADE_SLOTS))
        );
        for index in 0..usize::from(CASCADE_SLOTS) {
            let rect = FloatingRect::cascaded(index);
            assert!(rect.x + rect.width <= FLOATING_SCALE);
            assert!(rect.y + rect.height <= FLOATING_SCALE);
        }
    }

    #[test]
    fn move_shifts_by_cells_and_stops_at_area_edges() {
        let rect = FloatingRect::new(200, 250, 500, 500);
        let moved = rect.moved(NavDirection::Right, 5, area());
        assert_eq!(moved.resolve(area()).x, rect.resolve(area()).x + 5);
        assert_eq!(moved.width, rect.width);

        let far_left = rect.moved(NavDirection::Left, 500, area());
        assert_eq!(far_left.resolve(area()).x, area().x);
        let far_down = rect.moved(NavDirection::Down, 500, area());
        assert_eq!(far_down.resolve(area()).bottom(), area().bottom());
    }

    #[test]
    fn resize_changes_far_edges_and_grows_inward_at_the_border() {
        let rect = FloatingRect::new(200, 250, 500, 500);
        let wider = rect
            .resized(NavDirection::Right, 10, area())
            .resolve(area());
        assert_eq!(wider.width, 60);
        assert_eq!(wider.x, 30);

        let narrower = rect
            .resized(NavDirection::Left, 500, area())
            .resolve(area());
        assert_eq!(narrower.width, MIN_FLOATING_WIDTH);

        let at_edge = FloatingRect::new(500, 0, 500, 500);
        let grown = at_edge
            .resized(NavDirection::Right, 10, area())
            .resolve(area());
        assert_eq!(grown.right(), area().right());
        assert_eq!(grown.width, 60);
    }

    #[test]
    fn layer_focus_raises_and_shows() {
        let mut layer = FloatingLayer::default();
        layer.insert(pane(1), true);
        layer.insert(pane(2), true);
        layer.hide();
        assert!(!layer.is_visible());
        assert_eq!(layer.focused_pane(), None);

        assert!(layer.focus(PaneId::from_raw(1)));
        assert!(layer.is_visible());
        assert_eq!(layer.focused_pane(), Some(PaneId::from_raw(1)));
        assert_eq!(layer.top().map(|pane| pane.id), Some(PaneId::from_raw(1)));
    }

    #[test]
    fn unfocused_insert_keeps_the_focused_pane_on_top() {
        let mut layer = FloatingLayer::default();
        layer.insert(pane(1), true);
        layer.insert(pane(2), false);
        assert_eq!(layer.focused_pane(), Some(PaneId::from_raw(1)));
        assert_eq!(layer.len(), 2);

        layer.hide();
        layer.insert(pane(3), false);
        assert!(!layer.is_visible());
        assert_eq!(layer.top().map(|pane| pane.id), Some(PaneId::from_raw(3)));
    }

    #[test]
    fn hidden_layer_renders_only_pinned_panes() {
        let mut layer = FloatingLayer::default();
        layer.insert(pane(1), true);
        layer.insert(pane(2), true);
        assert!(layer.set_pinned(PaneId::from_raw(1), true));
        layer.hide();
        let rendered: Vec<_> = layer.rendered_panes().map(|pane| pane.id.raw()).collect();
        assert_eq!(rendered, vec![1]);
        assert!(layer.is_rendered(PaneId::from_raw(1)));
        assert!(!layer.is_rendered(PaneId::from_raw(2)));

        assert!(layer.show());
        assert_eq!(layer.rendered_panes().count(), 2);
    }

    #[test]
    fn removing_the_last_pane_clears_visibility_and_focus() {
        let mut layer = FloatingLayer::default();
        assert!(!layer.show());
        layer.insert(pane(1), true);
        assert!(layer.remove(PaneId::from_raw(1)).is_some());
        assert!(!layer.is_visible());
        assert_eq!(layer.focused_pane(), None);
        assert!(layer.is_empty());
    }

    #[test]
    fn saved_empty_layer_is_never_visible() {
        assert!(!FloatingLayer::from_saved(Vec::new(), true).is_visible());
        let layer = FloatingLayer::from_saved(vec![pane(1)], true);
        assert_eq!(layer.focused_pane(), Some(PaneId::from_raw(1)));
    }

    #[test]
    fn pin_button_sits_on_the_top_border_inside_the_frame() {
        let outer = Rect::new(5, 3, 40, 10);
        let button = pin_button_rect(outer, false).unwrap();
        assert_eq!(pin_button_rect(outer, true), Some(button));
        assert_eq!(button.y, outer.y);
        assert!(button.x > outer.x && button.right() < outer.right());
        assert!(pin_button_rect(Rect::new(0, 0, 6, 10), false).is_none());
    }
}
