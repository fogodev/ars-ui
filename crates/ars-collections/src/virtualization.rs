use alloc::{collections::BTreeMap, vec::Vec};
use core::{num::NonZeroUsize, ops::Range};

pub use ars_core::{Direction, Orientation};

use crate::key::Key;

/// The number of items to render beyond the visible range on each side.
pub const DEFAULT_OVERSCAN: usize = 5;

/// How item sizes are determined for virtualization math.
#[derive(Clone, Debug, PartialEq)]
pub enum LayoutStrategy {
    /// Every item has the same pixel height.
    FixedHeight {
        /// Pixel height of each item.
        item_height: f64,
    },

    /// Items use measured heights with an estimate for unmeasured rows.
    VariableHeight {
        /// Estimated height used for items without a measurement.
        estimated_item_height: f64,
    },

    /// Items are arranged in a grid with a fixed row height.
    Grid {
        /// Pixel height of each row.
        item_height: f64,
        /// Number of columns in the grid.
        columns: NonZeroUsize,
    },

    /// Equal-sized items in responsive columns.
    GridLayout {
        /// Minimum item width in pixels.
        min_item_width: f64,
        /// Maximum item width in pixels.
        max_item_width: f64,
        /// Minimum item height in pixels.
        min_item_height: f64,
        /// Optional maximum item height in pixels.
        max_item_height: Option<f64>,
        /// Gap between items in pixels.
        gap: f64,
    },

    /// Waterfall or masonry layout using responsive columns.
    WaterfallLayout {
        /// Minimum item width in pixels.
        min_item_width: f64,
        /// Maximum item width in pixels.
        max_item_width: f64,
        /// Minimum item height in pixels.
        min_item_height: f64,
        /// Gap between items in pixels.
        gap: f64,
    },

    /// Table-specific virtualization layout.
    TableLayout {
        /// Estimated height of each data row in pixels.
        row_height: f64,
        /// Height of the sticky header row in pixels.
        header_height: f64,
        /// Width of each visible column in pixels.
        column_widths: Vec<f64>,
        /// Vertical gap between rows in pixels.
        row_gap: f64,
    },
}

impl LayoutStrategy {
    /// Returns the estimated height for a single item in this layout.
    #[must_use]
    pub const fn estimated_item_height(&self) -> f64 {
        match self {
            Self::VariableHeight {
                estimated_item_height,
            } => *estimated_item_height,

            Self::FixedHeight { item_height } | Self::Grid { item_height, .. } => *item_height,

            Self::GridLayout {
                min_item_height, ..
            }
            | Self::WaterfallLayout {
                min_item_height, ..
            } => *min_item_height,
            Self::TableLayout { row_height, .. } => *row_height,
        }
    }

    fn estimated_item_extent(&self, orientation: Orientation) -> f64 {
        match orientation {
            Orientation::Vertical => self.estimated_item_height(),
            Orientation::Horizontal => match self {
                Self::GridLayout { min_item_width, .. }
                | Self::WaterfallLayout { min_item_width, .. } => *min_item_width,
                Self::TableLayout {
                    row_height,
                    column_widths,
                    ..
                } => column_widths.first().copied().unwrap_or(*row_height),
                _ => self.estimated_item_height(),
            },
        }
    }
}

/// Aligns a target item within the viewport during programmatic scrolling.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollAlign {
    /// Scroll only when the item is outside the viewport.
    #[default]
    Auto,

    /// Align the item to the top of the viewport.
    Top,

    /// Align the item to the bottom of the viewport.
    Bottom,

    /// Center the item within the viewport.
    Center,
}

/// Computes visible range and scroll math for a virtualized collection.
#[derive(Clone, Debug, PartialEq)]
pub struct Virtualizer {
    /// Total logical item count in the collection.
    pub total_count: usize,

    /// Layout strategy that drives height estimation.
    pub layout: LayoutStrategy,

    /// Height of the viewport in pixels.
    pub viewport_height: f64,

    /// Width of the viewport in pixels.
    pub viewport_width: f64,

    /// Current vertical scroll position in pixels.
    pub scroll_top: f64,

    /// Current horizontal scroll position in pixels.
    pub scroll_left: f64,

    /// Active scroll orientation.
    pub orientation: Orientation,

    /// Active text direction.
    pub dir: Direction,

    /// Overscan count applied on both sides of the visible range.
    pub overscan: usize,

    /// Currently focused index, if any.
    pub focused_index: Option<usize>,

    measured_heights: BTreeMap<usize, f64>,
}

impl Virtualizer {
    /// Creates a new virtualizer with the given item count and layout strategy.
    #[must_use]
    pub const fn new(total_count: usize, layout: LayoutStrategy) -> Self {
        Self {
            total_count,
            layout,
            viewport_height: 0.0,
            viewport_width: 0.0,
            scroll_top: 0.0,
            scroll_left: 0.0,
            orientation: Orientation::Vertical,
            dir: Direction::Ltr,
            overscan: DEFAULT_OVERSCAN,
            focused_index: None,
            measured_heights: BTreeMap::new(),
        }
    }

    /// Updates the tracked scroll position and viewport dimensions.
    pub const fn set_scroll_state_mut(
        &mut self,
        scroll_top: f64,
        scroll_left: f64,
        viewport_height: f64,
        viewport_width: f64,
    ) {
        self.scroll_top = scroll_top;
        self.scroll_left = scroll_left;
        self.viewport_height = viewport_height;
        self.viewport_width = viewport_width;
    }

    /// Records a measured height for a specific item.
    pub fn report_item_height_mut(&mut self, index: usize, height: f64) {
        self.measured_heights.insert(index, height);
    }

    /// Returns a cloned virtualizer with an updated measured height.
    #[must_use]
    pub fn report_item_height(&self, index: usize, height: f64) -> Self {
        let mut updated = self.clone();

        updated.report_item_height_mut(index, height);

        updated
    }

    /// Returns the rendered range `[start, end)` including overscan.
    #[must_use]
    pub fn visible_range(&self) -> Range<usize> {
        let viewport_extent = self.viewport_extent();

        if self.total_count == 0 || viewport_extent == 0.0 {
            return 0..0;
        }

        let scroll_offset = self.clamped_scroll_offset(viewport_extent);

        let (first_visible, last_visible) = match &self.layout {
            LayoutStrategy::FixedHeight { item_height } => {
                let first = (scroll_offset / item_height).floor() as usize;
                let last = ((scroll_offset + viewport_extent) / item_height).ceil() as usize;

                (first, last)
            }

            LayoutStrategy::VariableHeight {
                estimated_item_height,
            } => self.variable_height_range(*estimated_item_height, scroll_offset, viewport_extent),

            LayoutStrategy::Grid {
                item_height,
                columns,
            } => {
                let row_start = (scroll_offset / item_height).floor() as usize;
                let row_end = ((scroll_offset + viewport_extent) / item_height).ceil() as usize;
                let cols = columns.get();

                (row_start * cols, (row_end * cols).min(self.total_count))
            }

            _ => {
                let estimated = self.layout.estimated_item_extent(self.orientation);
                let first = (scroll_offset / estimated).floor() as usize;
                let last = ((scroll_offset + viewport_extent) / estimated).ceil() as usize;

                (first, last)
            }
        };

        let mut start = first_visible.saturating_sub(self.overscan);

        let mut end = (last_visible + self.overscan).min(self.total_count);

        if let Some(focused_index) = self.focused_index {
            if focused_index < self.total_count {
                start = start.min(focused_index);
                end = end.max(focused_index + 1);
            }
        }

        start..end
    }

    /// Returns a cloned virtualizer with the focused index updated.
    #[must_use]
    pub fn set_focused_index(&self, index: Option<usize>) -> Self {
        Self {
            focused_index: index,
            ..self.clone()
        }
    }

    /// Returns the Y-axis offset for the item at `index`.
    #[must_use]
    pub fn item_offset_px(&self, index: usize) -> f64 {
        match &self.layout {
            LayoutStrategy::FixedHeight { item_height } => index as f64 * item_height,

            LayoutStrategy::VariableHeight {
                estimated_item_height,
            } => (0..index)
                .map(|item_index| {
                    self.measured_heights
                        .get(&item_index)
                        .copied()
                        .unwrap_or(*estimated_item_height)
                })
                .sum(),

            LayoutStrategy::Grid {
                item_height,
                columns,
            } => (index / columns.get()) as f64 * item_height,

            _ => index as f64 * self.layout.estimated_item_extent(self.orientation),
        }
    }

    /// Returns the total scrollable height for the current layout.
    #[must_use]
    pub fn total_height_px(&self) -> f64 {
        match &self.layout {
            LayoutStrategy::FixedHeight { item_height } => self.total_count as f64 * item_height,

            LayoutStrategy::VariableHeight {
                estimated_item_height,
            } => (0..self.total_count)
                .map(|item_index| {
                    self.measured_heights
                        .get(&item_index)
                        .copied()
                        .unwrap_or(*estimated_item_height)
                })
                .sum(),

            LayoutStrategy::Grid {
                item_height,
                columns,
            } => {
                let cols = columns.get();
                let rows = self.total_count.div_ceil(cols);
                rows as f64 * item_height
            }

            _ => self.total_count as f64 * self.layout.estimated_item_height(),
        }
    }

    /// Returns the scroll offset needed to bring `index` into view.
    #[must_use]
    pub fn scroll_top_for_index(&self, index: usize, align: ScrollAlign) -> f64 {
        let offset = self.item_offset_px(index);

        let item_extent = match &self.layout {
            LayoutStrategy::FixedHeight { item_height }
            | LayoutStrategy::Grid { item_height, .. } => *item_height,

            LayoutStrategy::VariableHeight {
                estimated_item_height,
            } => self
                .measured_heights
                .get(&index)
                .copied()
                .unwrap_or(*estimated_item_height),

            _ => self.layout.estimated_item_extent(self.orientation),
        };

        let viewport_extent = self.viewport_extent();
        let scroll_offset = self.scroll_offset();
        let item_end = offset + item_extent;

        match align {
            ScrollAlign::Auto => {
                if offset < scroll_offset {
                    offset
                } else if item_end > scroll_offset + viewport_extent {
                    item_end - viewport_extent
                } else {
                    scroll_offset
                }
            }

            ScrollAlign::Top => offset,

            ScrollAlign::Bottom => (item_end - viewport_extent).max(0.0),

            ScrollAlign::Center => (offset - (viewport_extent - item_extent) / 2.0).max(0.0),
        }
    }

    /// Convenience wrapper around [`Self::scroll_top_for_index`].
    #[must_use]
    pub fn scroll_to_index(&self, index: usize, align: ScrollAlign) -> f64 {
        self.scroll_top_for_index(index, align)
    }

    /// Resolves a key to an index, then scrolls to that item if found.
    #[must_use]
    pub fn scroll_to_key(
        &self,
        key: &Key,
        align: ScrollAlign,
        key_to_index: impl Fn(&Key) -> Option<usize>,
    ) -> Option<f64> {
        key_to_index(key).map(|index| self.scroll_to_index(index, align))
    }

    const fn viewport_extent(&self) -> f64 {
        match self.orientation {
            Orientation::Vertical => self.viewport_height,
            Orientation::Horizontal => self.viewport_width,
        }
    }

    const fn scroll_offset(&self) -> f64 {
        match self.orientation {
            Orientation::Vertical => self.scroll_top,
            Orientation::Horizontal => self.scroll_left,
        }
    }

    fn clamped_scroll_offset(&self, viewport_extent: f64) -> f64 {
        let max_scroll = (self.total_main_axis_extent() - viewport_extent).max(0.0);

        self.scroll_offset().clamp(0.0, max_scroll)
    }

    fn total_main_axis_extent(&self) -> f64 {
        match self.orientation {
            Orientation::Vertical => self.total_height_px(),
            Orientation::Horizontal => match &self.layout {
                LayoutStrategy::GridLayout { .. }
                | LayoutStrategy::WaterfallLayout { .. }
                | LayoutStrategy::TableLayout { .. } => {
                    self.total_count as f64 * self.layout.estimated_item_extent(self.orientation)
                }
                _ => self.total_height_px(),
            },
        }
    }

    fn variable_height_range(
        &self,
        estimated: f64,
        scroll_offset: f64,
        viewport_extent: f64,
    ) -> (usize, usize) {
        let mut cumulative = 0.0_f64;

        let mut first = 0;

        let mut found_first = false;

        let mut last = self.total_count;

        for index in 0..self.total_count {
            let height = self
                .measured_heights
                .get(&index)
                .copied()
                .unwrap_or(estimated);

            if cumulative + height > scroll_offset && !found_first {
                first = index;
                found_first = true;
            }

            cumulative += height;

            if cumulative >= scroll_offset + viewport_extent {
                last = index + 1;

                break;
            }
        }

        (first, last)
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    fn fixed_height_virt() -> Virtualizer {
        let mut virt = Virtualizer::new(100, LayoutStrategy::FixedHeight { item_height: 40.0 });

        virt.viewport_height = 200.0;
        virt.overscan = 3;

        virt
    }

    fn grid_virt() -> Virtualizer {
        let mut virt = Virtualizer::new(
            10,
            LayoutStrategy::Grid {
                item_height: 30.0,
                columns: NonZeroUsize::new(3).expect("non-zero columns"),
            },
        );

        virt.viewport_height = 60.0;
        virt.overscan = 1;

        virt
    }

    #[test]
    fn visible_range_for_fixed_height_items() {
        let virt = fixed_height_virt();

        let range = virt.visible_range();

        assert_eq!(range.start, 0);
        assert_eq!(range.end, 8);
    }

    #[test]
    fn empty_collection_has_empty_visible_range() {
        let virt = Virtualizer::new(0, LayoutStrategy::FixedHeight { item_height: 40.0 });

        assert_eq!(virt.visible_range(), 0..0);
    }

    #[test]
    fn zero_viewport_has_empty_visible_range() {
        let virt = Virtualizer::new(10, LayoutStrategy::FixedHeight { item_height: 40.0 });

        assert_eq!(virt.visible_range(), 0..0);
    }

    #[test]
    fn scroll_to_index_top_returns_correct_offset() {
        let virt = fixed_height_virt();

        assert_eq!(virt.scroll_to_index(10, ScrollAlign::Top), 400.0);
    }

    #[test]
    fn scroll_to_index_bottom_returns_correct_offset() {
        let virt = fixed_height_virt();

        assert_eq!(virt.scroll_to_index(10, ScrollAlign::Bottom), 240.0);
    }

    #[test]
    fn scroll_to_index_center_returns_correct_offset() {
        let virt = fixed_height_virt();

        assert_eq!(virt.scroll_to_index(10, ScrollAlign::Center), 320.0);
    }

    #[test]
    fn scroll_to_index_auto_scrolls_when_item_is_below_viewport() {
        let virt = fixed_height_virt();

        assert_eq!(virt.scroll_to_index(10, ScrollAlign::Auto), 240.0);
    }

    #[test]
    fn scroll_to_index_auto_scrolls_when_item_is_above_viewport() {
        let mut virt = fixed_height_virt();

        virt.set_scroll_state_mut(500.0, 0.0, 200.0, 0.0);

        assert_eq!(virt.scroll_to_index(10, ScrollAlign::Auto), 400.0);
    }

    #[test]
    fn scroll_to_index_auto_keeps_visible_item_stationary() {
        let mut virt = fixed_height_virt();

        virt.set_scroll_state_mut(360.0, 0.0, 200.0, 0.0);

        assert_eq!(virt.scroll_to_index(10, ScrollAlign::Auto), 360.0);
    }

    #[test]
    fn scroll_to_key_delegates_through_closure() {
        let virt = fixed_height_virt();

        let offset = virt.scroll_to_key(&Key::from("item-10"), ScrollAlign::Top, |key| {
            if key == &Key::from("item-10") {
                Some(10)
            } else {
                None
            }
        });

        assert_eq!(offset, Some(400.0));
    }

    #[test]
    fn scroll_to_unknown_key_returns_none() {
        let virt = fixed_height_virt();

        assert_eq!(
            virt.scroll_to_key(&Key::from("missing"), ScrollAlign::Top, |_| None),
            None
        );
    }

    #[test]
    fn report_item_height_mut_updates_variable_height_calculations() {
        let mut virt = Virtualizer::new(
            100,
            LayoutStrategy::VariableHeight {
                estimated_item_height: 40.0,
            },
        );

        virt.viewport_height = 170.0;
        virt.overscan = 3;

        let range_before = virt.visible_range();

        let height_before = virt.total_height_px();

        virt.report_item_height_mut(0, 60.0);

        let range_after = virt.visible_range();

        let height_after = virt.total_height_px();

        assert_ne!(range_before, range_after);
        assert_eq!(height_before + 20.0, height_after);
    }

    #[test]
    fn report_item_height_returns_new_equivalent_state() {
        let mut baseline = Virtualizer::new(
            100,
            LayoutStrategy::VariableHeight {
                estimated_item_height: 40.0,
            },
        );

        baseline.viewport_height = 200.0;

        let updated = baseline.report_item_height(2, 64.0);

        let mut mutated = baseline.clone();

        mutated.report_item_height_mut(2, 64.0);

        assert_eq!(updated, mutated);
        assert_ne!(baseline, updated);
    }

    #[test]
    fn variable_height_affects_item_offset() {
        let mut virt = Virtualizer::new(
            100,
            LayoutStrategy::VariableHeight {
                estimated_item_height: 40.0,
            },
        );

        virt.report_item_height_mut(0, 60.0);

        assert_eq!(virt.item_offset_px(1), 60.0);
    }

    #[test]
    fn variable_height_scroll_top_for_index_uses_measured_height() {
        let mut virt = Virtualizer::new(
            10,
            LayoutStrategy::VariableHeight {
                estimated_item_height: 40.0,
            },
        );

        virt.viewport_height = 100.0;
        virt.report_item_height_mut(0, 60.0);
        virt.report_item_height_mut(1, 50.0);

        assert_eq!(virt.scroll_to_index(1, ScrollAlign::Bottom), 10.0);
    }

    #[test]
    fn variable_height_visible_range_can_extend_to_collection_end() {
        let mut virt = Virtualizer::new(
            4,
            LayoutStrategy::VariableHeight {
                estimated_item_height: 40.0,
            },
        );

        virt.set_scroll_state_mut(70.0, 0.0, 100.0, 0.0);
        virt.overscan = 0;
        virt.report_item_height_mut(0, 60.0);

        assert_eq!(virt.visible_range(), 1..4);
    }

    #[test]
    fn variable_height_visible_range_clamps_when_scrolled_past_end() {
        let mut virt = Virtualizer::new(
            4,
            LayoutStrategy::VariableHeight {
                estimated_item_height: 40.0,
            },
        );

        virt.set_scroll_state_mut(500.0, 0.0, 50.0, 0.0);
        virt.overscan = 0;

        assert_eq!(virt.visible_range(), 2..4);
    }

    #[test]
    fn focused_index_before_range_expands_start() {
        let mut virt = fixed_height_virt();

        virt.set_scroll_state_mut(200.0, 0.0, 200.0, 0.0);

        let range = virt.set_focused_index(Some(1)).visible_range();

        assert_eq!(range.start, 1);
    }

    #[test]
    fn focused_index_after_range_expands_end() {
        let mut virt = fixed_height_virt();

        virt.set_scroll_state_mut(0.0, 0.0, 200.0, 0.0);

        let range = virt.set_focused_index(Some(20)).visible_range();

        assert_eq!(range.end, 21);
    }

    #[test]
    fn out_of_bounds_focused_index_is_ignored() {
        let mut virt = fixed_height_virt();

        virt.set_scroll_state_mut(0.0, 0.0, 200.0, 0.0);

        assert_eq!(virt.set_focused_index(Some(200)).visible_range(), 0..8);
    }

    #[test]
    fn grid_layout_computes_visible_range_offsets_and_total_height() {
        let mut virt = grid_virt();

        virt.set_scroll_state_mut(30.0, 0.0, 60.0, 0.0);

        assert_eq!(virt.visible_range(), 2..10);
        assert_eq!(virt.item_offset_px(7), 60.0);
        assert_eq!(virt.total_height_px(), 120.0);
        assert_eq!(virt.scroll_to_index(7, ScrollAlign::Top), 60.0);
    }

    #[test]
    fn horizontal_visible_range_uses_scroll_left_and_viewport_width() {
        let mut virt = fixed_height_virt();

        virt.orientation = Orientation::Horizontal;
        virt.overscan = 0;
        virt.set_scroll_state_mut(400.0, 120.0, 40.0, 80.0);

        assert_eq!(virt.visible_range(), 3..5);
    }

    #[test]
    fn horizontal_scroll_to_index_uses_scroll_left_and_viewport_width() {
        let mut virt = fixed_height_virt();

        virt.orientation = Orientation::Horizontal;
        virt.set_scroll_state_mut(400.0, 80.0, 40.0, 80.0);

        assert_eq!(virt.scroll_to_index(4, ScrollAlign::Auto), 120.0);
        assert_eq!(virt.scroll_to_index(4, ScrollAlign::Bottom), 120.0);
        assert_eq!(virt.scroll_to_index(4, ScrollAlign::Center), 140.0);
    }

    #[test]
    fn fixed_height_visible_range_clamps_when_scrolled_past_end() {
        let mut virt = Virtualizer::new(4, LayoutStrategy::FixedHeight { item_height: 40.0 });

        virt.set_scroll_state_mut(500.0, 0.0, 50.0, 0.0);
        virt.overscan = 0;

        assert_eq!(virt.visible_range(), 2..4);
    }

    #[test]
    fn fixed_height_total_height_matches_item_count() {
        let virt = Virtualizer::new(7, LayoutStrategy::FixedHeight { item_height: 22.0 });

        assert_eq!(virt.total_height_px(), 154.0);
    }

    #[test]
    fn grid_layout_fallback_uses_estimated_height_math() {
        let mut virt = Virtualizer::new(
            20,
            LayoutStrategy::GridLayout {
                min_item_width: 120.0,
                max_item_width: 240.0,
                min_item_height: 50.0,
                max_item_height: Some(200.0),
                gap: 12.0,
            },
        );

        virt.set_scroll_state_mut(100.0, 0.0, 125.0, 0.0);
        virt.overscan = 2;

        assert_eq!(virt.visible_range(), 0..7);
        assert_eq!(virt.item_offset_px(3), 150.0);
        assert_eq!(virt.total_height_px(), 1000.0);
        assert_eq!(virt.scroll_to_index(3, ScrollAlign::Top), 150.0);
    }

    #[test]
    fn grid_layout_fallback_uses_inline_axis_estimate_when_horizontal() {
        let mut virt = Virtualizer::new(
            20,
            LayoutStrategy::GridLayout {
                min_item_width: 120.0,
                max_item_width: 240.0,
                min_item_height: 50.0,
                max_item_height: Some(200.0),
                gap: 12.0,
            },
        );

        virt.orientation = Orientation::Horizontal;
        virt.overscan = 0;
        virt.set_scroll_state_mut(0.0, 240.0, 40.0, 120.0);

        assert_eq!(virt.visible_range(), 2..3);
        assert_eq!(virt.scroll_to_index(2, ScrollAlign::Top), 240.0);
    }

    #[test]
    fn waterfall_layout_fallback_uses_estimated_height_math() {
        let mut virt = Virtualizer::new(
            20,
            LayoutStrategy::WaterfallLayout {
                min_item_width: 120.0,
                max_item_width: 240.0,
                min_item_height: 45.0,
                gap: 12.0,
            },
        );

        virt.set_scroll_state_mut(90.0, 0.0, 90.0, 0.0);
        virt.overscan = 1;

        assert_eq!(virt.visible_range(), 1..5);
        assert_eq!(virt.item_offset_px(4), 180.0);
        assert_eq!(virt.total_height_px(), 900.0);
        assert_eq!(virt.scroll_to_index(4, ScrollAlign::Center), 157.5);
    }

    #[test]
    fn table_layout_fallback_uses_estimated_height_math() {
        let mut virt = Virtualizer::new(
            8,
            LayoutStrategy::TableLayout {
                row_height: 35.0,
                header_height: 24.0,
                column_widths: vec![120.0, 160.0],
                row_gap: 4.0,
            },
        );

        virt.set_scroll_state_mut(70.0, 0.0, 70.0, 0.0);
        virt.overscan = 1;

        assert_eq!(virt.visible_range(), 1..5);
        assert_eq!(virt.item_offset_px(3), 105.0);
        assert_eq!(virt.total_height_px(), 280.0);
        assert_eq!(virt.scroll_to_index(3, ScrollAlign::Bottom), 70.0);
    }

    #[test]
    fn layout_strategy_estimated_height_matches_variant() {
        assert_eq!(
            LayoutStrategy::FixedHeight { item_height: 12.0 }.estimated_item_height(),
            12.0
        );
        assert_eq!(
            LayoutStrategy::VariableHeight {
                estimated_item_height: 13.0,
            }
            .estimated_item_height(),
            13.0
        );
        assert_eq!(
            LayoutStrategy::Grid {
                item_height: 14.0,
                columns: NonZeroUsize::new(2).expect("non-zero columns"),
            }
            .estimated_item_height(),
            14.0
        );
        assert_eq!(
            LayoutStrategy::GridLayout {
                min_item_width: 100.0,
                max_item_width: 200.0,
                min_item_height: 15.0,
                max_item_height: Some(250.0),
                gap: 8.0,
            }
            .estimated_item_height(),
            15.0
        );
        assert_eq!(
            LayoutStrategy::WaterfallLayout {
                min_item_width: 100.0,
                max_item_width: 200.0,
                min_item_height: 16.0,
                gap: 8.0,
            }
            .estimated_item_height(),
            16.0
        );
        assert_eq!(
            LayoutStrategy::TableLayout {
                row_height: 17.0,
                header_height: 24.0,
                column_widths: vec![100.0, 120.0],
                row_gap: 4.0,
            }
            .estimated_item_height(),
            17.0
        );
    }

    #[test]
    fn scroll_align_default_is_auto() {
        assert_eq!(ScrollAlign::default(), ScrollAlign::Auto);
    }

    #[test]
    fn new_virtualizer_uses_spec_defaults() {
        let virt = Virtualizer::new(5, LayoutStrategy::FixedHeight { item_height: 20.0 });

        assert_eq!(virt.orientation, Orientation::Vertical);
        assert_eq!(virt.dir, Direction::Ltr);
        assert_eq!(virt.overscan, DEFAULT_OVERSCAN);
        assert_eq!(virt.focused_index, None);
    }
}
