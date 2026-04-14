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
    use core::ops::Range;

    use super::{HorizontalVirtualLayout, VirtualLayout};

    struct DummyLayout;

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
            let _ = (index, height);
        }

        fn item_count(&self) -> usize {
            10
        }
    }

    struct DummyHorizontalLayout;

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
            let _ = (index, height);
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

    #[test]
    fn default_scroll_to_index_uses_item_offset() {
        let layout = DummyLayout;
        assert_eq!(layout.scroll_to_index(4), 40.0);
    }

    #[test]
    fn default_report_item_width_is_no_op() {
        let mut layout = DummyHorizontalLayout;
        layout.report_item_width(2, 88.0);
        assert_eq!(layout.item_count(), 6);
    }

    #[test]
    fn horizontal_layout_reports_visible_range() {
        let layout = DummyHorizontalLayout;
        assert_eq!(layout.visible_range_horizontal(16.0, 48.0), 2..5);
    }

    #[test]
    fn horizontal_layout_reports_item_offset_x() {
        let layout = DummyHorizontalLayout;
        assert_eq!(layout.item_offset_x(3), 36.0);
    }

    #[test]
    fn horizontal_layout_reports_total_width() {
        let layout = DummyHorizontalLayout;
        assert_eq!(layout.total_width(), 120.0);
    }
}
