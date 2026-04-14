//! Arrow-key navigation for composite widgets.

use core::num::NonZero;

use ars_core::KeyboardKey;

/// Configuration for a `FocusZone`.
#[derive(Clone, Debug)]
pub struct FocusZoneOptions {
    /// Axis of arrow-key navigation.
    pub direction: FocusZoneDirection,

    /// If true, pressing ArrowRight/Down at the last item wraps to the first.
    pub wrap: bool,

    /// If true, use roving tabindex strategy (only one item has tabindex=0).
    /// If false, use aria-activedescendant strategy.
    /// Corresponds to `FocusStrategy::RovingTabindex` (true) vs
    /// `FocusStrategy::ActiveDescendant` (false) — see §3.2.
    pub roving_tabindex: bool,

    /// If true, Home/End keys move to first/last item.
    pub home_end: bool,

    /// If true, PageUp/PageDown are active (useful for long lists).
    pub page_navigation: bool,

    /// Number of items to skip per PageUp/PageDown.
    pub page_size: NonZero<usize>,

    /// If true, disabled items are skipped during arrow-key navigation within
    /// the focus zone. Note: this controls arrow-key traversal only — disabled
    /// items remain in the Tab order per §13 (disabled elements stay focusable).
    ///
    /// Per-component guidance:
    ///   - `RadioGroup`, `Tabs`: SHOULD set `skip_disabled: false` to match APG
    ///     guidance that disabled options remain discoverable via arrow keys.
    ///   - Menu, Listbox: MAY keep `true` (default) since disabled items are
    ///     announced by screen readers but not interactable.
    pub skip_disabled: bool,
}

impl Default for FocusZoneOptions {
    fn default() -> Self {
        Self {
            direction: FocusZoneDirection::Vertical,
            wrap: true,
            roving_tabindex: true,
            home_end: true,
            page_navigation: false,
            page_size: NonZero::new(10).expect("hardcoded nonzero"),
            skip_disabled: true,
        }
    }
}

/// Axes supported by the focus-zone navigation engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusZoneDirection {
    /// Arrow Up/Down navigate; Arrow Left/Right are not intercepted.
    Vertical,
    /// Arrow Left/Right navigate; Arrow Up/Down are not intercepted.
    Horizontal,
    /// All four arrow keys navigate in a 2D grid.
    Grid {
        /// Number of columns in the grid.
        cols: NonZero<usize>,
    },
    /// Arrow Left/Right AND Up/Down navigate in a single-dimension flat list.
    /// All four arrow keys step +/- 1, with Left/Right flipped in RTL mode.
    /// This is NOT a 2D grid — use `Grid { cols }` for 2D navigation.
    /// Wrapping from the first item on `ArrowUp` to the last item (and vice versa)
    /// is controlled by `FocusZoneOptions::wrap` and is intentional for flat lists.
    Both,
}

impl FocusZoneDirection {
    /// Creates a `Grid` direction with the given non-zero column count.
    #[must_use]
    pub const fn grid(cols: NonZero<usize>) -> Self {
        Self::Grid { cols }
    }
}

/// A managed set of items navigable via arrow keys.
/// Used in the context of a component's machine context.
#[derive(Debug)]
pub struct FocusZone {
    /// Focus-zone behavior flags and navigation mode.
    pub options: FocusZoneOptions,
    /// Index of the currently active/focused item.
    pub active_index: usize,
    /// Total number of items (may be computed from a collection).
    pub item_count: usize,
}

impl FocusZone {
    /// Creates a focus zone with the given options and item count.
    #[must_use]
    pub const fn new(options: FocusZoneOptions, item_count: usize) -> Self {
        Self {
            options,
            active_index: 0,
            item_count,
        }
    }

    /// Process a navigation key and return the new active index (if changed).
    ///
    /// Returns `Some(new_index)` if navigation occurred, `None` if the key is not handled.
    pub fn handle_key(
        &self,
        key: KeyboardKey,
        is_rtl: bool,
        is_disabled: impl Fn(usize) -> bool,
    ) -> Option<usize> {
        if self.item_count == 0 {
            return None;
        }

        let (prev_key, next_key) = match self.options.direction {
            FocusZoneDirection::Vertical => (KeyboardKey::ArrowUp, KeyboardKey::ArrowDown),

            FocusZoneDirection::Horizontal => {
                if is_rtl {
                    (KeyboardKey::ArrowRight, KeyboardKey::ArrowLeft)
                } else {
                    (KeyboardKey::ArrowLeft, KeyboardKey::ArrowRight)
                }
            }

            FocusZoneDirection::Both | FocusZoneDirection::Grid { .. } => {
                (KeyboardKey::ArrowUp, KeyboardKey::ArrowDown)
            }
        };

        let (h_prev_key, h_next_key) = if is_rtl {
            (KeyboardKey::ArrowRight, KeyboardKey::ArrowLeft)
        } else {
            (KeyboardKey::ArrowLeft, KeyboardKey::ArrowRight)
        };

        let next = match key {
            k if k == prev_key
                && !matches!(self.options.direction, FocusZoneDirection::Grid { .. }) =>
            {
                self.navigate(-1, &is_disabled)
            }

            k if k == next_key
                && !matches!(self.options.direction, FocusZoneDirection::Grid { .. }) =>
            {
                self.navigate(1, &is_disabled)
            }

            k if matches!(self.options.direction, FocusZoneDirection::Both) && k == h_prev_key => {
                self.navigate(-1, &is_disabled)
            }

            k if matches!(self.options.direction, FocusZoneDirection::Both) && k == h_next_key => {
                self.navigate(1, &is_disabled)
            }

            k if matches!(self.options.direction, FocusZoneDirection::Grid { .. })
                && k == h_prev_key =>
            {
                self.navigate(-1, &is_disabled)
            }

            k if matches!(self.options.direction, FocusZoneDirection::Grid { .. })
                && k == h_next_key =>
            {
                self.navigate(1, &is_disabled)
            }

            KeyboardKey::ArrowUp
                if matches!(self.options.direction, FocusZoneDirection::Grid { .. }) =>
            {
                if let FocusZoneDirection::Grid { cols } = self.options.direction {
                    let stride = cols.get();

                    let mut candidate = self.active_index.checked_sub(stride);

                    while let Some(target) = candidate {
                        if !self.options.skip_disabled || !is_disabled(target) {
                            break;
                        }

                        candidate = target.checked_sub(stride);
                    }

                    candidate.filter(|&target| !self.options.skip_disabled || !is_disabled(target))
                } else {
                    None
                }
            }

            KeyboardKey::ArrowDown
                if matches!(self.options.direction, FocusZoneDirection::Grid { .. }) =>
            {
                if let FocusZoneDirection::Grid { cols } = self.options.direction {
                    let stride = cols.get();

                    let mut target = self.active_index + stride;

                    while target < self.item_count {
                        if !self.options.skip_disabled || !is_disabled(target) {
                            break;
                        }
                        target += stride;
                    }

                    if target < self.item_count
                        && (!self.options.skip_disabled || !is_disabled(target))
                    {
                        Some(target)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }

            KeyboardKey::Home if self.options.home_end => {
                self.find_from_inclusive(0, 1, &is_disabled)
            }

            KeyboardKey::End if self.options.home_end => {
                let last = self.item_count.saturating_sub(1);

                self.find_from_inclusive(last, -1, &is_disabled)
            }

            KeyboardKey::PageDown if self.options.page_navigation => {
                let target = (self.active_index + self.options.page_size.get())
                    .min(self.item_count.saturating_sub(1));

                self.find_from_inclusive(target, 1, &is_disabled)
            }

            KeyboardKey::PageUp if self.options.page_navigation => {
                let target = self
                    .active_index
                    .saturating_sub(self.options.page_size.get());

                self.find_from_inclusive(target, -1, &is_disabled)
            }
            _ => None,
        };

        next.filter(|&idx| idx != self.active_index)
    }

    fn navigate(&self, delta: i32, is_disabled: &impl Fn(usize) -> bool) -> Option<usize> {
        self.find_from(self.active_index, delta, is_disabled)
    }

    fn find_from(
        &self,
        start: usize,
        delta: i32,
        is_disabled: &impl Fn(usize) -> bool,
    ) -> Option<usize> {
        let count =
            i32::try_from(self.item_count).expect("FocusZone supports up to i32::MAX items");

        let mut idx = start as i32 + delta;

        for _ in 0..self.item_count {
            if self.options.wrap {
                idx = idx.rem_euclid(count);
            } else if idx < 0 || idx >= count {
                return None;
            }

            let candidate = idx as usize;

            if !self.options.skip_disabled || !is_disabled(candidate) {
                return Some(candidate);
            }

            idx += delta;
        }
        None
    }

    /// Like `find_from`, but tests `start` itself before stepping.
    /// Used by Home/End to ensure the boundary index is evaluated.
    fn find_from_inclusive(
        &self,
        start: usize,
        delta: i32,
        is_disabled: &impl Fn(usize) -> bool,
    ) -> Option<usize> {
        if !self.options.skip_disabled || !is_disabled(start) {
            return Some(start);
        }

        self.find_from(start, delta, is_disabled)
    }

    /// Generate tabindex value for an item at the given index.
    /// In roving tabindex mode: 0 for active, -1 for all others.
    /// In non-roving mode: all items get tabindex -1 (aria-activedescendant is used).
    #[must_use]
    pub const fn tabindex_for(&self, index: usize) -> i32 {
        if self.options.roving_tabindex {
            if index == self.active_index { 0 } else { -1 }
        } else {
            -1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disabled_items(indices: &[usize]) -> impl Fn(usize) -> bool + '_ {
        move |index| indices.contains(&index)
    }

    #[test]
    fn focus_zone_options_default_matches_spec_defaults() {
        let options = FocusZoneOptions::default();

        assert_eq!(options.direction, FocusZoneDirection::Vertical);
        assert!(options.wrap);
        assert!(options.roving_tabindex);
        assert!(options.home_end);
        assert!(!options.page_navigation);
        assert_eq!(options.page_size.get(), 10);
        assert!(options.skip_disabled);
    }

    #[test]
    fn focus_zone_new_starts_at_index_zero() {
        let options = FocusZoneOptions {
            direction: FocusZoneDirection::Both,
            ..FocusZoneOptions::default()
        };

        let zone = FocusZone::new(options.clone(), 7);

        assert_eq!(zone.options.direction, FocusZoneDirection::Both);
        assert_eq!(zone.active_index, 0);
        assert_eq!(zone.item_count, 7);
    }

    #[test]
    fn handle_key_returns_none_for_empty_zone() {
        let zone = FocusZone::new(FocusZoneOptions::default(), 0);

        assert_eq!(
            zone.handle_key(KeyboardKey::ArrowDown, false, disabled_items(&[])),
            None
        );
    }

    #[test]
    fn vertical_zone_arrow_keys_move_by_one() {
        let zone = FocusZone {
            options: FocusZoneOptions::default(),
            active_index: 1,
            item_count: 4,
        };

        assert_eq!(
            zone.handle_key(KeyboardKey::ArrowDown, false, disabled_items(&[])),
            Some(2)
        );
        assert_eq!(
            zone.handle_key(KeyboardKey::ArrowUp, false, disabled_items(&[])),
            Some(0)
        );
    }

    #[test]
    fn horizontal_zone_is_rtl_aware() {
        let zone = FocusZone {
            options: FocusZoneOptions {
                direction: FocusZoneDirection::Horizontal,
                ..FocusZoneOptions::default()
            },
            active_index: 1,
            item_count: 4,
        };

        assert_eq!(
            zone.handle_key(KeyboardKey::ArrowRight, false, disabled_items(&[])),
            Some(2)
        );
        assert_eq!(
            zone.handle_key(KeyboardKey::ArrowLeft, false, disabled_items(&[])),
            Some(0)
        );
        assert_eq!(
            zone.handle_key(KeyboardKey::ArrowLeft, true, disabled_items(&[])),
            Some(2)
        );
        assert_eq!(
            zone.handle_key(KeyboardKey::ArrowRight, true, disabled_items(&[])),
            Some(0)
        );
    }

    #[test]
    fn handle_key_returns_none_for_unhandled_key() {
        let zone = FocusZone::new(FocusZoneOptions::default(), 4);

        assert_eq!(
            zone.handle_key(KeyboardKey::Enter, false, disabled_items(&[])),
            None
        );
    }

    #[test]
    fn both_mode_uses_all_four_arrows_as_flat_list_navigation() {
        let zone = FocusZone {
            options: FocusZoneOptions {
                direction: FocusZoneDirection::Both,
                ..FocusZoneOptions::default()
            },
            active_index: 1,
            item_count: 4,
        };

        assert_eq!(
            zone.handle_key(KeyboardKey::ArrowUp, false, disabled_items(&[])),
            Some(0)
        );
        assert_eq!(
            zone.handle_key(KeyboardKey::ArrowDown, false, disabled_items(&[])),
            Some(2)
        );
        assert_eq!(
            zone.handle_key(KeyboardKey::ArrowLeft, false, disabled_items(&[])),
            Some(0)
        );
        assert_eq!(
            zone.handle_key(KeyboardKey::ArrowRight, false, disabled_items(&[])),
            Some(2)
        );
    }

    #[test]
    fn both_mode_horizontal_keys_flip_in_rtl() {
        let zone = FocusZone {
            options: FocusZoneOptions {
                direction: FocusZoneDirection::Both,
                ..FocusZoneOptions::default()
            },
            active_index: 1,
            item_count: 4,
        };

        assert_eq!(
            zone.handle_key(KeyboardKey::ArrowLeft, true, disabled_items(&[])),
            Some(2)
        );
        assert_eq!(
            zone.handle_key(KeyboardKey::ArrowRight, true, disabled_items(&[])),
            Some(0)
        );
    }

    #[test]
    fn grid_mode_uses_horizontal_and_vertical_strides() {
        let zone = FocusZone {
            options: FocusZoneOptions {
                direction: FocusZoneDirection::grid(NonZero::new(3).expect("hardcoded nonzero")),
                ..FocusZoneOptions::default()
            },
            active_index: 4,
            item_count: 9,
        };

        assert_eq!(
            zone.handle_key(KeyboardKey::ArrowLeft, false, disabled_items(&[])),
            Some(3)
        );
        assert_eq!(
            zone.handle_key(KeyboardKey::ArrowRight, false, disabled_items(&[])),
            Some(5)
        );
        assert_eq!(
            zone.handle_key(KeyboardKey::ArrowUp, false, disabled_items(&[])),
            Some(1)
        );
        assert_eq!(
            zone.handle_key(KeyboardKey::ArrowDown, false, disabled_items(&[])),
            Some(7)
        );
    }

    #[test]
    fn grid_horizontal_keys_flip_in_rtl() {
        let zone = FocusZone {
            options: FocusZoneOptions {
                direction: FocusZoneDirection::grid(NonZero::new(3).expect("hardcoded nonzero")),
                ..FocusZoneOptions::default()
            },
            active_index: 4,
            item_count: 9,
        };

        assert_eq!(
            zone.handle_key(KeyboardKey::ArrowRight, true, disabled_items(&[])),
            Some(3)
        );
        assert_eq!(
            zone.handle_key(KeyboardKey::ArrowLeft, true, disabled_items(&[])),
            Some(5)
        );
    }

    #[test]
    fn wrap_navigation_cycles_at_edges() {
        let zone = FocusZone {
            options: FocusZoneOptions::default(),
            active_index: 3,
            item_count: 4,
        };

        let reverse_zone = FocusZone {
            options: FocusZoneOptions::default(),
            active_index: 0,
            item_count: 4,
        };

        assert_eq!(
            zone.handle_key(KeyboardKey::ArrowDown, false, disabled_items(&[])),
            Some(0)
        );
        assert_eq!(
            reverse_zone.handle_key(KeyboardKey::ArrowUp, false, disabled_items(&[])),
            Some(3)
        );
    }

    #[test]
    fn no_wrap_returns_none_at_boundary() {
        let zone = FocusZone {
            options: FocusZoneOptions {
                wrap: false,
                ..FocusZoneOptions::default()
            },
            active_index: 3,
            item_count: 4,
        };

        assert_eq!(
            zone.handle_key(KeyboardKey::ArrowDown, false, disabled_items(&[])),
            None
        );
    }

    #[test]
    fn handle_key_returns_none_when_navigation_keeps_same_index() {
        let zone = FocusZone {
            options: FocusZoneOptions::default(),
            active_index: 0,
            item_count: 4,
        };

        assert_eq!(
            zone.handle_key(KeyboardKey::Home, false, disabled_items(&[])),
            None
        );
    }

    #[test]
    fn skip_disabled_traverses_multiple_consecutive_disabled_items() {
        let zone = FocusZone {
            options: FocusZoneOptions::default(),
            active_index: 0,
            item_count: 5,
        };

        assert_eq!(
            zone.handle_key(KeyboardKey::ArrowDown, false, disabled_items(&[1, 2])),
            Some(3)
        );
    }

    #[test]
    fn returns_none_when_all_reachable_items_are_disabled() {
        let zone = FocusZone {
            options: FocusZoneOptions::default(),
            active_index: 0,
            item_count: 3,
        };

        assert_eq!(
            zone.handle_key(KeyboardKey::ArrowDown, false, disabled_items(&[0, 1, 2])),
            None
        );
    }

    #[test]
    fn skip_disabled_false_allows_disabled_items_to_receive_focus() {
        let zone = FocusZone {
            options: FocusZoneOptions {
                skip_disabled: false,
                ..FocusZoneOptions::default()
            },
            active_index: 0,
            item_count: 4,
        };

        assert_eq!(
            zone.handle_key(KeyboardKey::ArrowDown, false, disabled_items(&[1])),
            Some(1)
        );
    }

    #[test]
    fn home_and_end_land_on_first_and_last_non_disabled_items() {
        let zone = FocusZone {
            options: FocusZoneOptions::default(),
            active_index: 4,
            item_count: 5,
        };

        assert_eq!(
            zone.handle_key(KeyboardKey::Home, false, disabled_items(&[0, 1])),
            Some(2)
        );
        assert_eq!(
            zone.handle_key(KeyboardKey::End, false, disabled_items(&[4])),
            Some(3)
        );
    }

    #[test]
    fn page_navigation_jumps_by_page_size() {
        let zone = FocusZone {
            options: FocusZoneOptions {
                page_navigation: true,
                page_size: NonZero::new(3).expect("hardcoded nonzero"),
                ..FocusZoneOptions::default()
            },
            active_index: 4,
            item_count: 10,
        };

        assert_eq!(
            zone.handle_key(KeyboardKey::PageDown, false, disabled_items(&[])),
            Some(7)
        );
        assert_eq!(
            zone.handle_key(KeyboardKey::PageUp, false, disabled_items(&[])),
            Some(1)
        );
    }

    #[test]
    fn tabindex_for_matches_roving_and_activedescendant_modes() {
        let roving = FocusZone {
            options: FocusZoneOptions::default(),
            active_index: 2,
            item_count: 4,
        };

        let activedescendant = FocusZone {
            options: FocusZoneOptions {
                roving_tabindex: false,
                ..FocusZoneOptions::default()
            },
            active_index: 2,
            item_count: 4,
        };

        assert_eq!(roving.tabindex_for(2), 0);
        assert_eq!(roving.tabindex_for(1), -1);
        assert_eq!(activedescendant.tabindex_for(2), -1);
        assert_eq!(activedescendant.tabindex_for(1), -1);
    }

    #[test]
    fn grid_vertical_navigation_skips_disabled_rows() {
        let zone = FocusZone {
            options: FocusZoneOptions {
                direction: FocusZoneDirection::grid(NonZero::new(3).expect("hardcoded nonzero")),
                ..FocusZoneOptions::default()
            },
            active_index: 0,
            item_count: 9,
        };

        assert_eq!(
            zone.handle_key(KeyboardKey::ArrowDown, false, disabled_items(&[3, 6])),
            None
        );

        let upward_zone = FocusZone {
            options: FocusZoneOptions {
                direction: FocusZoneDirection::grid(NonZero::new(3).expect("hardcoded nonzero")),
                ..FocusZoneOptions::default()
            },
            active_index: 6,
            item_count: 9,
        };

        assert_eq!(
            upward_zone.handle_key(KeyboardKey::ArrowUp, false, disabled_items(&[3])),
            Some(0)
        );
    }

    #[test]
    fn focus_zone_direction_grid_preserves_column_count() {
        let direction = FocusZoneDirection::grid(NonZero::new(4).expect("hardcoded nonzero"));

        assert_eq!(
            direction,
            FocusZoneDirection::Grid {
                cols: NonZero::new(4).expect("hardcoded nonzero")
            }
        );
    }
}
