//! DOM utilities for focus management, scroll control, z-index allocation,
//! overlay stacking, positioning, and platform capability detection.
//!
//! This crate provides browser-level helpers shared across framework adapters,
//! including focus management, scroll lock management for modal overlays,
//! overlay stack registry for nested overlay dismissal, media-query capability
//! probes, and a DOM measurement-to-position pipeline
//! (`measure_and_compute_position`) that overlay adapters call to avoid
//! duplicating browser measurement and coordinate-space conversion.
//!
//! # Feature flags
//!
//! - `web` (enabled by default) — exposes DOM-backed APIs that take raw
//!   `web_sys` types. On non-`wasm32` targets these compile to safe no-op /
//!   unavailable-result stubs so components can still link.
//! - `ssr` — opt-in marker for the cross-build CI matrix entry
//!   (`cargo test -p ars-dom --features ssr`, per
//!   `spec/testing/13-policies.md` §2.3). No production code is gated behind
//!   it today; it only enables the `ssr_smoke_tests` module that anchors the
//!   matrix entry. The cross-build subset of the crate (focus scopes, scroll
//!   lock, overlay stack, z-index allocator, media defaults, and the
//!   `positioning` types/engine) is available whenever the `web` feature is
//!   disabled, regardless of `ssr`. To build an SSR-safe target, pass
//!   `--no-default-features` (with or without `--features ssr`).

// Many ars-dom functions have cfg-gated web implementations that call
// web_sys/js_sys APIs at runtime. The non-web stubs look const-eligible
// to clippy, but the web paths cannot be const.
#![expect(
    clippy::missing_const_for_fn,
    reason = "cfg-gated web impls prevent const fn"
)]

mod announcer;
mod debug;
pub mod focus;
pub mod media;
pub mod modality;
pub mod outside_interaction;
pub mod overlay_stack;
#[cfg(feature = "web")]
pub mod platform;
pub mod portal;
pub mod positioning;
mod scroll;
pub mod scroll_lock;
pub mod z_index;

#[cfg(feature = "web")]
pub use announcer::ensure_dom;
pub use focus::{
    FocusScope, FocusedElement, focus_body, focus_element_by_id, focus_first_tabbable,
};
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub use focus::{
    document_contains, focus_element, get_first_focusable, get_focusable_elements,
    get_html_element_by_id, get_last_focusable,
};
pub use media::{
    ColorScheme, is_forced_colors_active, prefers_color_scheme, prefers_high_contrast,
    prefers_reduced_motion, prefers_reduced_transparency,
};
pub use modality::ModalityManager;
pub use outside_interaction::{OutsideInteractionConfig, id_matches_inside_set};
#[cfg(feature = "web")]
pub use outside_interaction::{install_outside_interaction_listeners, target_is_inside_boundary};
pub use overlay_stack::{
    OverlayEntry, contains_overlay, is_above, is_topmost, overlay_count, overlays_above,
    push_overlay, remove_overlay, reset_overlay_stack, topmost_overlay,
};
#[cfg(feature = "web")]
pub use platform::WebPlatformEffects;
#[cfg(feature = "web")]
pub use portal::{
    ensure_portal_mount_root, get_or_create_portal_root, remove_inert_from_siblings,
    set_background_inert, supports_inert,
};
pub use positioning::{
    Alignment, Axis, Boundary, Offset, Overflow, Placement, PositioningOptions, PositioningResult,
    Rect, Side, Strategy, VirtualElement, client_point_to_local_space, client_rect_to_local_space,
    compute_position,
};
#[cfg(feature = "web")]
pub use positioning::{
    auto_update, find_containing_block_ancestor, measure_and_compute_position, offset_parent_rect,
    warn_if_floating_element_has_containment_issue, warn_if_portal_target_has_containing_block,
};
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub use positioning::{viewport_height, viewport_rect, viewport_width};
pub use scroll::{ScrollIntoViewOptions, ScrollLogicalPosition, supports_scroll_into_view_options};
#[cfg(feature = "web")]
pub use scroll::{nearest_scrollable_ancestor, scroll_into_view_if_needed, scrollable_ancestors};
pub use scroll_lock::{
    ScrollLockManager, acquire, depth, is_locked, prevent_scroll, release, restore_scroll,
};
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub use scroll_lock::{needs_ios_workaround, scrollbar_width};
pub use z_index::{
    Z_INDEX_BASE, Z_INDEX_CEILING, ZIndexAllocator, ZIndexClaim, next_z_index, reset_z_index,
    supports_top_layer,
};

#[cfg(all(test, feature = "ssr"))]
mod ssr_smoke_tests {
    //! Anchors the `--features ssr` CI matrix entry from
    //! `spec/testing/13-policies.md` §4 "ars-dom Feature Flags".
    //!
    //! The crate's SSR contract is that the non-`web_sys` public surface
    //! compiles and runs on any target. These tests exercise a
    //! representative slice so a regression that leaks a browser type into
    //! the cross-build API surfaces immediately in CI instead of waiting
    //! for an SSR consumer to trip over it.
    use super::{
        OverlayEntry, Placement, PositioningOptions, Rect, ZIndexAllocator,
        client_point_to_local_space, client_rect_to_local_space, compute_position,
        contains_overlay, overlay_count, push_overlay, remove_overlay, reset_overlay_stack,
    };

    #[test]
    fn ssr_dom_abstraction_available() {
        // 1. Overlay stack round-trip (pure Rust, no DOM).
        reset_overlay_stack();

        assert_eq!(overlay_count(), 0);

        push_overlay(OverlayEntry {
            id: "ssr-probe".into(),
            modal: false,
            z_index: None,
        });

        assert!(contains_overlay("ssr-probe"));

        remove_overlay("ssr-probe");

        assert!(!contains_overlay("ssr-probe"));

        // 2. Z-index allocator is host-usable without the `web` feature.
        let allocator = ZIndexAllocator::new();

        let z = allocator.allocate();

        allocator.release(z);

        // 3. The positioning engine types and `compute_position()` belong to
        //    the cross-build surface: they accept rects, not `web_sys`
        //    elements, so SSR renderers that pre-measure geometry can reuse
        //    them without pulling in the browser.
        let anchor = Rect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 40.0,
        };

        let floating = Rect {
            x: 0.0,
            y: 0.0,
            width: 80.0,
            height: 20.0,
        };

        let viewport = Rect {
            x: 0.0,
            y: 0.0,
            width: 1024.0,
            height: 768.0,
        };

        let options = PositioningOptions {
            placement: Placement::Bottom,
            flip: false,
            shift: false,
            ..PositioningOptions::default()
        };

        let _result = compute_position(&anchor, &floating, &viewport, &options);

        // 4. Coordinate-space helpers round-trip without the `web` feature.
        //    SSR adapters that pre-measure geometry call these to convert
        //    client-space rects/points into a containing block's local space
        //    before feeding them to `compute_position`.
        let local_origin = Rect {
            x: 12.5,
            y: 20.25,
            width: 80.0,
            height: 40.0,
        };

        assert_eq!(
            client_point_to_local_space(33.75, 48.5, &local_origin),
            (21.25, 28.25)
        );

        let client_rect = Rect {
            x: 100.5,
            y: 250.25,
            width: 75.75,
            height: 40.5,
        };

        assert_eq!(
            client_rect_to_local_space(&client_rect, &local_origin),
            Rect {
                x: 88.0,
                y: 230.0,
                width: 75.75,
                height: 40.5,
            }
        );
    }
}
