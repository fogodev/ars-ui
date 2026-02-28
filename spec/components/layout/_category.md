# Layout Components Specification

## Table of Contents

- [Splitter (Resizable Panels)](splitter.md)
- [ScrollArea (Custom Scrollbar)](scroll-area.md)
- [Carousel](carousel.md)
- [AspectRatio](aspect-ratio.md)
- [Stack](stack.md)
- [Center](center.md)
- [Grid](grid.md)
- [Portal](portal.md)
- [Collapsible](collapsible.md)
- [Toolbar](toolbar.md)
- [Frame](frame.md)

## Appendix A: Core Types Reference

See `foundation/01-architecture.md` for core types used by all layout components:

- **`AttrMap`** — framework-agnostic attribute/style container (§3.2)
- **`Bindable<T>`** — controlled/uncontrolled value abstraction (§2.6)
- **`ComponentIds`** — hydration-safe ID derivation for parts (§2.1)
- **`ComponentPart` / `ConnectApi`** — part enum and API dispatch traits (§2.1)

ARIA attributes are conveyed through `AttrMap` using `HtmlAttr::Aria(AriaAttr::*)` variants. There is no flat `AriaAttributes` struct.

## Appendix B: CSS Custom Property Hooks

All layout components expose CSS custom properties for zero-specificity
customisation.

### Splitter

```css
:root {
  --ars-splitter-handle-size: 4px;
  --ars-splitter-handle-color: transparent;
  --ars-splitter-handle-hover-color: var(--color-accent);
  --ars-splitter-handle-active-color: var(--color-accent-strong);
  --ars-splitter-panel-min-size: 0px;
}
```

### ScrollArea

```css
:root {
  --ars-scrollbar-size: 8px;
  --ars-scrollbar-track-color: transparent;
  --ars-scrollbar-thumb-color: rgba(0, 0, 0, 0.3);
  --ars-scrollbar-thumb-hover-color: rgba(0, 0, 0, 0.5);
  --ars-scrollbar-thumb-radius: 4px;
  --ars-scrollbar-fade-duration: 300ms;
}
```

### Carousel

```css
:root {
  --ars-carousel-transition-duration: 300ms;
  --ars-carousel-transition-easing: ease-in-out;
  --ars-carousel-indicator-size: 8px;
  --ars-carousel-indicator-gap: 6px;
  --ars-carousel-indicator-color: rgba(0, 0, 0, 0.3);
  --ars-carousel-indicator-active-color: var(--color-accent);
}
```

## Appendix C: Unit Tests

```rust
#[cfg(test)]
mod layout_tests {
    use super::*;
    use crate::splitter::{self, *};
    use crate::scroll_area::*;
    use crate::carousel::*;
    use crate::stack::*;
    use crate::aspect_ratio::*;

    fn two_panels() -> Vec<splitter::Panel> {
        vec![
            splitter::Panel {
                id: "a".into(),
                min_size: 10.0,
                default_size: 50.0,
                ..splitter::Panel::default()
            },
            splitter::Panel {
                id: "b".into(),
                min_size: 10.0,
                default_size: 50.0,
                ..splitter::Panel::default()
            },
        ]
    }

    // ── Splitter ─────────────────────────────────────────────────────────

    #[test]
    fn resize_respects_min_size() {
        let panels = two_panels();
        let result = compute_resize(&[50.0, 50.0], 0, 200.0, &panels);
        assert_eq!(result[1], 10.0, "panel b at min");
        assert_eq!(result[0], 90.0, "panel a gets remainder");
    }

    #[test]
    fn resize_negative_delta_respects_left_min() {
        let panels = two_panels();
        let result = compute_resize(&[50.0, 50.0], 0, -200.0, &panels);
        assert_eq!(result[0], 10.0, "panel a at min");
        assert_eq!(result[1], 90.0, "panel b gets remainder");
    }

    #[test]
    fn collapse_and_expand_round_trip() {
        let panels = vec![
            splitter::Panel {
                id: "a".into(),
                collapsible: true,
                min_size: 20.0,
                default_size: 40.0,
                ..splitter::Panel::default()
            },
            splitter::Panel {
                id: "b".into(),
                min_size: 20.0,
                default_size: 60.0,
                ..splitter::Panel::default()
            },
        ];
        let mut sizes = vec![40.0, 60.0];
        collapse_panel(&mut sizes, 0, &panels);
        assert_eq!(sizes[0], 0.0, "collapsed to 0");
        assert_eq!(sizes[1], 100.0, "b absorbed freed space");

        expand_panel(&mut sizes, 0, &panels);
        assert_eq!(sizes[0], 40.0, "restored to default_size");
        assert_eq!(sizes[1], 60.0, "b gave back space");
    }

    #[test]
    fn max_size_constraint_is_respected() {
        let panels = vec![
            splitter::Panel {
                id: "a".into(),
                min_size: 10.0,
                max_size: Some(60.0),
                default_size: 50.0,
                ..splitter::Panel::default()
            },
            splitter::Panel {
                id: "b".into(),
                min_size: 10.0,
                default_size: 50.0,
                ..splitter::Panel::default()
            },
        ];
        let result = compute_resize(&[50.0, 50.0], 0, 30.0, &panels);
        assert_eq!(result[0], 60.0, "panel a at max_size");
        assert_eq!(result[1], 40.0, "panel b reduced");
    }

    // ── ScrollArea ───────────────────────────────────────────────────────

    #[test]
    fn thumb_at_start_of_track() {
        let (size, pos) = compute_thumb_metrics(500.0, 1000.0, 0.0, 500.0, 20.0);
        assert!((size - 250.0).abs() < 0.01, "thumb = half track");
        assert!((pos - 0.0).abs() < 0.01, "thumb at top");
    }

    #[test]
    fn thumb_at_end_of_track() {
        let (size, pos) = compute_thumb_metrics(500.0, 1000.0, 500.0, 500.0, 20.0);
        assert!((size - 250.0).abs() < 0.01);
        assert!((pos - 250.0).abs() < 0.01, "thumb at bottom: {pos}");
    }

    #[test]
    fn thumb_at_middle_of_track() {
        let (size, pos) = compute_thumb_metrics(500.0, 1000.0, 250.0, 500.0, 20.0);
        assert!((size - 250.0).abs() < 0.01);
        assert!((pos - 125.0).abs() < 0.01, "thumb at midpoint: {pos}");
    }

    #[test]
    fn min_thumb_size_applied() {
        let (size, _) = compute_thumb_metrics(10.0, 10000.0, 0.0, 500.0, 30.0);
        assert!(size >= 30.0, "thumb must be at least min_thumb_size");
    }

    #[test]
    fn no_overflow_fills_track() {
        let (size, pos) = compute_thumb_metrics(500.0, 400.0, 0.0, 500.0, 20.0);
        assert_eq!(size, 500.0, "no overflow: thumb fills track");
        assert_eq!(pos, 0.0);
    }

    #[test]
    fn thumb_pos_to_scroll_round_trips() {
        let viewport = 500.0;
        let content  = 1000.0;
        let track    = 500.0;
        let min_thumb = 20.0;
        let scroll_pos = 200.0;

        let (thumb_size, thumb_pos) =
            compute_thumb_metrics(viewport, content, scroll_pos, track, min_thumb);
        let recovered = thumb_pos_to_scroll(thumb_pos, track, thumb_size, content, viewport);
        assert!((recovered - scroll_pos).abs() < 0.01, "round-trip: {recovered}");
    }

    // ── Carousel ─────────────────────────────────────────────────────────

    fn carousel_ctx(index: usize, count: usize, loop_nav: bool) -> Context {
        Context {
            index: Bindable::uncontrolled(index),
            slide_count: NonZero::new(count).expect("count must be > 0"),
            loop_nav,
            auto_play: None,
            auto_play_stopped: false,
            auto_play_paused: false,
            spacing: 0.0,
            slides_per_view: 1.0,
            align: SlideAlignment::Start,
            orientation: Orientation::Horizontal,
            transition_duration: Duration::from_millis(300),
            drag_start_pos: None,
            drag_delta: 0.0,
            swipe_threshold: 50.0,
            swipe_velocity: 0.0,
            drag_last_timestamp: None,
            is_rtl: false,
            ids: ComponentIds::from_id("test-carousel"),
        }
    }

    #[test]
    fn carousel_loop_prev_wraps() {
        let ctx = carousel_ctx(0, 3, true);
        assert_eq!(ctx.clamp_index(-1), 2);
    }

    #[test]
    fn carousel_loop_next_wraps() {
        let ctx = carousel_ctx(2, 3, true);
        assert_eq!(ctx.clamp_index(3), 0);
    }

    #[test]
    fn carousel_no_loop_clamps_at_boundaries() {
        let ctx = carousel_ctx(0, 3, false);
        assert_eq!(ctx.clamp_index(-1), 0, "cannot go before 0");
        let ctx2 = carousel_ctx(2, 3, false);
        assert_eq!(ctx2.clamp_index(3), 2, "cannot go past last");
    }

    #[test]
    fn carousel_can_go_prev_no_loop() {
        assert!(!carousel_ctx(0, 3, false).can_go_prev());
        assert!(carousel_ctx(1, 3, false).can_go_prev());
    }

    #[test]
    fn carousel_can_go_next_no_loop() {
        assert!(carousel_ctx(0, 3, false).can_go_next());
        assert!(!carousel_ctx(2, 3, false).can_go_next());
    }

    // ── Stack ────────────────────────────────────────────────────────────

    #[test]
    fn stack_direction_ltr_logical() {
        assert_eq!(StackDirection::RowLogical.resolve(false), StackDirection::Row);
    }

    #[test]
    fn stack_direction_rtl_logical() {
        assert_eq!(StackDirection::RowLogical.resolve(true), StackDirection::RowReverse);
    }

    #[test]
    fn stack_direction_physical_unchanged_in_rtl() {
        assert_eq!(StackDirection::Column.resolve(true), StackDirection::Column);
    }

    // ── AspectRatio ──────────────────────────────────────────────────────

    #[test]
    fn aspect_ratio_16_9() {
        let props = aspect_ratio::Props { id: String::new(), ratio: 16.0 / 9.0 };
        let pct = props.padding_top_percent();
        assert!((pct - 56.25).abs() < 0.01, "16:9 = 56.25%: {pct}");
    }

    #[test]
    fn aspect_ratio_square() {
        let props = aspect_ratio::Props { id: String::new(), ratio: 1.0 };
        assert!((props.padding_top_percent() - 100.0).abs() < 0.01);
    }

    #[test]
    fn aspect_ratio_portrait() {
        let props = aspect_ratio::Props { id: String::new(), ratio: 2.0 / 3.0 };
        let pct = props.padding_top_percent();
        assert!((pct - 150.0).abs() < 0.01, "2:3 = 150%: {pct}");
    }
}
```

## Appendix D: Feature Flag Map

```toml
# crates/ars-core/Cargo.toml

[features]
default = [
    "splitter",
    "scroll-area",
    "carousel",
    "aspect-ratio",
    "stack",
    "center",
    "grid",
    "portal",
    "collapsible",
    "toolbar",
    "frame",
]

splitter     = []
scroll-area  = []
carousel     = []
aspect-ratio = []
stack        = []
center       = []
grid         = []
portal       = []
collapsible  = []
toolbar      = []
frame        = []
```

Each feature gates the corresponding module. Unused components are removed
by the linker, ensuring consumers pay only for what they use.
