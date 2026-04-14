use core::ops::Range;

/// A layout algorithm that maps collection indices to vertical pixel coordinates.
///
/// The [`crate::Virtualizer`] type provides built-in strategies for the
/// standard layouts defined by the spec. This trait exists as an extension
/// point for layouts that need custom vertical range or positioning logic.
pub trait VirtualLayout {
    /// Returns the visible item range `[start, end)` for the given vertical
    /// scroll state.
    fn visible_range(&self, scroll_offset: f64, viewport_height: f64) -> Range<usize>;

    /// Returns the Y-axis pixel offset for the item at `index`.
    fn item_offset(&self, index: usize) -> f64;

    /// Returns the total scrollable height of the layout.
    fn total_height(&self) -> f64;

    /// Reports the measured height for an item.
    fn report_item_height(&mut self, index: usize, height: f64);

    /// Returns the scroll position that aligns the item to the viewport top.
    fn scroll_to_index(&self, index: usize) -> f64 {
        self.item_offset(index)
    }

    /// Returns the total number of items known to the layout.
    fn item_count(&self) -> usize;
}

/// An optional horizontal extension for layouts that support inline-axis virtualization.
///
/// Layouts for carousels, horizontal grid lists, or bidirectional scrollers
/// implement this trait in addition to [`VirtualLayout`].
pub trait HorizontalVirtualLayout: VirtualLayout {
    /// Returns the visible item range `[start, end)` for the given horizontal
    /// scroll state.
    fn visible_range_horizontal(&self, scroll_offset: f64, viewport_width: f64) -> Range<usize>;

    /// Returns the X-axis pixel offset for the item at `index`.
    fn item_offset_x(&self, index: usize) -> f64;

    /// Returns the total scrollable width of the layout.
    fn total_width(&self) -> f64;

    /// Reports the measured width for an item.
    fn report_item_width(&mut self, index: usize, width: f64) {
        let _ = (index, width);
    }
}

#[cfg(test)]
mod tests {
    use core::{cell::Cell, ops::Range};

    use super::{HorizontalVirtualLayout, VirtualLayout};

    #[derive(Default)]
    struct DummyLayout {
        last_height_report: Cell<Option<(usize, f64)>>,
    }

    impl VirtualLayout for DummyLayout {
        fn visible_range(&self, scroll_offset: f64, viewport_height: f64) -> Range<usize> {
            let _ = (scroll_offset, viewport_height);
            1..3
        }

        fn item_offset(&self, index: usize) -> f64 {
            index as f64 * 10.0
        }

        fn total_height(&self) -> f64 {
            100.0
        }

        fn report_item_height(&mut self, index: usize, height: f64) {
            self.last_height_report.set(Some((index, height)));
        }

        fn item_count(&self) -> usize {
            10
        }
    }

    #[derive(Default)]
    struct DummyHorizontalLayout {
        last_width_report: Cell<Option<(usize, f64)>>,
    }

    impl VirtualLayout for DummyHorizontalLayout {
        fn visible_range(&self, scroll_offset: f64, viewport_height: f64) -> Range<usize> {
            let _ = (scroll_offset, viewport_height);
            0..2
        }

        fn item_offset(&self, index: usize) -> f64 {
            index as f64 * 8.0
        }

        fn total_height(&self) -> f64 {
            80.0
        }

        fn report_item_height(&mut self, index: usize, height: f64) {
            self.last_width_report.set(Some((index, height)));
        }

        fn item_count(&self) -> usize {
            6
        }
    }

    impl HorizontalVirtualLayout for DummyHorizontalLayout {
        fn visible_range_horizontal(
            &self,
            scroll_offset: f64,
            viewport_width: f64,
        ) -> Range<usize> {
            let _ = (scroll_offset, viewport_width);
            2..5
        }

        fn item_offset_x(&self, index: usize) -> f64 {
            index as f64 * 12.0
        }

        fn total_width(&self) -> f64 {
            120.0
        }
    }

    fn top_offset_for<T: VirtualLayout>(layout: &T, index: usize) -> f64 {
        layout.scroll_to_index(index)
    }

    fn horizontal_window_for<T: HorizontalVirtualLayout>(
        layout: &T,
        scroll_offset: f64,
        viewport_width: f64,
    ) -> Range<usize> {
        layout.visible_range_horizontal(scroll_offset, viewport_width)
    }

    fn vertical_window_for<T: VirtualLayout>(
        layout: &T,
        scroll_offset: f64,
        viewport_height: f64,
    ) -> Range<usize> {
        layout.visible_range(scroll_offset, viewport_height)
    }

    #[test]
    fn default_scroll_to_index_uses_item_offset() {
        let layout = DummyLayout::default();
        assert_eq!(layout.scroll_to_index(4), 40.0);
    }

    #[test]
    fn default_report_item_width_is_no_op() {
        let mut layout = DummyHorizontalLayout::default();
        layout.report_item_width(2, 88.0);
        assert_eq!(layout.item_count(), 6);
        assert_eq!(layout.last_width_report.get(), None);
    }

    #[test]
    fn vertical_layout_reports_visible_range() {
        let layout = DummyLayout::default();
        assert_eq!(layout.visible_range(12.0, 48.0), 1..3);
    }

    #[test]
    fn vertical_layout_reports_total_height() {
        let layout = DummyLayout::default();
        assert_eq!(layout.total_height(), 100.0);
    }

    #[test]
    fn vertical_layout_reports_item_count() {
        let layout = DummyLayout::default();
        assert_eq!(layout.item_count(), 10);
    }

    #[test]
    fn vertical_layout_records_reported_item_height() {
        let mut layout = DummyLayout::default();
        layout.report_item_height(3, 44.0);
        assert_eq!(layout.last_height_report.get(), Some((3, 44.0)));
    }

    #[test]
    fn horizontal_layout_reports_visible_range() {
        let layout = DummyHorizontalLayout::default();
        assert_eq!(layout.visible_range_horizontal(16.0, 48.0), 2..5);
    }

    #[test]
    fn horizontal_layout_also_reports_vertical_visible_range() {
        let layout = DummyHorizontalLayout::default();
        assert_eq!(layout.visible_range(10.0, 24.0), 0..2);
    }

    #[test]
    fn horizontal_layout_reports_item_offset_x() {
        let layout = DummyHorizontalLayout::default();
        assert_eq!(layout.item_offset_x(3), 36.0);
    }

    #[test]
    fn horizontal_layout_also_reports_vertical_item_offset() {
        let layout = DummyHorizontalLayout::default();
        assert_eq!(layout.item_offset(4), 32.0);
    }

    #[test]
    fn horizontal_layout_reports_total_width() {
        let layout = DummyHorizontalLayout::default();
        assert_eq!(layout.total_width(), 120.0);
    }

    #[test]
    fn horizontal_layout_also_reports_total_height() {
        let layout = DummyHorizontalLayout::default();
        assert_eq!(layout.total_height(), 80.0);
    }

    #[test]
    fn horizontal_layout_records_reported_item_height() {
        let mut layout = DummyHorizontalLayout::default();
        layout.report_item_height(1, 22.0);
        assert_eq!(layout.last_width_report.get(), Some((1, 22.0)));
    }

    #[test]
    fn horizontal_layout_reports_item_count() {
        let layout = DummyHorizontalLayout::default();
        assert_eq!(layout.item_count(), 6);
    }

    #[test]
    fn virtual_layout_trait_bound_uses_default_scroll_helper() {
        let layout = DummyLayout::default();
        assert_eq!(top_offset_for(&layout, 5), 50.0);
    }

    #[test]
    fn horizontal_virtual_layout_trait_bound_uses_horizontal_range() {
        let layout = DummyHorizontalLayout::default();
        assert_eq!(horizontal_window_for(&layout, 24.0, 60.0), 2..5);
    }

    #[test]
    fn horizontal_layout_also_satisfies_vertical_trait_bound() {
        let layout = DummyHorizontalLayout::default();
        assert_eq!(vertical_window_for(&layout, 12.0, 24.0), 0..2);
    }
}
