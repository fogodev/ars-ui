use alloc::{collections::BTreeMap, vec, vec::Vec};
use core::{num::NonZeroUsize, ops::Range};

pub use ars_core::{Direction, Orientation};

use crate::key::Key;

/// The number of items to render beyond the visible range on each side.
pub const DEFAULT_OVERSCAN: usize = 5;

/// Normalizes a browser's `scrollLeft` value for RTL content to a
/// consistent `0..max_scroll` range measured from the inline-start edge.
///
/// Browsers differ in how they report `scrollLeft` for RTL containers:
/// - Chrome and Firefox use negative values (`-max..0`).
/// - Safari uses positive values (`0..max`).
///
/// This function converts both conventions to `0..max_scroll` where
/// `max_scroll = scroll_width - client_width`.
///
/// For LTR content, `scrollLeft` is already `0..max` and does not need
/// normalization.
#[must_use]
pub fn normalize_scroll_left_rtl(raw: f64, scroll_width: f64, client_width: f64) -> f64 {
    // Floor at 0 to prevent clamp panic when client_width > scroll_width
    // (transient browser measurement or caller error).
    let max_scroll = (scroll_width - client_width).max(0.0);

    if raw >= 0.0 {
        // Safari: raw is 0..max (already inline-start-based)
        raw.clamp(0.0, max_scroll)
    } else {
        // Chrome/Firefox: raw is -max..0 (negate to get 0..max)
        raw.abs().clamp(0.0, max_scroll)
    }
}

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

    /// Estimated size of a single item along the active scroll axis.
    #[must_use]
    pub fn estimated_item_extent(&self, orientation: Orientation) -> f64 {
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

    /// Applies a collection update that may have changed flat item indices.
    ///
    /// Because measured heights are keyed by flat index, any insert, removal,
    /// filter, or reorder can make cached measurements point at the wrong
    /// items. The tracked focused index is also invalid after any such update,
    /// because it may now identify a different item even when it remains
    /// in-bounds. This method updates the tracked item count, clears the
    /// measured height cache, and clears the focused index.
    pub fn apply_collection_change_mut(&mut self, total_count: usize) {
        self.total_count = total_count;

        self.measured_heights.clear();
        self.focused_index = None;
    }

    /// Returns a cloned virtualizer with a collection update applied.
    #[must_use]
    pub fn apply_collection_change(&self, total_count: usize) -> Self {
        let mut updated = self.clone();

        updated.apply_collection_change_mut(total_count);

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

            LayoutStrategy::GridLayout {
                min_item_width,
                min_item_height,
                gap,
                ..
            } => {
                let (main_size, cross_size) = self.axis_sizes(*min_item_width, *min_item_height);
                let cols = self.responsive_columns(cross_size, *gap);
                let row_stride = main_size + gap;
                let row_start = (scroll_offset / row_stride).floor() as usize;
                let row_end = ((scroll_offset + viewport_extent) / row_stride).ceil() as usize;

                (row_start * cols, (row_end * cols).min(self.total_count))
            }

            LayoutStrategy::WaterfallLayout {
                min_item_width,
                min_item_height,
                gap,
                ..
            } => {
                let (main_size, cross_size) = self.axis_sizes(*min_item_width, *min_item_height);
                let positions = self.waterfall_positions(cross_size, main_size, *gap);
                let mut first = self.total_count;
                let mut last = 0;

                for (i, &y) in positions.iter().enumerate() {
                    let h = self.measured_heights.get(&i).copied().unwrap_or(main_size);

                    let item_bottom = y + h;

                    // Item is visible if its bottom > scroll_offset and
                    // its top < scroll_offset + viewport_extent.
                    if item_bottom > scroll_offset && y < scroll_offset + viewport_extent {
                        first = first.min(i);
                        last = last.max(i + 1);
                    }
                }

                if first > last {
                    first = 0;
                    last = 0;
                }

                (first, last)
            }

            LayoutStrategy::TableLayout {
                row_height,
                header_height,
                column_widths,
                row_gap,
            } => match self.orientation {
                Orientation::Vertical => {
                    let row_stride = row_height + row_gap;
                    let data_offset = (scroll_offset - header_height).max(0.0);
                    let data_end = (scroll_offset + viewport_extent - header_height).max(0.0);
                    let first = (data_offset / row_stride).floor() as usize;
                    let last = (data_end / row_stride).ceil() as usize;

                    (first, last.min(self.total_count))
                }
                Orientation::Horizontal => {
                    let mut cumulative = 0.0_f64;
                    let mut first = column_widths.len();
                    let mut found_first = false;
                    let mut last = column_widths.len();

                    for (i, &w) in column_widths.iter().enumerate() {
                        if cumulative + w > scroll_offset && !found_first {
                            first = i;
                            found_first = true;
                        }
                        cumulative += w;
                        if cumulative >= scroll_offset + viewport_extent {
                            last = i + 1;
                            break;
                        }
                    }

                    (first, last.min(column_widths.len()))
                }
            },
        };

        let mut start = first_visible.saturating_sub(self.overscan);

        let mut end = last_visible
            .saturating_add(self.overscan)
            .min(self.total_count);

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

            LayoutStrategy::GridLayout {
                min_item_width,
                min_item_height,
                gap,
                ..
            } => {
                let (main_size, cross_size) = self.axis_sizes(*min_item_width, *min_item_height);
                let cols = self.responsive_columns(cross_size, *gap);

                (index / cols) as f64 * (main_size + gap)
            }

            LayoutStrategy::WaterfallLayout {
                min_item_width,
                min_item_height,
                gap,
                ..
            } => {
                let (main_size, cross_size) = self.axis_sizes(*min_item_width, *min_item_height);
                self.waterfall_positions(cross_size, main_size, *gap)
                    .get(index)
                    .copied()
                    .unwrap_or(0.0)
            }

            LayoutStrategy::TableLayout {
                row_height,
                header_height,
                column_widths,
                row_gap,
            } => match self.orientation {
                Orientation::Vertical => header_height + index as f64 * (row_height + row_gap),
                Orientation::Horizontal => column_widths.iter().take(index).sum(),
            },
        }
    }

    /// Returns the total scrollable extent for the current layout.
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

            LayoutStrategy::GridLayout {
                min_item_width,
                min_item_height,
                gap,
                ..
            } => {
                if self.total_count == 0 {
                    return 0.0;
                }

                let (main_size, cross_size) = self.axis_sizes(*min_item_width, *min_item_height);
                let cols = self.responsive_columns(cross_size, *gap);
                let rows = self.total_count.div_ceil(cols);

                rows as f64 * (main_size + gap) - gap
            }

            LayoutStrategy::WaterfallLayout {
                min_item_width,
                min_item_height,
                gap,
                ..
            } => {
                let (main_size, cross_size) = self.axis_sizes(*min_item_width, *min_item_height);
                self.waterfall_total_height(cross_size, main_size, *gap)
            }

            LayoutStrategy::TableLayout {
                row_height,
                header_height,
                row_gap,
                ..
            } => {
                if self.total_count == 0 {
                    return *header_height;
                }

                header_height + self.total_count as f64 * (row_height + row_gap) - row_gap
            }
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

            LayoutStrategy::GridLayout {
                min_item_width,
                min_item_height,
                ..
            } => self.axis_sizes(*min_item_width, *min_item_height).0,

            LayoutStrategy::WaterfallLayout {
                min_item_width,
                min_item_height,
                ..
            } => {
                let main_size = self.axis_sizes(*min_item_width, *min_item_height).0;
                self.measured_heights
                    .get(&index)
                    .copied()
                    .unwrap_or(main_size)
            }

            LayoutStrategy::TableLayout {
                row_height,
                column_widths,
                ..
            } => match self.orientation {
                Orientation::Vertical => *row_height,
                Orientation::Horizontal => column_widths.get(index).copied().unwrap_or(*row_height),
            },
        };

        let viewport_extent = self.viewport_extent();
        let max_scroll = (self.total_main_axis_extent() - viewport_extent).max(0.0);
        let clamped_scroll_offset = self.clamped_scroll_offset(viewport_extent);
        let item_end = offset + item_extent;

        let target_offset = match align {
            ScrollAlign::Auto => {
                if offset < clamped_scroll_offset {
                    offset
                } else if item_end > clamped_scroll_offset + viewport_extent {
                    item_end - viewport_extent
                } else {
                    clamped_scroll_offset
                }
            }

            ScrollAlign::Top => offset,

            ScrollAlign::Bottom => (item_end - viewport_extent).max(0.0),

            ScrollAlign::Center => (offset - (viewport_extent - item_extent) / 2.0).max(0.0),
        };

        target_offset.clamp(0.0, max_scroll)
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

    /// Computes the scroll adjustment needed to keep `anchor_index` at the
    /// same visual position after a layout change.
    ///
    /// Call this after [`Self::report_item_height_mut`] or
    /// [`Self::apply_collection_change_mut`] to compute the delta that the
    /// adapter should apply to `scroll_top` (see spec §6.6).
    ///
    /// `old_offset` is the `item_offset_px(anchor_index)` recorded **before**
    /// the layout change.
    #[must_use]
    pub fn scroll_adjustment_for_anchor(&self, anchor_index: usize, old_offset: f64) -> f64 {
        self.item_offset_px(anchor_index) - old_offset
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
                LayoutStrategy::TableLayout { column_widths, .. } => column_widths.iter().sum(),
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

    /// Returns `(main_axis_item_size, cross_axis_item_size)` for a responsive
    /// grid or waterfall layout based on the current orientation.
    ///
    /// Vertical: main = height, cross = width.
    /// Horizontal: main = width, cross = height.
    const fn axis_sizes(&self, min_item_width: f64, min_item_height: f64) -> (f64, f64) {
        match self.orientation {
            Orientation::Vertical => (min_item_height, min_item_width),
            Orientation::Horizontal => (min_item_width, min_item_height),
        }
    }

    /// Computes the responsive column count for `GridLayout` and
    /// `WaterfallLayout` from the cross-axis viewport extent.
    ///
    /// `cross_item_size` is the item dimension along the cross axis (width
    /// for vertical scroll, height for horizontal scroll).
    fn responsive_columns(&self, cross_item_size: f64, gap: f64) -> usize {
        let cross = match self.orientation {
            Orientation::Vertical => self.viewport_width,
            Orientation::Horizontal => self.viewport_height,
        };

        if cross <= 0.0 || cross_item_size <= 0.0 {
            return 1;
        }

        ((cross + gap) / (cross_item_size + gap)).floor().max(1.0) as usize
    }

    /// Computes the main-axis offset for every item in a waterfall (masonry)
    /// layout. Items are assigned to the shortest column. Returns a `Vec` of
    /// length `self.total_count` where each element is the offset of that item.
    ///
    /// `cross_item_size` and `main_item_size` are the orientation-resolved
    /// item dimensions (call [`Self::axis_sizes`] first).
    fn waterfall_positions(&self, cross_item_size: f64, main_item_size: f64, gap: f64) -> Vec<f64> {
        let columns = self.responsive_columns(cross_item_size, gap);

        let mut column_heights = vec![0.0_f64; columns];
        let mut positions = Vec::with_capacity(self.total_count);

        for i in 0..self.total_count {
            let (min_col, _) = column_heights
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal))
                .unwrap_or((0, &0.0));

            let y = column_heights[min_col];
            positions.push(y);

            let item_main = self
                .measured_heights
                .get(&i)
                .copied()
                .unwrap_or(main_item_size);

            column_heights[min_col] = y + item_main + gap;
        }

        positions
    }

    /// Returns the total main-axis extent of a waterfall layout (tallest
    /// column minus trailing gap).
    fn waterfall_total_height(&self, cross_item_size: f64, main_item_size: f64, gap: f64) -> f64 {
        if self.total_count == 0 {
            return 0.0;
        }

        let columns = self.responsive_columns(cross_item_size, gap);
        let mut column_heights = vec![0.0_f64; columns];

        for i in 0..self.total_count {
            let (min_col, _) = column_heights
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal))
                .unwrap_or((0, &0.0));

            let item_main = self
                .measured_heights
                .get(&i)
                .copied()
                .unwrap_or(main_item_size);

            column_heights[min_col] += item_main + gap;
        }

        let tallest = column_heights.iter().copied().fold(0.0_f64, f64::max);

        // Subtract trailing gap from the tallest column
        (tallest - gap).max(0.0)
    }
}

#[cfg(test)]
mod tests {
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
    fn scroll_to_index_top_clamps_to_scrollable_extent() {
        let mut virt = Virtualizer::new(4, LayoutStrategy::FixedHeight { item_height: 40.0 });

        virt.set_scroll_state_mut(0.0, 0.0, 50.0, 0.0);

        assert_eq!(virt.scroll_to_index(3, ScrollAlign::Top), 110.0);
    }

    #[test]
    fn scroll_to_index_center_clamps_to_scrollable_extent() {
        let mut virt = Virtualizer::new(4, LayoutStrategy::FixedHeight { item_height: 40.0 });

        virt.set_scroll_state_mut(0.0, 0.0, 50.0, 0.0);

        assert_eq!(virt.scroll_to_index(3, ScrollAlign::Center), 110.0);
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
    fn scroll_to_index_auto_clamps_stale_scroll_before_visibility_check() {
        let mut virt = Virtualizer::new(4, LayoutStrategy::FixedHeight { item_height: 40.0 });

        virt.set_scroll_state_mut(500.0, 0.0, 50.0, 0.0);
        virt.overscan = 0;

        assert_eq!(virt.scroll_to_index(3, ScrollAlign::Auto), 110.0);
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
    fn apply_collection_change_mut_clears_index_based_measurements_even_when_count_is_unchanged() {
        let mut virt = Virtualizer::new(
            4,
            LayoutStrategy::VariableHeight {
                estimated_item_height: 40.0,
            },
        );

        virt.report_item_height_mut(0, 60.0);
        virt.report_item_height_mut(1, 50.0);

        assert_eq!(virt.total_height_px(), 190.0);

        virt.apply_collection_change_mut(4);

        assert_eq!(virt.total_height_px(), 160.0);
    }

    #[test]
    fn apply_collection_change_updates_total_count_and_clears_focus() {
        let mut virt = fixed_height_virt();

        virt.focused_index = Some(8);
        virt.apply_collection_change_mut(3);

        assert_eq!(virt.total_count, 3);
        assert_eq!(virt.focused_index, None);
    }

    #[test]
    fn apply_collection_change_returns_new_virtualizer() {
        let mut baseline = Virtualizer::new(
            4,
            LayoutStrategy::VariableHeight {
                estimated_item_height: 40.0,
            },
        );

        baseline.focused_index = Some(1);
        baseline.report_item_height_mut(0, 60.0);

        let updated = baseline.apply_collection_change(6);

        assert_eq!(baseline.total_count, 4);
        assert_eq!(baseline.focused_index, Some(1));
        assert_eq!(baseline.total_height_px(), 180.0);

        assert_eq!(updated.total_count, 6);
        assert_eq!(updated.focused_index, None);
        assert_eq!(updated.total_height_px(), 240.0);
    }

    #[test]
    fn apply_collection_change_clears_focus_even_when_count_is_unchanged() {
        let mut virt = fixed_height_virt();

        virt.focused_index = Some(5);
        virt.apply_collection_change_mut(100);

        assert_eq!(virt.focused_index, None);
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
    fn horizontal_scroll_to_index_top_clamps_to_scrollable_extent() {
        let mut virt = Virtualizer::new(4, LayoutStrategy::FixedHeight { item_height: 40.0 });

        virt.orientation = Orientation::Horizontal;
        virt.set_scroll_state_mut(0.0, 0.0, 40.0, 50.0);

        assert_eq!(virt.scroll_to_index(3, ScrollAlign::Top), 110.0);
    }

    #[test]
    fn fixed_height_visible_range_clamps_when_scrolled_past_end() {
        let mut virt = Virtualizer::new(4, LayoutStrategy::FixedHeight { item_height: 40.0 });

        virt.set_scroll_state_mut(500.0, 0.0, 50.0, 0.0);
        virt.overscan = 0;

        assert_eq!(virt.visible_range(), 2..4);
    }

    #[test]
    fn visible_range_end_uses_saturating_add_for_large_overscan() {
        let mut virt = fixed_height_virt();

        virt.viewport_height = 50.0;
        virt.overscan = usize::MAX;

        assert_eq!(virt.visible_range(), 0..100);
    }

    #[test]
    fn fixed_height_total_height_matches_item_count() {
        let virt = Virtualizer::new(7, LayoutStrategy::FixedHeight { item_height: 22.0 });

        assert_eq!(virt.total_height_px(), 154.0);
    }

    // ── GridLayout specialized math ─────────────────────────────────

    fn grid_layout_virt() -> Virtualizer {
        // columns = floor((500 + 12) / (120 + 12)) = floor(512/132) = 3
        // row stride = 50 + 12 = 62 px
        // rows = ceil(20 / 3) = 7
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

        virt.set_scroll_state_mut(0.0, 0.0, 200.0, 500.0);
        virt.overscan = 2;

        virt
    }

    #[test]
    fn grid_layout_visible_range_uses_responsive_columns() {
        let virt = grid_layout_virt();

        // scroll_top=0, viewport=200, row_stride=62
        // visible rows 0..ceil(200/62) = 0..4 → items 0..12
        // overscan 2 rows: start stays 0, end = min(12 + 2*3, 20) = 18
        // Actually overscan is item-count based, not row-based:
        // first_visible = 0*3 = 0, last_visible = min(4*3, 20) = 12
        // start = 0 - 2 = 0, end = min(12 + 2, 20) = 14
        let range = virt.visible_range();

        assert_eq!(range.start, 0);
        assert_eq!(range.end, 14);
    }

    #[test]
    fn grid_layout_visible_range_scrolled() {
        let mut virt = grid_layout_virt();

        virt.set_scroll_state_mut(100.0, 0.0, 200.0, 500.0);

        // scroll_top=100, row_stride=62
        // first row = floor(100/62) = 1, last row = ceil(300/62) = 5
        // items: 1*3=3 .. min(5*3, 20)=15
        // overscan: start=3-2=1, end=min(15+2, 20)=17
        let range = virt.visible_range();

        assert_eq!(range.start, 1);
        assert_eq!(range.end, 17);
    }

    #[test]
    fn grid_layout_item_offset_uses_row_and_gap() {
        let virt = grid_layout_virt();

        // 3 columns, row stride = 62
        assert_eq!(virt.item_offset_px(0), 0.0); // row 0
        assert_eq!(virt.item_offset_px(2), 0.0); // row 0
        assert_eq!(virt.item_offset_px(3), 62.0); // row 1
        assert_eq!(virt.item_offset_px(4), 62.0); // row 1
        assert_eq!(virt.item_offset_px(6), 124.0); // row 2
    }

    #[test]
    fn grid_layout_total_height_accounts_for_gaps() {
        let virt = grid_layout_virt();

        // 7 rows * 62 - 12 = 422
        assert_eq!(virt.total_height_px(), 422.0);
    }

    #[test]
    fn grid_layout_total_height_empty_is_zero() {
        let virt = Virtualizer::new(
            0,
            LayoutStrategy::GridLayout {
                min_item_width: 120.0,
                max_item_width: 240.0,
                min_item_height: 50.0,
                max_item_height: Some(200.0),
                gap: 12.0,
            },
        );

        assert_eq!(virt.total_height_px(), 0.0);
    }

    #[test]
    fn grid_layout_scroll_to_index_uses_row_height() {
        let virt = grid_layout_virt();

        // item 4 is in row 1, offset = 62
        assert_eq!(virt.scroll_to_index(4, ScrollAlign::Top), 62.0);

        // item 6 is in row 2, offset = 124
        assert_eq!(virt.scroll_to_index(6, ScrollAlign::Top), 124.0);
    }

    #[test]
    fn grid_layout_narrow_viewport_defaults_to_one_column() {
        let mut virt = Virtualizer::new(
            6,
            LayoutStrategy::GridLayout {
                min_item_width: 120.0,
                max_item_width: 240.0,
                min_item_height: 50.0,
                max_item_height: None,
                gap: 12.0,
            },
        );

        // viewport_width = 0 → 1 column
        virt.set_scroll_state_mut(0.0, 0.0, 200.0, 0.0);

        // 1 column, 6 rows, row stride = 62
        // total = 6 * 62 - 12 = 360
        assert_eq!(virt.total_height_px(), 360.0);
        assert_eq!(virt.item_offset_px(3), 186.0); // row 3 = 3 * 62
    }

    #[test]
    fn grid_layout_horizontal_uses_cross_axis_for_columns() {
        let mut virt = Virtualizer::new(
            12,
            LayoutStrategy::GridLayout {
                min_item_width: 100.0,
                max_item_width: 200.0,
                min_item_height: 40.0,
                max_item_height: None,
                gap: 10.0,
            },
        );

        // Horizontal: cross axis = viewport_height, cross item = min_item_height = 40
        virt.orientation = Orientation::Horizontal;
        // cross_cols = floor((200 + 10) / (40 + 10)) = floor(210/50) = 4
        // main stride = min_item_width + gap = 100 + 10 = 110
        // rows (along main axis) = ceil(12/4) = 3
        // total main = 3 * 110 - 10 = 320
        virt.set_scroll_state_mut(0.0, 110.0, 200.0, 150.0);
        virt.overscan = 0;

        // scroll_left=110, viewport_width=150, main_stride=110
        // max_scroll = 320 - 150 = 170 → scroll_left clamped to 110
        // first = floor(110/110) = 1, last = ceil(260/110) = ceil(2.36) = 3
        // items: 1*4=4 .. min(3*4, 12)=12
        let range = virt.visible_range();

        assert_eq!(range.start, 4);
        assert_eq!(range.end, 12);
    }

    // ── WaterfallLayout specialized math ─────────────────────────────

    fn waterfall_virt() -> Virtualizer {
        // columns = floor((400 + 12) / (120 + 12)) = floor(412/132) = 3
        let mut virt = Virtualizer::new(
            6,
            LayoutStrategy::WaterfallLayout {
                min_item_width: 120.0,
                max_item_width: 240.0,
                min_item_height: 45.0,
                gap: 12.0,
            },
        );

        virt.set_scroll_state_mut(0.0, 0.0, 200.0, 400.0);
        virt.overscan = 0;

        virt
    }

    #[test]
    fn waterfall_layout_masonry_positioning_uniform_heights() {
        let virt = waterfall_virt();

        // 3 columns, all heights = 45 (min_item_height), gap = 12
        // Item 0 → col 0, Y=0   (col heights: [57, 0, 0])
        // Item 1 → col 1, Y=0   (col heights: [57, 57, 0])
        // Item 2 → col 2, Y=0   (col heights: [57, 57, 57])
        // Item 3 → col 0, Y=57  (col heights: [114, 57, 57])
        // Item 4 → col 1, Y=57  (col heights: [114, 114, 57])
        // Item 5 → col 2, Y=57  (col heights: [114, 114, 114])
        assert_eq!(virt.item_offset_px(0), 0.0);
        assert_eq!(virt.item_offset_px(1), 0.0);
        assert_eq!(virt.item_offset_px(2), 0.0);
        assert_eq!(virt.item_offset_px(3), 57.0);
        assert_eq!(virt.item_offset_px(4), 57.0);
        assert_eq!(virt.item_offset_px(5), 57.0);
    }

    #[test]
    fn waterfall_layout_uses_measured_heights() {
        let mut virt = waterfall_virt();

        // Measure items 0, 1, 2 with different heights
        virt.report_item_height_mut(0, 80.0);
        virt.report_item_height_mut(1, 45.0);
        virt.report_item_height_mut(2, 60.0);

        // Item 0 → col 0, Y=0   (col heights: [92, 0, 0])
        // Item 1 → col 1, Y=0   (col heights: [92, 57, 0])
        // Item 2 → col 2, Y=0   (col heights: [92, 57, 72])
        // Item 3 → col 1 (shortest=57), Y=57
        assert_eq!(virt.item_offset_px(3), 57.0);
    }

    #[test]
    fn waterfall_layout_total_height_is_tallest_column() {
        let mut virt = waterfall_virt();

        virt.report_item_height_mut(0, 80.0);
        virt.report_item_height_mut(1, 45.0);
        virt.report_item_height_mut(2, 60.0);

        // After all 6 items assigned:
        // Item 0 → col 0 h=80, col heights: [92, 0, 0]
        // Item 1 → col 1 h=45, col heights: [92, 57, 0]
        // Item 2 → col 2 h=60, col heights: [92, 57, 72]
        // Item 3 → col 1 h=45, col heights: [92, 114, 72]
        //   (col 1 was 57, item 3 at Y=57, new = 57+45+12=114)
        // Item 4 → col 2 h=45, col heights: [92, 114, 129]
        //   (col 2 was 72, item 4 at Y=72, new = 72+45+12=129)
        // Item 5 → col 0 h=45, col heights: [149, 114, 129]
        //   (col 0 was 92, item 5 at Y=92, new = 92+45+12=149)
        // Tallest = 149, subtract trailing gap = 149 - 12 = 137
        assert_eq!(virt.total_height_px(), 137.0);
    }

    #[test]
    fn waterfall_layout_visible_range_uses_masonry_positions() {
        let mut virt = Virtualizer::new(
            9,
            LayoutStrategy::WaterfallLayout {
                min_item_width: 120.0,
                max_item_width: 240.0,
                min_item_height: 45.0,
                gap: 12.0,
            },
        );

        // viewport_width=400 → 3 columns
        // All uniform: rows at Y=0, Y=57, Y=114
        virt.set_scroll_state_mut(50.0, 0.0, 70.0, 400.0);
        virt.overscan = 0;

        // Viewport [50, 120): items at Y=0 have bottom 45 < 50 → not visible
        // Items at Y=57 have bottom 102 → visible (57 < 120 and 102 > 50)
        // Items at Y=114 have bottom 159 → visible (114 < 120)
        // So items 3,4,5 (Y=57) and 6,7,8 (Y=114) are visible
        let range = virt.visible_range();

        assert_eq!(range.start, 3);
        assert_eq!(range.end, 9);
    }

    #[test]
    fn waterfall_layout_empty_collection() {
        let virt = Virtualizer::new(
            0,
            LayoutStrategy::WaterfallLayout {
                min_item_width: 120.0,
                max_item_width: 240.0,
                min_item_height: 45.0,
                gap: 12.0,
            },
        );

        assert_eq!(virt.total_height_px(), 0.0);
        assert_eq!(virt.visible_range(), 0..0);
    }

    #[test]
    fn waterfall_layout_scroll_to_index_uses_measured_extent() {
        let mut virt = Virtualizer::new(
            30,
            LayoutStrategy::WaterfallLayout {
                min_item_width: 120.0,
                max_item_width: 240.0,
                min_item_height: 45.0,
                gap: 12.0,
            },
        );

        // viewport_width=400 → 3 columns, need enough items so max_scroll > 0
        virt.set_scroll_state_mut(0.0, 0.0, 100.0, 400.0);
        virt.report_item_height_mut(10, 80.0);

        // scroll_to_index exercises the WaterfallLayout extent branch in
        // scroll_top_for_index, using measured height for the target item.
        let offset = virt.item_offset_px(10);
        let pos = virt.scroll_to_index(10, ScrollAlign::Top);

        assert_eq!(pos, offset);
    }

    // ── TableLayout specialized math ─────────────────────────────────

    fn table_virt() -> Virtualizer {
        let mut virt = Virtualizer::new(
            8,
            LayoutStrategy::TableLayout {
                row_height: 35.0,
                header_height: 24.0,
                column_widths: vec![120.0, 160.0],
                row_gap: 4.0,
            },
        );

        virt.set_scroll_state_mut(0.0, 0.0, 200.0, 400.0);

        virt
    }

    #[test]
    fn table_layout_item_offset_accounts_for_header_and_gap() {
        let virt = table_virt();

        // row stride = 35 + 4 = 39
        // item 0: header_height + 0 * 39 = 24
        // item 1: 24 + 39 = 63
        // item 3: 24 + 3 * 39 = 141
        // item 7: 24 + 7 * 39 = 297
        assert_eq!(virt.item_offset_px(0), 24.0);
        assert_eq!(virt.item_offset_px(1), 63.0);
        assert_eq!(virt.item_offset_px(3), 141.0);
        assert_eq!(virt.item_offset_px(7), 297.0);
    }

    #[test]
    fn table_layout_total_height_includes_header() {
        let virt = table_virt();

        // 24 + 8 * 39 - 4 = 24 + 312 - 4 = 332
        assert_eq!(virt.total_height_px(), 332.0);
    }

    #[test]
    fn table_layout_total_height_empty_is_header_only() {
        let virt = Virtualizer::new(
            0,
            LayoutStrategy::TableLayout {
                row_height: 35.0,
                header_height: 24.0,
                column_widths: vec![120.0, 160.0],
                row_gap: 4.0,
            },
        );

        assert_eq!(virt.total_height_px(), 24.0);
    }

    #[test]
    fn table_layout_visible_range_accounts_for_header() {
        let mut virt = table_virt();

        virt.set_scroll_state_mut(0.0, 0.0, 100.0, 400.0);
        virt.overscan = 0;

        // Data starts at Y=24, row stride=39
        // first = floor((0 - 24).max(0) / 39) = 0
        // last = ceil((100 - 24).max(0) / 39) = ceil(76/39) = 2
        let range = virt.visible_range();

        assert_eq!(range.start, 0);
        assert_eq!(range.end, 2);
    }

    #[test]
    fn table_layout_visible_range_scrolled_past_header() {
        let mut virt = table_virt();

        // row stride = 39, header = 24
        // item 0 at Y=24, item 1 at Y=63, item 2 at Y=102, item 3 at Y=141
        virt.set_scroll_state_mut(60.0, 0.0, 80.0, 400.0);
        virt.overscan = 1;

        // first = floor((60 - 24) / 39) = floor(36/39) = 0
        // last = ceil((140 - 24) / 39) = ceil(116/39) = 3
        // overscan: start = 0-1 sat= 0, end = min(3+1, 8) = 4
        let range = virt.visible_range();

        assert_eq!(range.start, 0);
        assert_eq!(range.end, 4);
    }

    #[test]
    fn table_layout_scroll_to_index_uses_header_offset() {
        let mut virt = Virtualizer::new(
            20,
            LayoutStrategy::TableLayout {
                row_height: 35.0,
                header_height: 24.0,
                column_widths: vec![120.0, 160.0],
                row_gap: 4.0,
            },
        );

        // total = 24 + 20*39 - 4 = 800, so max_scroll >> 141
        virt.set_scroll_state_mut(0.0, 0.0, 200.0, 400.0);

        // item 3 offset = 24 + 3*39 = 141
        assert_eq!(virt.scroll_to_index(3, ScrollAlign::Top), 141.0);
    }

    #[test]
    fn table_layout_horizontal_uses_column_widths() {
        let mut virt = Virtualizer::new(
            8,
            LayoutStrategy::TableLayout {
                row_height: 35.0,
                header_height: 24.0,
                column_widths: vec![120.0, 160.0, 80.0],
                row_gap: 4.0,
            },
        );

        virt.orientation = Orientation::Horizontal;
        // viewport_width = 200 for horizontal scroll
        virt.set_scroll_state_mut(0.0, 0.0, 300.0, 200.0);

        // total_height_px returns the vertical total regardless of orientation
        assert_eq!(virt.total_height_px(), 332.0);

        // Horizontal item_offset_px uses cumulative column widths:
        // col 0 offset = 0, col 1 offset = 120, col 2 offset = 280
        assert_eq!(virt.item_offset_px(0), 0.0);
        assert_eq!(virt.item_offset_px(1), 120.0);
        assert_eq!(virt.item_offset_px(2), 280.0);

        // total_main_axis_extent = sum(column_widths) = 360
        // max_scroll = 360 - 200 = 160
        // scroll_to_index(1, Top) = min(120, 160) = 120
        assert_eq!(virt.scroll_to_index(1, ScrollAlign::Top), 120.0);

        // visible_range with scroll_left=0: columns 0 (0..120) and 1 (120..280) overlap [0, 200)
        virt.overscan = 0;

        let range = virt.visible_range();

        assert_eq!(range.start, 0);
        assert_eq!(range.end, 2);
    }

    // ── RTL scroll normalization ─────────────────────────────────────

    #[test]
    fn normalize_scroll_left_rtl_chrome_firefox_negative() {
        // Chrome/Firefox: raw is -max..0
        assert_eq!(normalize_scroll_left_rtl(-200.0, 1000.0, 600.0), 200.0);
    }

    #[test]
    fn normalize_scroll_left_rtl_safari_positive() {
        // Safari: raw is 0..max
        assert_eq!(normalize_scroll_left_rtl(200.0, 1000.0, 600.0), 200.0);
    }

    #[test]
    fn normalize_scroll_left_rtl_zero() {
        assert_eq!(normalize_scroll_left_rtl(0.0, 1000.0, 600.0), 0.0);
    }

    #[test]
    fn normalize_scroll_left_rtl_clamps_to_max() {
        // max_scroll = 400, |-500| = 500 → clamped to 400
        assert_eq!(normalize_scroll_left_rtl(-500.0, 1000.0, 600.0), 400.0);
    }

    #[test]
    fn normalize_scroll_left_rtl_negative_max_scroll_returns_zero() {
        // client_width > scroll_width → max_scroll would be negative.
        // Must not panic; returns 0.
        assert_eq!(normalize_scroll_left_rtl(-10.0, 100.0, 200.0), 0.0);
        assert_eq!(normalize_scroll_left_rtl(10.0, 100.0, 200.0), 0.0);
        assert_eq!(normalize_scroll_left_rtl(0.0, 100.0, 200.0), 0.0);
    }

    #[test]
    fn normalize_scroll_left_rtl_max_value() {
        assert_eq!(normalize_scroll_left_rtl(400.0, 1000.0, 600.0), 400.0);
    }

    // ── Direction field tests ────────────────────────────────────────

    #[test]
    fn rtl_direction_does_not_affect_vertical_range() {
        let mut ltr_virt = fixed_height_virt();

        ltr_virt.dir = Direction::Ltr;

        let mut rtl_virt = fixed_height_virt();

        rtl_virt.dir = Direction::Rtl;

        assert_eq!(ltr_virt.visible_range(), rtl_virt.visible_range());
        assert_eq!(ltr_virt.total_height_px(), rtl_virt.total_height_px());
        assert_eq!(ltr_virt.item_offset_px(5), rtl_virt.item_offset_px(5));
    }

    #[test]
    fn rtl_horizontal_with_normalized_scroll_works_correctly() {
        let mut virt = fixed_height_virt();

        virt.dir = Direction::Rtl;
        virt.orientation = Orientation::Horizontal;
        virt.overscan = 0;

        // Pre-normalized scroll_left (adapter already called normalize_scroll_left_rtl)
        virt.set_scroll_state_mut(0.0, 120.0, 40.0, 80.0);

        assert_eq!(virt.visible_range(), 3..5);
    }

    #[test]
    fn direction_preserved_through_clone_and_apply() {
        let mut virt = fixed_height_virt();

        virt.dir = Direction::Rtl;

        let after_change = virt.apply_collection_change(50);

        assert_eq!(after_change.dir, Direction::Rtl);

        let after_focus = virt.set_focused_index(Some(3));

        assert_eq!(after_focus.dir, Direction::Rtl);
    }

    // ── Scroll position maintenance ──────────────────────────────────

    #[test]
    fn scroll_maintenance_height_change_above_viewport() {
        let mut virt = Virtualizer::new(
            10,
            LayoutStrategy::VariableHeight {
                estimated_item_height: 40.0,
            },
        );

        virt.set_scroll_state_mut(120.0, 0.0, 200.0, 0.0);

        let anchor = 5;
        let old_offset = virt.item_offset_px(anchor);

        assert_eq!(old_offset, 200.0); // 5 * 40

        // Item 0 measured taller: 40 → 80 (delta = +40)
        virt.report_item_height_mut(0, 80.0);

        let new_offset = virt.item_offset_px(anchor);

        assert_eq!(new_offset, 240.0); // 80 + 4*40 = 240

        let adjustment = virt.scroll_adjustment_for_anchor(anchor, old_offset);

        assert_eq!(adjustment, 40.0);
    }

    #[test]
    fn scroll_maintenance_apply_collection_change_resets_offsets() {
        let mut virt = Virtualizer::new(
            10,
            LayoutStrategy::VariableHeight {
                estimated_item_height: 40.0,
            },
        );

        virt.report_item_height_mut(0, 80.0);

        assert_eq!(virt.item_offset_px(3), 160.0); // 80 + 40 + 40

        virt.apply_collection_change_mut(10);

        // Measurements cleared, reverts to estimate
        assert_eq!(virt.item_offset_px(3), 120.0); // 3 * 40
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
    fn estimated_item_extent_vertical_matches_estimated_height() {
        let grid = LayoutStrategy::GridLayout {
            min_item_width: 100.0,
            max_item_width: 200.0,
            min_item_height: 50.0,
            max_item_height: None,
            gap: 10.0,
        };

        assert_eq!(grid.estimated_item_extent(Orientation::Vertical), 50.0);
    }

    #[test]
    fn estimated_item_extent_horizontal_uses_width_for_grid_and_waterfall() {
        let grid = LayoutStrategy::GridLayout {
            min_item_width: 100.0,
            max_item_width: 200.0,
            min_item_height: 50.0,
            max_item_height: None,
            gap: 10.0,
        };

        assert_eq!(grid.estimated_item_extent(Orientation::Horizontal), 100.0);

        let waterfall = LayoutStrategy::WaterfallLayout {
            min_item_width: 120.0,
            max_item_width: 240.0,
            min_item_height: 45.0,
            gap: 12.0,
        };

        assert_eq!(
            waterfall.estimated_item_extent(Orientation::Horizontal),
            120.0
        );
    }

    #[test]
    fn estimated_item_extent_horizontal_table_uses_first_column_width() {
        let table = LayoutStrategy::TableLayout {
            row_height: 35.0,
            header_height: 24.0,
            column_widths: vec![150.0, 200.0],
            row_gap: 4.0,
        };

        assert_eq!(table.estimated_item_extent(Orientation::Horizontal), 150.0);

        let table_no_cols = LayoutStrategy::TableLayout {
            row_height: 35.0,
            header_height: 24.0,
            column_widths: vec![],
            row_gap: 4.0,
        };

        // Falls back to row_height when no columns
        assert_eq!(
            table_no_cols.estimated_item_extent(Orientation::Horizontal),
            35.0
        );
    }

    #[test]
    fn estimated_item_extent_horizontal_fixed_height_falls_back_to_height() {
        let fixed = LayoutStrategy::FixedHeight { item_height: 40.0 };

        assert_eq!(fixed.estimated_item_extent(Orientation::Horizontal), 40.0);
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
