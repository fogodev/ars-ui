# DOM Utilities (`ars-dom`)

## 1. Overview

The `ars-dom` crate provides browser DOM utilities shared by all framework adapters (Leptos, Dioxus). It serves as the convergence layer between the framework-agnostic core crates (`ars-core`, `ars-a11y`, `ars-interactions`) and the actual browser DOM. Components never touch `web_sys` directly — they delegate through `ars-dom` APIs for positioning, focus management, scroll control, z-index allocation, portal management, modality tracking, media queries, and URL sanitization.

`ars-dom` depends on: `ars-core`, `ars-a11y`, `ars-i18n`, `ars-interactions`, `web-sys`, `js-sys`. It is always `std`-enabled because `web-sys` requires it. The crate exposes two feature flags: `web` (default, enables the DOM-backed APIs that expose raw `web_sys` types) and `ssr` (server-side rendering support for the cross-build subset of `ars-dom`).

**Feature-surface rule:** any public API that mentions `web_sys` types in its signature is **web-only** and may be omitted entirely when the `web` feature is disabled. Only the cross-build subset of `ars-dom` participates in the `ssr` contract, where functions return safe defaults or no-op instead of touching the DOM. Adapters and components must treat raw DOM-typed helpers as browser-only APIs and gate their usage accordingly.

On `wasm32` targets the crate uses `thread_local!` with `Cell`/`RefCell` for mutable global state, consistent with the library's single-threaded WASM-first design. On native targets (Dioxus Desktop), most utilities follow the same pattern, with one exception: Scroll Locking (§5.3) uses `AtomicU32`/`Mutex` where thread safety is needed because Dioxus Desktop runs the event loop and rendering on separate threads.

The sections below are organized by functional area. Each section documents the public API, implementation details, and the components that depend on it.

---

## 2. Positioning Engine

### 2.1 Overview

The positioning engine computes where to place floating elements (popovers, tooltips, menus, selects, comboboxes, date pickers, hover cards) relative to their anchor/trigger elements. It runs in `ars-dom` and provides a pure-data API that frameworks wire into their rendering.

The engine takes two rectangles — the anchor element and the floating element — along with a set of positioning options, and returns the exact coordinates where the floating element should be placed. All collision detection and overflow handling is performed against a boundary (typically the viewport).

### 2.2 Types

```rust
// ars-dom/src/positioning.rs

/// The four sides of a rectangle, used for placement direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side { Top, Right, Bottom, Left }

/// Cross-axis alignment within a placement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alignment { Start, Center, End }

/// Placement options for floating elements relative to their anchor.
/// The first word is the side of the anchor the floating element appears on.
/// The optional second word is the alignment along that side.
///
/// **Auto variants**: `Auto`, `AutoStart`, `AutoEnd` let the positioning engine
/// choose the side with the most available space. The optional alignment
/// (`Start`/`End`) is preserved after the side is resolved.
///
/// **Logical variants**: `Start`, `End`, `StartTop`, `StartBottom`, `EndTop`,
/// `EndBottom` resolve based on `ctx.dir` (LTR: Start->Left, RTL: Start->Right).
/// This enables correct popover/tooltip placement in bidirectional layouts without
/// requiring the consumer to check text direction manually.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Placement {
    #[default]
    Bottom,
    BottomStart,
    BottomEnd,
    Top,
    TopStart,
    TopEnd,
    Left,
    LeftStart,
    LeftEnd,
    Right,
    RightStart,
    RightEnd,
    /// Positioning engine chooses the side with the most available space.
    Auto,
    /// Auto side selection, aligned to the start of that side.
    AutoStart,
    /// Auto side selection, aligned to the end of that side.
    AutoEnd,
    /// Logical inline-start side (LTR: Left, RTL: Right).
    Start,
    /// Logical inline-end side (LTR: Right, RTL: Left).
    End,
    /// Logical inline-start side, aligned to the top.
    StartTop,
    /// Logical inline-start side, aligned to the bottom.
    StartBottom,
    /// Logical inline-end side, aligned to the top.
    EndTop,
    /// Logical inline-end side, aligned to the bottom.
    EndBottom,
}

impl Placement {
    /// Returns the opposite placement (used by the flip algorithm).
    /// Handles both resolved and logical variants. Logical Start/End are swapped.
    /// Auto returns Auto (identity).
    pub fn opposite(&self) -> Placement {
        match self {
            Placement::Bottom      => Placement::Top,
            Placement::BottomStart => Placement::TopStart,
            Placement::BottomEnd   => Placement::TopEnd,
            Placement::Top         => Placement::Bottom,
            Placement::TopStart    => Placement::BottomStart,
            Placement::TopEnd      => Placement::BottomEnd,
            Placement::Left        => Placement::Right,
            Placement::LeftStart   => Placement::RightStart,
            Placement::LeftEnd     => Placement::RightEnd,
            Placement::Right       => Placement::Left,
            Placement::RightStart  => Placement::LeftStart,
            Placement::RightEnd    => Placement::LeftEnd,
            Placement::Start       => Placement::End,
            Placement::End         => Placement::Start,
            Placement::StartTop    => Placement::EndTop,
            Placement::StartBottom => Placement::EndBottom,
            Placement::EndTop      => Placement::StartTop,
            Placement::EndBottom   => Placement::StartBottom,
            // Auto variants have no meaningful opposite — return as-is.
            Self::Auto | Self::AutoStart | Self::AutoEnd => *self,
        }
    }

    /// Returns the main axis for this placement (Horizontal or Vertical).
    /// Auto variants default to Axis::Vertical; the engine resolves the actual axis.
    pub fn main_axis(&self) -> Axis {
        match self {
            // Physical horizontal + logical inline-start/end variants.
            Placement::Left | Placement::LeftStart | Placement::LeftEnd
            | Placement::Right | Placement::RightStart | Placement::RightEnd
            | Placement::Start | Placement::End
            | Placement::StartTop | Placement::StartBottom
            | Placement::EndTop | Placement::EndBottom => Axis::Horizontal,
            // Physical vertical + auto variants (auto defaults to vertical).
            Placement::Top | Placement::TopStart | Placement::TopEnd
            | Placement::Bottom | Placement::BottomStart | Placement::BottomEnd
            | Placement::Auto | Placement::AutoStart | Placement::AutoEnd => Axis::Vertical,
        }
    }

    /// Resolves logical (Start/End) and Auto variants into physical placements.
    /// Logical variants resolve based on `dir` (LTR: Start->Left, RTL: Start->Right).
    /// Auto variants are resolved by the positioning engine based on available space
    /// and are not handled here — they pass through unchanged.
    pub fn resolve_logical(&self, dir: ResolvedDirection) -> Placement {
        match (self, dir) {
            (Placement::Start, ResolvedDirection::Ltr)
            | (Placement::End, ResolvedDirection::Rtl) => Placement::Left,
            (Placement::Start, ResolvedDirection::Rtl)
            | (Placement::End, ResolvedDirection::Ltr) => Placement::Right,
            (Placement::StartTop, ResolvedDirection::Ltr)
            | (Placement::EndTop, ResolvedDirection::Rtl) => Placement::LeftStart,
            (Placement::StartTop, ResolvedDirection::Rtl)
            | (Placement::EndTop, ResolvedDirection::Ltr) => Placement::RightStart,
            (Placement::StartBottom, ResolvedDirection::Ltr)
            | (Placement::EndBottom, ResolvedDirection::Rtl) => Placement::LeftEnd,
            (Placement::StartBottom, ResolvedDirection::Rtl)
            | (Placement::EndBottom, ResolvedDirection::Ltr) => Placement::RightEnd,
            (other, _) => *other,
        }
    }

    /// Returns the primary side for this placement.
    /// Logical (Start/End) and Auto variants should be resolved first via
    /// `resolve_logical()` — calling `side()` on them returns `Side::Bottom` as fallback.
    pub fn side(&self) -> Side {
        match self {
            Placement::Top | Placement::TopStart | Placement::TopEnd => Side::Top,
            Placement::Left | Placement::LeftStart | Placement::LeftEnd => Side::Left,
            Placement::Right | Placement::RightStart | Placement::RightEnd => Side::Right,
            // Bottom variants + Auto/Logical fallback to Side::Bottom.
            _ => Side::Bottom,
        }
    }

    /// The cross axis alignment (Start, Center, End).
    /// Returns `Alignment::Center` for unaligned placements (Top, Bottom, Left, Right).
    pub fn alignment(&self) -> Alignment {
        match self {
            Self::TopStart | Self::BottomStart | Self::LeftStart | Self::RightStart
            | Self::AutoStart | Self::StartTop | Self::EndTop => Alignment::Start,
            Self::TopEnd | Self::BottomEnd | Self::LeftEnd | Self::RightEnd
            | Self::AutoEnd | Self::StartBottom | Self::EndBottom => Alignment::End,
            _ => Alignment::Center,
        }
    }

    /// Extract the side and alignment as a tuple.
    /// Auto/Logical variants return `(Side::Bottom, alignment)` — resolve first.
    pub fn side_and_alignment(&self) -> (Side, Alignment) {
        (self.side(), self.alignment())
    }

    /// Return a new Placement with the same alignment but a different side.
    /// Only works on resolved (physical) placements.
    pub fn with_side(&self, new_side: Side) -> Placement {
        let alignment = self.alignment();
        match (new_side, alignment) {
            (Side::Top, Alignment::Start) => Self::TopStart,
            (Side::Top, Alignment::End) => Self::TopEnd,
            (Side::Top, Alignment::Center) => Self::Top,
            (Side::Bottom, Alignment::Start) => Self::BottomStart,
            (Side::Bottom, Alignment::End) => Self::BottomEnd,
            (Side::Bottom, Alignment::Center) => Self::Bottom,
            (Side::Left, Alignment::Start) => Self::LeftStart,
            (Side::Left, Alignment::End) => Self::LeftEnd,
            (Side::Left, Alignment::Center) => Self::Left,
            (Side::Right, Alignment::Start) => Self::RightStart,
            (Side::Right, Alignment::End) => Self::RightEnd,
            (Side::Right, Alignment::Center) => Self::Right,
        }
    }
}

impl fmt::Display for Placement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Top => f.write_str("top"),
            Self::TopStart => f.write_str("top-start"),
            Self::TopEnd => f.write_str("top-end"),
            Self::Bottom => f.write_str("bottom"),
            Self::BottomStart => f.write_str("bottom-start"),
            Self::BottomEnd => f.write_str("bottom-end"),
            Self::Left => f.write_str("left"),
            Self::LeftStart => f.write_str("left-start"),
            Self::LeftEnd => f.write_str("left-end"),
            Self::Right => f.write_str("right"),
            Self::RightStart => f.write_str("right-start"),
            Self::RightEnd => f.write_str("right-end"),
            Self::Auto => f.write_str("auto"),
            Self::AutoStart => f.write_str("auto-start"),
            Self::AutoEnd => f.write_str("auto-end"),
            Self::Start => f.write_str("start"),
            Self::End => f.write_str("end"),
            Self::StartTop => f.write_str("start-top"),
            Self::StartBottom => f.write_str("start-bottom"),
            Self::EndTop => f.write_str("end-top"),
            Self::EndBottom => f.write_str("end-bottom"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

/// Overflow distances on each side relative to the clipping boundary.
/// Positive values indicate overflow beyond the boundary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Overflow {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

/// Offset applied after initial position computation.
/// `main_axis` pushes along the placement direction (e.g., further from anchor).
/// `cross_axis` shifts perpendicular to the placement direction.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Offset {
    pub main_axis: f64,
    pub cross_axis: f64,
}

/// Full set of options controlling how a floating element is positioned.
///
/// Note: `PartialEq` is derived even though this struct contains `f64` fields.
/// Rust's `f64` implements `PartialEq` (but not `Eq`) — `NaN != NaN`.
/// For Dioxus memoization and signal equality checks, this is acceptable because
/// positioning options are always finite values in practice.
#[derive(Debug, Clone, PartialEq)]
pub struct PositioningOptions {
    /// Desired placement relative to the anchor.
    pub placement: Placement,

    /// Distance offset from the anchor edge.
    pub offset: Offset,

    /// When true, automatically flip to the opposite side if the floating element
    /// overflows the boundary on the main axis.
    pub flip: bool,

    /// When true, shift the floating element along the cross axis to keep it
    /// within the boundary while maintaining the anchor connection.
    pub shift: bool,

    /// Minimum distance from viewport edge when shifting (cross-axis padding).
    pub shift_padding: f64,

    /// When true, allow the floating element to overlap the anchor.
    pub overlap: bool,

    /// The container that defines the overflow boundary.
    pub boundary: Boundary,

    /// Padding from the clipping boundary (applied on all sides during overflow detection).
    pub boundary_padding: f64,

    /// CSS positioning strategy for the floating element.
    pub strategy: Strategy,

    /// Layout direction for resolving logical placements (Start/End) to physical
    /// placements (Left/Right). Default: LTR.
    pub dir: ResolvedDirection,

    /// Min distance from arrow to floating element edge.
    pub arrow_padding: f64,

    /// When true, automatically constrain `max-width`/`max-height` so the floating
    /// element does not exceed the available space within the boundary.
    pub auto_max_size: bool,

    /// Optional list of fallback placements to try (in order) when the primary
    /// placement overflows. When this is non-empty, the flip step tries each
    /// fallback before falling back to the direct opposite.
    pub fallback_placements: Vec<Placement>,

    /// When true, reposition on virtual keyboard open/close via visualViewport events.
    /// Enable for floating elements containing or triggered by input fields. Default: false.
    pub keyboard_aware: bool,

    /// When true, try all 12 placements and select the one with the least overflow.
    /// When enabled, the `flip` option is ignored (auto placement already considers
    /// all placements). The `shift` step still applies after auto placement.
    /// This is the most expensive positioning mode — use sparingly.
    ///
    /// `auto_placement: true` is the broad "consider all 12 physical placements"
    /// mode. Separately, `placement: Auto | AutoStart | AutoEnd` enables the
    /// narrower 4-candidate auto modes even when `auto_placement` is `false`.
    pub auto_placement: bool,
}

impl Default for PositioningOptions {
    fn default() -> Self {
        PositioningOptions {
            placement: Placement::default(),
            offset: Offset::default(),
            flip: true,
            shift: true,
            shift_padding: 5.0,
            overlap: false,
            boundary: Boundary::default(),
            boundary_padding: 8.0,
            strategy: Strategy::default(),
            dir: ResolvedDirection::Ltr,
            arrow_padding: 8.0,
            auto_max_size: true,
            fallback_placements: Vec::new(),
            keyboard_aware: false,
            auto_placement: false,
        }
    }
}

/// CSS positioning strategy.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Strategy {
    /// Position relative to the nearest positioned ancestor.
    #[default]
    Absolute,
    /// Position relative to the viewport (ignores scroll).
    ///
    /// **WARNING: CSS Containment Breakage.** `position: fixed` is positioned relative
    /// to the viewport ONLY when no ancestor creates a containing block. The following
    /// CSS properties on ANY ancestor break fixed positioning, causing the element to
    /// be positioned relative to that ancestor instead:
    /// `transform`, `perspective`, `filter`, `backdrop-filter`,
    /// `will-change: transform|perspective|filter`, `contain: paint|layout|strict`.
    ///
    /// When using `Strategy::Fixed`, ensure the floating element is a direct child of
    /// `<body>` or `ars-portal-root`, and that Portal target ancestors do NOT have
    /// these properties. See Step 0 (Detect Containing Block) and Portal §10 in
    /// `components/layout/portal.md` for runtime detection.
    Fixed,
}

/// The boundary against which overflow is detected.
/// `ElementRef` is a platform-conditional type-erased smart pointer (`Rc<dyn Any>` on
/// WASM, `Arc<dyn Any>` on native) defined in `ars-dom/src/types.rs`.
/// See `shared/layout-shared-types.md` §1 for the full definition. Wrap in
/// `DomElementRef` for access to the underlying `web_sys::Element`.
#[derive(Debug, Clone, Default)]
pub enum Boundary {
    /// Use the viewport as the overflow boundary (default).
    #[default]
    Viewport,
    /// Use a specific element's bounding rect as the overflow boundary.
    Element(ElementRef),
}

impl PartialEq for Boundary {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Viewport, Self::Viewport) => true,
            #[cfg(target_arch = "wasm32")]
            (Self::Element(a), Self::Element(b)) => Rc::ptr_eq(a, b),
            #[cfg(not(target_arch = "wasm32"))]
            (Self::Element(a), Self::Element(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

/// The computed result of positioning a floating element.
/// After computing the result, the adapter MUST write
/// `data-ars-placement={actual_placement}` onto the floating element
/// (see `00-overview.md §3` for the canonical attribute name).
/// CSS uses this attribute to orient arrow elements.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PositioningResult {
    /// Final x coordinate for the floating element.
    pub x: f64,
    /// Final y coordinate for the floating element.
    pub y: f64,
    /// The actual placement after flip/shift adjustments.
    /// May differ from the requested placement if flipping occurred.
    pub actual_placement: Placement,
    /// Arrow x position along the floating element edge.
    pub arrow_x: Option<f64>,
    /// Arrow y position along the floating element edge.
    pub arrow_y: Option<f64>,
    /// The maximum height of the floating element (set when `auto_max_size` is enabled).
    pub max_height: Option<f64>,
    /// The maximum width of the floating element (set when `auto_max_size` is enabled).
    pub max_width: Option<f64>,
}

/// A simple rectangle with position and dimensions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn right(&self) -> f64 { self.x + self.width }
    pub fn bottom(&self) -> f64 { self.y + self.height }
    pub fn center_x(&self) -> f64 { self.x + self.width / 2.0 }
    pub fn center_y(&self) -> f64 { self.y + self.height / 2.0 }
}
```

#### 2.2.1 ResizeObserver Lifecycle for Positioning

The Positioning Engine MUST use `ResizeObserver` to detect size changes in both the reference element and the floating element, triggering repositioning when dimensions change.

**Observer Creation**: Each call to `auto_update()` MUST create a `ResizeObserver` instance observing both the reference and floating elements:

See `auto_update()` in §3.5 for the canonical Rust implementation with `ResizeObserver`, `requestAnimationFrame` debouncing, and cleanup.

**Cleanup**: The observer MUST be disconnected via `observer.disconnect()` when:

- The floating element is hidden or unmounted
- `auto_update()` cleanup function is called
- The component is unmounted

**Performance**: Use a single `ResizeObserver` instance per `auto_update()` call with multiple observed elements, rather than creating one observer per element. The `requestAnimationFrame` debounce ensures at most one reposition per frame during rapid resizes (e.g., window resize, content reflow).

**SSR Safety**: `ResizeObserver` creation MUST be gated behind `#[cfg(target_arch = "wasm32")]`. During SSR, `auto_update()` returns a no-op cleanup function.

**Window Resize**: In addition to `ResizeObserver`, `auto_update()` MUST listen to the `resize` event on `window` for viewport-level size changes that don't trigger element-level resize observations.

**Scroll Container Detection**: `auto_update()` MUST attach `scroll` event listeners (with `{ passive: true, capture: true }`) on all scrollable ancestors of the anchor element, not just the nearest one. Use the `scrollable_ancestors()` utility from §4.2 to collect the full ancestor chain.

#### 2.2.2 Visual Viewport and Virtual Keyboard

On mobile browsers, the virtual keyboard reduces the visible area without changing `window.innerHeight` (Safari) or changes `window.innerHeight` but not `document.documentElement.clientHeight` (Chrome). The positioning engine MUST account for this.

**Requirements:**

1. **`auto_update()` MUST subscribe to `visualViewport` events** when `window.visualViewport` is available:

    ```rust
    // In ars-dom auto_update implementation:
    if let Some(vv) = window.visual_viewport() {
        // Subscribe to resize (keyboard open/close) and scroll (keyboard pan)
        vv.add_event_listener_with_callback("resize", reposition_callback.as_ref().unchecked_ref())
            .expect("addEventListener on VisualViewport");
        vv.add_event_listener_with_callback("scroll", reposition_callback.as_ref().unchecked_ref())
            .expect("addEventListener on VisualViewport");
        // Cleanup must remove both listeners
    }
    ```

2. **Boundary height MUST use `visualViewport.height`** when `Boundary::Viewport` is active and the visual viewport API is available. Fallback to `window.innerHeight` when unavailable:

    ```rust
    fn viewport_width(window: &web_sys::Window) -> f64 {
        let width = window.visual_viewport()
            .map(|vv| vv.width())
            .unwrap_or_else(|| {
                window.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(0.0)
            });
        #[cfg(feature = "debug")]
        if width == 0.0 {
            log::warn!(
                "[ars-dom] viewport_width() returned 0.0; window may not be fully initialized"
            );
        }
        width
    }

    fn viewport_height(window: &web_sys::Window) -> f64 {
        let height = window.visual_viewport()
            .map(|vv| vv.height())
            .unwrap_or_else(|| {
                window.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(0.0)
            });
        #[cfg(feature = "debug")]
        if height == 0.0 {
            log::warn!(
                "[ars-dom] viewport_height() returned 0.0; window may not be fully initialized"
            );
        }
        height
    }

    fn viewport_rect(window: &web_sys::Window) -> Rect {
        if let Some(vv) = window.visual_viewport() {
            Rect {
                x: vv.offset_left(),
                y: vv.offset_top(),
                width: vv.width(),
                height: vv.height(),
            }
        } else {
            Rect {
                x: 0.0,
                y: 0.0,
                width: window.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(0.0),
                height: window.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(0.0),
            }
        }
    }
    ```

    `viewport_width()` and `viewport_height()` expose only the effective dimensions.
    `viewport_rect()` is the API that carries the visual viewport origin via
    `offsetLeft` / `offsetTop` when the browser exposes it.

3. **`keyboard_aware` positioning option**: Add to `PositioningOptions`:

    ```rust
    /// When true, reposition floating elements in response to virtual keyboard
    /// open/close events (visualViewport resize). Default: false.
    /// Enable for floating elements that contain or are triggered by input fields.
    pub keyboard_aware: bool, // default: false
    ```

4. **Scroll lock adjustment**: Dialog scroll lock (see `components/overlay/dialog.md`) MUST account for `visualViewport.offsetTop` when computing the scroll position to preserve. On iOS Safari, `visualViewport.offsetTop` reflects the amount the page has been scrolled to accommodate the keyboard.

5. **SSR Safety**: All `visualViewport` access MUST be gated behind
   `#[cfg(all(feature = "web", target_arch = "wasm32"))]`. Host and SSR builds
   MUST keep the fallback logic in pure helpers or internal stubs rather than
   exposing browser-only types in non-web public APIs.

### 2.3 Algorithm

The positioning algorithm runs in six sequential steps. Each step may modify the working coordinates before passing them to the next step.

#### 2.3.1 Step 0: Detect Containing Block

Before measuring, check whether the floating element has an ancestor that
creates a containing block (via `transform`, `perspective`, `will-change`,
`filter`, or `contain: paint`). When `position: absolute` is used inside such
an ancestor, `getBoundingClientRect()` returns coordinates relative to the
viewport but the element is positioned relative to the containing block — so
the raw viewport-relative values must be adjusted.

> **Containing Block Detection Checklist**
> The following CSS properties on ancestor elements create a new containing block for
> `position:fixed` descendants, causing the floating element to be positioned relative to
> the ancestor instead of the viewport:
>
> - `transform` (any non-`none` value)
> - `perspective` (any non-`none` value)
> - `filter` (any non-`none` value)
> - `backdrop-filter` (any non-`none` value)
> - `contain: layout` or `contain: paint` (or shorthand `contain: strict`/`content`)
> - `content-visibility: auto`
> - `will-change` referencing any of the above properties

```rust
/// Check whether the floating element has an ancestor that creates a
/// containing block for absolutely-positioned descendants.
///
/// CSS properties that create a containing block:
/// - `transform` (any value other than `none`)
/// - `perspective` (any value other than `none`)
/// - `will-change: transform | perspective | filter`
/// - `filter` (any value other than `none`)
/// - `backdrop-filter` (any value other than `none`)
/// - `contain: layout` or `contain: paint` (or shorthand `contain: strict`/`content`)
/// - `content-visibility: auto`
/// - `container-type: size` or `container-type: inline-size` (CSS Container Queries)
/// **Dev-mode runtime warning**: In debug builds, `detect_containing_block()` SHOULD
/// also be applied to Portal target elements (not just floating elements). If the
/// Portal target has a containing-block ancestor, emit a console warning:
/// "Portal target has ancestor with [property] which breaks position:fixed positioning.
///  Overlays may be mispositioned. Move the portal target outside this ancestor."
///
fn find_containing_block_ancestor(element: &Element) -> Option<Rect> {
    // See `has_containing_block_ancestor()` in §2.5 for the bool-returning version.
    // This variant returns the ancestor's bounding rect for coordinate adjustment.
    let window = web_sys::window().expect("window must exist in browser context");
    let mut current = element.parent_element();
    while let Some(el) = current {
        let style = window.get_computed_style(&el)
            .expect("get_computed_style available in browser")
            .expect("style object is non-null for attached elements");
        let transform = style.get_property_value("transform").unwrap_or_default();
        let perspective = style.get_property_value("perspective").unwrap_or_default();
        let filter = style.get_property_value("filter").unwrap_or_default();
        let contain = style.get_property_value("contain").unwrap_or_default();
        let container_type = style.get_property_value("container-type").unwrap_or_default();
        let will_change = style.get_property_value("will-change").unwrap_or_default();
        let backdrop_filter = style.get_property_value("backdrop-filter").unwrap_or_default();
        let content_visibility = style.get_property_value("content-visibility").unwrap_or_default();

        if (transform != "none" && !transform.is_empty())
            || (perspective != "none" && !perspective.is_empty())
            || (filter != "none" && !filter.is_empty())
            || contain.contains("paint") || contain.contains("layout")
            || contain.contains("strict") || contain.contains("content")
            || (container_type != "normal" && !container_type.is_empty())
            || will_change.contains("transform") || will_change.contains("perspective")
            || will_change.contains("filter") || will_change.contains("backdrop-filter")
            || (!backdrop_filter.is_empty() && backdrop_filter != "none")
            || content_visibility == "auto"
        {
            let rect = el.get_bounding_client_rect();
            return Some(Rect {
                x: rect.x(),
                y: rect.y(),
                width: rect.width(),
                height: rect.height(),
            });
        }
        current = el.parent_element();
    }
    None
}
```

If a containing-block ancestor is found, its bounding rect is subtracted from
all subsequent coordinate calculations (anchor rect, viewport rect) so the
final `(x, y)` values are relative to the correct coordinate space.

#### 2.3.2 Step 1: Measure

Read the anchor element's bounding client rect and the floating element's dimensions from the DOM. Both are represented as `Rect` values. The viewport `Rect` is also measured (or provided explicitly in tests).

```rust
let anchor_dom = anchor.get_bounding_client_rect();
let anchor_rect = Rect { x: anchor_dom.x(), y: anchor_dom.y(),
    width: anchor_dom.width(), height: anchor_dom.height() };
let floating_dom = floating.get_bounding_client_rect();
let floating_rect = Rect { x: 0.0, y: 0.0, // only width/height used
    width: floating_dom.width(), height: floating_dom.height() };
let window = web_sys::window().expect("window must exist in browser context");
let viewport = viewport_rect(&window); // see §2.2.2 for visual viewport preference
```

> **Visual Viewport on Mobile:** `window.innerWidth`/`innerHeight` return the
> _layout viewport_ dimensions, which do **not** change during pinch-to-zoom on
> mobile browsers. After a pinch-zoom, the visible area (the _visual viewport_)
> is smaller than the layout viewport, so floating elements positioned against
> `innerWidth`/`innerHeight` may overflow the screen. When available, prefer
> `window.visualViewport.width` / `window.visualViewport.height` plus
> `visualViewport.offsetLeft` / `offsetTop` for the viewport rect. This means
> `viewport_rect().x` and `.y` track the visible origin, while
> `viewport_width()` / `viewport_height()` continue to report dimensions only.
> This ensures
> the floating element stays within the user-visible area even while zoomed.
> Feature-detect with `if window.visual_viewport.is_some()` and fall back to
> `innerWidth`/`innerHeight` when the API is absent (e.g., older WebViews).

Before overflow, shift, or max-size calculations, resolve the effective
clipping boundary from `options.boundary` and apply `boundary_padding` by
insetting that rect equally on all four sides. Inset means moving the origin
inward by `boundary_padding` on both axes and subtracting `2 * boundary_padding`
from width and height. Clamp the resulting width/height at zero if padding is
larger than the boundary itself.

#### 2.3.3 Step 2: Compute Initial Position

Based on the requested `placement`, compute the initial `(x, y)` so the floating element sits at the correct side and alignment of the anchor.

```text
match placement:
  Bottom      → x = anchor.center_x - floating.width/2,  y = anchor.bottom
  BottomStart → x = anchor.x,                             y = anchor.bottom
  BottomEnd   → x = anchor.right - floating.width,        y = anchor.bottom
  Top         → x = anchor.center_x - floating.width/2,  y = anchor.y - floating.height
  TopStart    → x = anchor.x,                             y = anchor.y - floating.height
  TopEnd      → x = anchor.right - floating.width,        y = anchor.y - floating.height
  Right       → x = anchor.right,                         y = anchor.center_y - floating.height/2
  RightStart  → x = anchor.right,                         y = anchor.y
  RightEnd    → x = anchor.right,                         y = anchor.bottom - floating.height
  Left        → x = anchor.x - floating.width,            y = anchor.center_y - floating.height/2
  LeftStart   → x = anchor.x - floating.width,            y = anchor.y
  LeftEnd     → x = anchor.x - floating.width,            y = anchor.bottom - floating.height
```

#### 2.3.4 Step 3: Apply Offset

Add the configured offset to the working coordinates.

```text
if main_axis is Vertical:
    y += offset.main_axis * sign  // sign is +1 for Bottom, -1 for Top
    x += offset.cross_axis
if main_axis is Horizontal:
    x += offset.main_axis * sign  // sign is +1 for Right, -1 for Left
    y += offset.cross_axis
```

`main_axis` is always interpreted as "push away from the anchor on the chosen
side". `cross_axis` is always interpreted as "slide along the aligned edge".
The sign flip simply converts those logical directions into viewport-space `x`
and `y` deltas.

#### 2.3.5 Step 4: Flip (if enabled)

Check whether the floating element overflows the boundary on the main axis. If so, evaluate fallback placements and keep the candidate with the lowest total overflow.

```text
if options.flip:
    overflow_main = compute_overflow(x, y, floating, boundary, placement)
    if overflow_main > 0:
        candidates = options.fallback_placements
        candidates += [placement.opposite()]  // always include direct opposite as final fallback
        evaluate each candidate in order
        choose the candidate with the lowest total overflow
        if best_candidate improves on the current placement:
            (x, y) = best_candidate_coords
            actual_placement = best_candidate
```

> **NOTE:** Explicit `fallback_placements` are tried first and preserve caller preference order, but the engine still appends the direct opposite placement as a final recovery candidate if it was not already present. This ensures the basic opposite-side fallback remains available even when custom fallbacks are configured.

#### 2.3.6 Step 5: Shift (if enabled)

Check whether the floating element overflows the boundary on the cross axis. If so, clamp its position to stay within bounds.

```text
if options.shift:
    if main_axis is Vertical:
        // Clamp x so floating stays within boundary horizontally
        x = clamp(x, boundary.x, boundary.right - floating.width)
    if main_axis is Horizontal:
        // Clamp y so floating stays within boundary vertically
        y = clamp(y, boundary.y, boundary.bottom - floating.height)
```

Shift never changes the main-axis coordinate. A `Top` placement that moves
vertically would no longer be on the top side of the anchor; the same applies
to `Left`/`Right` placements on the horizontal axis.

#### 2.3.7 Step 6: Arrow

If arrow padding is configured (`arrow_padding > 0`), compute the arrow's position along the cross axis of the floating element. The arrow is clamped so that it never extends beyond the floating element's edges minus the arrow padding. The arrow position is expressed as `(arrow_x, arrow_y)` where the axis perpendicular to the placement side receives the computed value and the other axis is `None`.

#### 2.3.8 Step 7: Auto Max Size

If `auto_max_size` is enabled, compute `max_width` and `max_height` constraints so the floating element does not exceed the available space within the boundary. The available space on the main axis is measured from the floating element's current position to the boundary edge on the placement side. On the cross axis, use the full clipped boundary extent so the floating element may still span the boundary's available width or height after placement.

#### 2.3.9 Step 8: Return

Return the `PositioningResult` with the final `(x, y)` coordinates and the `actual_placement` (which may differ from the requested placement if a flip occurred in step 4).

### 2.4 Core Function Signature

```rust
// ars-dom/src/positioning.rs

/// Calculate overlay position relative to an anchor element.
///
/// Algorithm:
/// 1. **Place** — compute initial (x, y) from placement + offsets.
/// 2. **Offset** — apply main/cross axis offsets.
/// 3. **Flip** — if floating overflows the clipping boundary on the main axis,
///    try the opposite placement. If both overflow, use the one with more space.
/// 4. **Shift** — if floating overflows the clipping boundary on the cross axis,
///    slide it along that axis to fit, respecting `shift_padding`.
/// 5. **Arrow** — compute arrow position clamped within the floating element,
///    respecting `arrow_padding` from edges.
/// 6. **Max dimensions** — if `auto_max_size` is set, constrain max-width/max-height
///    so the floating element does not exceed the available space.
pub fn compute_position(
    anchor: &Rect,
    floating: &Rect,
    viewport: &Rect,
    options: &PositioningOptions,
) -> PositioningResult {
    // 0. Resolve logical placement (Start/End) to physical (Left/Right) using dir
    let placement = options.placement.resolve_logical(options.dir);

    // 1. Place — compute initial (x, y) from placement + offsets
    let (mut x, mut y) = compute_coords(anchor, floating, placement);

    // 2. Apply offset
    let (off_x, off_y) = resolve_offset(options.offset.main_axis, options.offset.cross_axis, placement);
    x += off_x;
    y += off_y;

    // 3. Flip — if floating overflows the clipping boundary on the main axis,
    //    try fallback placements (or the direct opposite) and keep the best.
    let mut final_placement = placement;
    if options.flip {
        let overflow = detect_overflow(x, y, floating, viewport);
        if should_flip(&overflow, placement) {
            let primary_total = total_overflow(&overflow);
            let mut best_total = primary_total;
            let mut best_x = x;
            let mut best_y = y;
            let mut best_placement = placement;

            // Build candidate list: explicit fallbacks first, then the direct opposite
            // as a final recovery candidate if it is not already present.
            let mut candidates = options.fallback_placements.clone();
            let opposite = flip_placement(placement);
            if !candidates.contains(&opposite) {
                candidates.push(opposite);
            }

            for candidate in &candidates {
                let (cx, cy) = compute_coords(anchor, floating, *candidate);
                let (co_x, co_y) = resolve_offset(
                    options.offset.main_axis, options.offset.cross_axis, *candidate,
                );
                let candidate_overflow = detect_overflow(cx + co_x, cy + co_y, floating, viewport);
                let candidate_total = total_overflow(&candidate_overflow);
                if candidate_total < best_total {
                    best_total = candidate_total;
                    best_x = cx + co_x;
                    best_y = cy + co_y;
                    best_placement = *candidate;
                }
                if candidate_total == 0.0 {
                    break; // perfect fit — no need to check remaining candidates
                }
            }

            if best_placement != placement {
                x = best_x;
                y = best_y;
                final_placement = best_placement;
            }
        }
    }

    // 4. Shift — slide along the cross axis to stay within the boundary
    if options.shift {
        let overflow = detect_overflow(x, y, floating, viewport);
        let (sx, sy) = compute_shift(&overflow, floating, viewport);
        x += sx;
        y += sy;
    }

    // 5. Arrow — compute arrow position clamped within the floating element
    let (arrow_x, arrow_y) = if options.arrow_padding > 0.0 {
        compute_arrow_position(anchor, x, y, floating, final_placement, options.arrow_padding)
    } else {
        (None, None)
    };

    // 6. Max dimensions — constrain so floating does not exceed available space
    let (max_width, max_height) = if options.auto_max_size {
        compute_max_size(x, y, viewport, final_placement)
    } else {
        (None, None)
    };

    PositioningResult {
        x,
        y,
        actual_placement: final_placement,
        arrow_x,
        arrow_y,
        max_height,
        max_width,
    }
}
```

### 2.5 Overflow Detection and Helpers

```rust
/// Compute overflow on all four sides relative to the clipping boundary.
/// Positive values indicate overflow beyond the boundary.
fn detect_overflow(x: f64, y: f64, floating: &Rect, viewport: &Rect) -> Overflow {
    Overflow {
        top:    viewport.y - y,
        right:  (x + floating.width) - (viewport.x + viewport.width),
        bottom: (y + floating.height) - (viewport.y + viewport.height),
        left:   viewport.x - x,
    }
}

/// Determine whether the floating element should flip to the opposite side.
/// Returns `true` if overflow on the main axis exceeds the threshold.
fn should_flip(overflow: &Overflow, placement: Placement) -> bool { /* ... */ }

/// Sum of all positive overflow values — used to compare flip candidates.
fn total_overflow(overflow: &Overflow) -> f64 { /* ... */ }

/// Compute the shift (x, y) needed to slide the floating element along the
/// cross axis so it stays within the viewport boundary.
fn compute_shift(overflow: &Overflow, floating: &Rect, viewport: &Rect) -> (f64, f64) { /* ... */ }

/// Compute the arrow position clamped within the floating element,
/// respecting `arrow_padding` from edges.
///
/// The arrow sits on the edge closest to the anchor, center-aligned with
/// the anchor by default. It is clamped to `[arrow_padding, extent - arrow_padding]`
/// along the cross axis. Implementations should additionally account for
/// `border_radius` when clamping (i.e., the effective minimum offset is
/// `max(arrow_padding, border_radius + arrow_size / 2.0)`) to prevent the
/// arrow from overlapping rounded corners.
fn compute_arrow_position(
    anchor: &Rect, x: f64, y: f64, floating: &Rect,
    placement: Placement, arrow_padding: f64,
) -> (Option<f64>, Option<f64>) {
    let (side, _alignment) = placement.side_and_alignment();

    match side {
        // Vertical placements (Top, Bottom): arrow slides along the x-axis
        Side::Top | Side::Bottom => {
            // Center of anchor relative to the floating element's left edge
            let center = (anchor.x + anchor.width / 2.0) - x;
            let min = arrow_padding;
            let max = floating.width - arrow_padding;
            let arrow_x = center.clamp(min, max);
            (Some(arrow_x), None)
        }
        // Horizontal placements (Left, Right): arrow slides along the y-axis
        Side::Left | Side::Right => {
            // Center of anchor relative to the floating element's top edge
            let center = (anchor.y + anchor.height / 2.0) - y;
            let min = arrow_padding;
            let max = floating.height - arrow_padding;
            let arrow_y = center.clamp(min, max);
            (None, Some(arrow_y))
        }
    }
}

/// Compute maximum width/height so the floating element does not exceed
/// available space in the viewport for the given placement.
///
/// The main-axis dimension is constrained by the space remaining on the chosen
/// side of the placement. The cross-axis dimension uses the full clipped
/// boundary extent.
fn compute_max_size(
    x: f64, y: f64, viewport: &Rect, placement: Placement,
) -> (Option<f64>, Option<f64>) { /* ... */ }

/// Resolve main/cross offsets into (x, y) deltas based on the placement direction.
fn resolve_offset(offset_main: f64, offset_cross: f64, placement: Placement) -> (f64, f64) { /* ... */ }

/// Compute initial (x, y) coordinates for the given placement using
/// side + alignment decomposition.
fn compute_coords(anchor: &Rect, floating: &Rect, placement: Placement) -> (f64, f64) {
    let (side, alignment) = placement.side_and_alignment();

    let x = match side {
        Side::Top | Side::Bottom => match alignment {
            Alignment::Start  => anchor.x,
            Alignment::End    => anchor.x + anchor.width - floating.width,
            Alignment::Center => anchor.x + anchor.width / 2.0 - floating.width / 2.0,
        },
        Side::Left  => anchor.x - floating.width,
        Side::Right => anchor.x + anchor.width,
    };

    let y = match side {
        Side::Top    => anchor.y - floating.height,
        Side::Bottom => anchor.y + anchor.height,
        Side::Left | Side::Right => match alignment {
            Alignment::Start  => anchor.y,
            Alignment::End    => anchor.y + anchor.height - floating.height,
            Alignment::Center => anchor.y + anchor.height / 2.0 - floating.height / 2.0,
        },
    };

    (x, y)
}

/// Convenience wrapper: callers may also use `placement.opposite()` directly.
fn flip_placement(placement: Placement) -> Placement {
    placement.opposite()
}
```

`compute_coords()` is easiest to read as two independent decisions:

- choose the cross-axis coordinate from the requested alignment (`Start`,
  `Center`, `End`)
- choose the main-axis coordinate from the requested side (`Top`, `Bottom`,
  `Left`, `Right`)

`overlap` mode keeps the same alignment math but swaps the main-axis coordinate
so the floating rect shares the anchor edge instead of sitting fully outside it.

### 2.6 Virtual Elements

`compute_position()` accepts `anchor: &Rect`, so it already supports positioning relative to arbitrary rectangles instead of DOM elements. A `VirtualElement` helper provides a consistent interface for dynamic rect sources:

```rust
/// A non-DOM anchor for positioning. The `get_rect` callback returns the
/// current client-space bounding rect of the virtual anchor.
pub struct VirtualElement {
    /// Returns the current bounding rectangle in client-space coordinates.
    pub get_rect: Box<dyn Fn() -> Rect>,
}
```

**Usage example — context menu at cursor position:**

```rust
let cursor_rect = Rect { x: cursor_x, y: cursor_y, width: 0.0, height: 0.0 };
let result = compute_position(&cursor_rect, &menu_rect, &viewport, &options);
```

**Auto-update limitation:** `auto_update()` accepts `web_sys::Element` parameters and uses `ResizeObserver` / scroll listeners to detect geometry changes. Virtual elements cannot be observed this way. When using a virtual element, the consumer must call `compute_position()` manually whenever the virtual rect changes (e.g., on `mousemove` for context menus, on selection change for text selection anchors).

### 2.7 Auto Placement

Auto placement tries all 12 physical placement options and selects the one with the least overflow. This is the most expensive positioning mode and should be used sparingly (e.g., context menus where the ideal direction is unpredictable).

```rust
/// Try all 12 placements and return the one with the least total overflow.
/// Breaks early if a placement has zero overflow (perfect fit).
fn compute_auto_placement(
    anchor: &Rect,
    floating: &Rect,
    viewport: &Rect,
    options: &PositioningOptions,
) -> Placement {
    const ALL_PLACEMENTS: [Placement; 12] = [
        Placement::Top,       Placement::TopStart,    Placement::TopEnd,
        Placement::Bottom,    Placement::BottomStart, Placement::BottomEnd,
        Placement::Left,      Placement::LeftStart,   Placement::LeftEnd,
        Placement::Right,     Placement::RightStart,  Placement::RightEnd,
    ];

    let mut best_placement = ALL_PLACEMENTS[0];
    let mut best_overflow = f64::MAX;

    for &candidate in &ALL_PLACEMENTS {
        let (cx, cy) = compute_coords(anchor, floating, candidate);
        let (ox, oy) = resolve_offset(options.offset.main_axis, options.offset.cross_axis, candidate);
        let overflow = detect_overflow(cx + ox, cy + oy, floating, viewport);
        let total = total_overflow(&overflow);

        if total < best_overflow {
            best_overflow = total;
            best_placement = candidate;
        }
        if total == 0.0 {
            break; // perfect fit — no need to check remaining placements
        }
    }

    best_placement
}
```

To use auto placement, set `auto_placement: true` in `PositioningOptions`. When enabled, the algorithm runs `compute_auto_placement()` before the flip step to determine the initial placement:

```rust
// In compute_position(), before step 1:
let placement = if options.auto_placement {
    compute_auto_placement(anchor, floating, viewport, &options)
} else {
    options.placement.resolve_logical(options.dir)
};
```

When `auto_placement` is `true`, the `flip` option is ignored (auto placement already considers all placements). The `shift` step still applies after auto placement to handle cross-axis overflow.

When `placement` itself is `Auto`, `AutoStart`, or `AutoEnd`, the engine also
enters auto-placement mode even if `auto_placement` is `false`:

- `Auto` evaluates the 4 centered sides (`Top`, `Bottom`, `Left`, `Right`)
- `AutoStart` evaluates the 4 `*Start` placements
- `AutoEnd` evaluates the 4 `*End` placements

If `auto_placement` is `true`, it overrides these narrower modes and evaluates
all 12 physical placements.

### 2.8 Coordinate System

All positioning calculations use a consistent coordinate system. This section documents the conventions and edge cases.

- **Client-space coordinates.** All coordinates are in client-space (viewport-relative) obtained via `getBoundingClientRect()`. Both the anchor rect and the viewport rect use this coordinate space.

- **Subpixel rounding.** `getBoundingClientRect()` returns fractional pixel values (e.g., `{ top: 100.5, left: 200.25 }`). When applying computed coordinates as CSS `top`/`left` values, round to the nearest integer (`Math.round()`) for standard positioning. Use `Math.ceil()` for width/height calculations where conservative (overlap-avoiding) sizing is needed. Fractional pixels can cause 1px gaps or overlaps between the floating element and its anchor due to browser rounding differences. For high-DPI displays, consider rounding to the nearest `0.5px` instead of full integers to reduce visual jitter.

- **Positioning strategy.** The positioned floating element uses `position: fixed; top: Y; left: X` with client-space coordinates by default (`Strategy::Fixed`). This avoids scroll-offset calculations since `getBoundingClientRect()` already returns viewport-relative values.

- **CSS transform containing blocks.** If a parent element has CSS transforms (`transform`, `perspective`, `will-change: transform`), it creates a new containing block for `position: fixed` descendants. When this occurs, the floating element's `top`/`left` are interpreted relative to the transformed parent, not the viewport. The positioning engine MUST detect this case (see `has_containing_block_ancestor()` in §2.3 Step 0) and convert coordinates from client-space to the transformed parent's local space by subtracting the parent's `getBoundingClientRect()` origin. Alternatively, avoid CSS transforms on positioning containers entirely.

- **`position: absolute` positioning (non-portal).** When the floating element uses `Strategy::Absolute` (e.g., it is not portaled and lives inside a positioned ancestor), subtract the offset parent's `getBoundingClientRect()` from the client-space coordinates to produce values relative to the offset parent:

    ```rust
    let offset_parent_rect = offset_parent.get_bounding_client_rect();
    let local_x = client_x - offset_parent_rect.x();
    let local_y = client_y - offset_parent_rect.y();
    // Apply: element.style.left = local_x; element.style.top = local_y;
    ```

- **Recalculation triggers.** Recalculate the floating element's position on:
    1. **Scroll events** — any ancestor scroll container between the anchor and the boundary.
    2. **Resize events** — `window` resize and `ResizeObserver` on both the reference (anchor) and floating elements.
    3. **`requestAnimationFrame`** — for smooth updates during CSS animations or transitions that change the anchor's geometry.
    4. **Layout mutations** — when the anchor or floating element's size changes (detected via `ResizeObserver`).

    The adapter SHOULD debounce scroll/resize recalculations using `requestAnimationFrame` to avoid layout thrashing. At most one position update per animation frame.

#### 2.8.1 CSS Transform Ancestor Detection

The positioning engine must detect CSS `transform`, `perspective`, or `will-change: transform` on ancestor elements using `getComputedStyle()`. When detected, coordinates are converted from the transformed ancestor's local coordinate space to viewport space. Adapters should cache ancestor transform detection per positioning cycle and invalidate on resize/scroll.

### 2.9 Usage by Components

The following components use the positioning engine to place their floating content:

| Component    | Default Placement | Uses Flip | Uses Shift | Notes                                            |
| ------------ | ----------------- | --------- | ---------- | ------------------------------------------------ |
| `Popover`    | `Bottom`          | Yes       | Yes        | General purpose floating content                 |
| `Tooltip`    | `Top`             | Yes       | Yes        | Small, text-only; tight offset                   |
| `Menu`       | `BottomStart`     | Yes       | Yes        | Submenu uses `RightStart` / `LeftStart` for RTL  |
| `Select`     | `BottomStart`     | Yes       | Yes        | Listbox positioned below trigger                 |
| `Combobox`   | `BottomStart`     | Yes       | Yes        | Same as Select but with text input as anchor     |
| `HoverCard`  | `Bottom`          | Yes       | Yes        | Richer content than Tooltip                      |
| `DatePicker` | `BottomStart`     | Yes       | Yes        | Calendar panel below input                       |
| `Toast`      | (none)            | No        | No         | Optional; typically uses fixed viewport position |

> **Accessibility note:** Repositioning (flip, shift) does not affect ARIA relationships. The anchor element MUST have `aria-controls` pointing to the floating element's ID — **except for Tooltip**, which uses `aria-describedby` per WAI-ARIA APG (the tooltip describes the trigger rather than being a controlled widget). Components using the positioning engine are responsible for maintaining the appropriate ARIA link. Screen readers rely on this relationship, not visual positioning.

### 2.10 Auto-Update

The `auto_update()` function keeps a floating element positioned correctly as the page
changes. It mirrors the semantics of [Floating UI's `autoUpdate`](https://floating-ui.com/docs/autoUpdate).

**Recalculation triggers:**

| Trigger                | Mechanism                                         | Rationale                                          |
| ---------------------- | ------------------------------------------------- | -------------------------------------------------- |
| Window resize          | `resize` event on `window`                        | Viewport dimensions changed                        |
| Scroll                 | `scroll` event on every scroll-ancestor           | Anchor may have moved relative to the viewport     |
| Anchor/floating resize | `ResizeObserver` on both elements                 | Element dimensions changed (e.g., content update)  |
| DOM mutation           | `MutationObserver` on the anchor's parent element | Content changes that affect size                   |
| Anchor clipping        | `IntersectionObserver` on anchor                  | Anchor scrolled behind `overflow: hidden` ancestor |

**Throttle strategy (adapter-level):** Adapters SHOULD wrap the `update` callback in a
`requestAnimationFrame` guard so multiple triggers within the same frame coalesce into one
`compute_position()` call, preventing layout thrashing. The core `auto_update()` implementation
calls `update()` directly from each observer; RAF batching is the adapter's responsibility.

```rust
// ars-dom/src/positioning.rs

/// Set up automatic repositioning when the anchor or viewport changes.
/// Returns a cleanup function that stops observation.
///
/// Uses `ResizeObserver` on the anchor and floating elements,
/// scroll listeners on all scroll-ancestor elements, and
/// a `MutationObserver` for DOM structure changes.
pub fn auto_update(
    anchor: &web_sys::HtmlElement,
    floating: &web_sys::HtmlElement,
    update: impl Fn() + 'static,
) -> Box<dyn FnOnce()> {
    let update = Rc::new(update);
    let mut cleanups: Vec<Box<dyn FnOnce()>> = Vec::new();

    // 1. ResizeObserver — recompute when either element resizes
    let update_ro = update.clone();
    let resize_cb = Closure::wrap(Box::new(move |_entries: js_sys::Array, _: web_sys::ResizeObserver| {
        update_ro();
    }) as Box<dyn FnMut(js_sys::Array, web_sys::ResizeObserver)>);
    let resize_observer = web_sys::ResizeObserver::new(resize_cb.as_ref().unchecked_ref())
        .expect("ResizeObserver constructor should not throw");
    resize_observer.observe(anchor);
    resize_observer.observe(floating);
    let ro = resize_observer.clone();
    cleanups.push(Box::new(move || { ro.disconnect(); drop(resize_cb); }));

    // 2. Scroll listeners — recompute when any ancestor scrolls
    let update_scroll = update.clone();
    let scroll_cb = Closure::wrap(Box::new(move |_: web_sys::Event| {
        update_scroll();
    }) as Box<dyn FnMut(web_sys::Event)>);

    let scroll_parents: Vec<web_sys::Element> = scrollable_ancestors(&anchor);
    for parent in &scroll_parents {
        let opts = web_sys::AddEventListenerOptions::new();
        opts.set_passive(true);
        opts.set_capture(true);
        parent.add_event_listener_with_callback_and_add_event_listener_options(
            "scroll", scroll_cb.as_ref().unchecked_ref(), &opts,
        ).expect("addEventListener should not throw for scroll");
    }
    let window = web_sys::window().expect("window must exist in browser context");
    window.add_event_listener_with_callback("scroll", scroll_cb.as_ref().unchecked_ref())
        .expect("addEventListener should not throw for scroll");
    window.add_event_listener_with_callback("resize", scroll_cb.as_ref().unchecked_ref())
        .expect("addEventListener should not throw for resize");

    let scroll_fn_ref = scroll_cb.as_ref().unchecked_ref::<js_sys::Function>().clone();
    cleanups.push(Box::new(move || {
        for parent in &scroll_parents {
            parent.remove_event_listener_with_callback("scroll", &scroll_fn_ref)
                .expect("removeEventListener should not throw for scroll");
        }
        let window = web_sys::window().expect("window must exist in browser context");
        window.remove_event_listener_with_callback("scroll", &scroll_fn_ref)
            .expect("removeEventListener should not throw for scroll");
        window.remove_event_listener_with_callback("resize", &scroll_fn_ref)
            .expect("removeEventListener should not throw for resize");
        drop(scroll_cb);
    }));

    // 3. MutationObserver — recompute when DOM structure changes around the anchor
    let update_mo = update.clone();
    let mutation_cb = Closure::wrap(Box::new(move |_: js_sys::Array, _: web_sys::MutationObserver| {
        update_mo();
    }) as Box<dyn FnMut(js_sys::Array, web_sys::MutationObserver)>);
    let mutation_observer = web_sys::MutationObserver::new(mutation_cb.as_ref().unchecked_ref())
        .expect("MutationObserver constructor should not throw");
    let mut opts = web_sys::MutationObserverInit::new();
    opts.set_child_list(true);
    opts.set_subtree(true);
    opts.set_attributes(true);
    opts.set_attribute_filter(&js_sys::Array::of2(&"style".into(), &"class".into()));
    if let Some(parent) = anchor.parent_element() {
        mutation_observer.observe_with_options(&parent, &opts)
            .expect("MutationObserver.observe should not throw for valid options");
    }
    let mo = mutation_observer.clone();
    cleanups.push(Box::new(move || { mo.disconnect(); drop(mutation_cb); }));

    // 4. IntersectionObserver — detect when the anchor is clipped by an
    //    overflow: hidden ancestor (scrolled out of view). When the anchor
    //    is fully clipped (intersection ratio = 0), hide the floating element.
    //    When it re-enters, trigger an update to reposition.
    let update_io = update.clone();
    let floating_el = floating.clone();
    let intersection_cb = Closure::wrap(Box::new(move |entries: js_sys::Array, _: web_sys::IntersectionObserver| {
        if let Some(entry) = entries.get(0).dyn_ref::<web_sys::IntersectionObserverEntry>() {
            if entry.intersection_ratio() == 0.0 {
                // Anchor is fully clipped — hide the floating element.
                let _ = floating_el.style().set_property("visibility", "hidden");
            } else {
                // Anchor is (partially) visible — show and reposition.
                let _ = floating_el.style().remove_property("visibility");
                update_io();
            }
        }
    }) as Box<dyn FnMut(js_sys::Array, web_sys::IntersectionObserver)>);
    let mut io_opts = web_sys::IntersectionObserverInit::new();
    io_opts.threshold(&js_sys::Array::of1(&JsValue::from_f64(0.0)));
    let intersection_observer = web_sys::IntersectionObserver::new_with_options(
        intersection_cb.as_ref().unchecked_ref(), &io_opts,
    ).expect("IntersectionObserver constructor should not throw");
    intersection_observer.observe(anchor);
    let io = intersection_observer.clone();
    cleanups.push(Box::new(move || { io.disconnect(); drop(intersection_cb); }));

    // Initial position update
    update();

    Box::new(move || {
        for cleanup in cleanups {
            cleanup();
        }
    })
}
```

### 2.10 Native Popover API Integration

When using the native HTML `popover` attribute with ars-ui components, the adapter MUST:

1. Listen for the `toggle` event on the popover element and sync `newState` back to the state machine via `Event::Close` when the browser dismisses the popover.
2. Call `element.showPopover()` / `element.hidePopover()` from effects when the machine transitions, rather than relying on attribute presence alone.
3. Use `popover='manual'` (not `popover='auto'`) when the state machine handles dismiss logic, to avoid conflicting light-dismiss behavior.

If `popover='auto'` is desired, the state machine MUST be the follower, not the leader — browser events drive state.

---

## 3. Focus Utilities

### 3.1 Element Querying

```rust
/// Raw selector used to collect focusable DOM candidates before filtering.
/// Includes elements with `tabindex="-1"` because they remain programmatically focusable.
pub const FOCUSABLE_SELECTOR: &str = concat!(
    "button:not([disabled]):not([aria-hidden='true']),",
    "input:not([disabled]):not([aria-hidden='true']),",
    "select:not([disabled]):not([aria-hidden='true']),",
    "textarea:not([disabled]):not([aria-hidden='true']),",
    "a[href]:not([aria-hidden='true']),",
    "area[href]:not([aria-hidden='true']),",
    "[tabindex]:not([disabled]):not([aria-hidden='true']),",
    "[contenteditable]:not([contenteditable='false']):not([aria-hidden='true'])",
);

/// Raw selector used to collect tabbable DOM candidates before filtering.
pub const TABBABLE_SELECTOR: &str = concat!(
    "button:not([disabled]):not([tabindex='-1']):not([aria-hidden='true']),",
    "input:not([disabled]):not([tabindex='-1']):not([aria-hidden='true']),",
    "select:not([disabled]):not([tabindex='-1']):not([aria-hidden='true']),",
    "textarea:not([disabled]):not([tabindex='-1']):not([aria-hidden='true']),",
    "a[href]:not([tabindex='-1']):not([aria-hidden='true']),",
    "area[href]:not([tabindex='-1']):not([aria-hidden='true']),",
    "[tabindex]:not([tabindex='-1']):not([disabled]):not([aria-hidden='true']),",
    "[contenteditable]:not([contenteditable='false']):not([tabindex='-1']):not([aria-hidden='true'])",
);

/// Return the selector used to collect tabbable DOM candidates.
pub fn get_tabbable_elements_selector() -> &'static str {
    TABBABLE_SELECTOR
}

/// Query focusable elements within a container.
pub fn get_focusable_elements(container: &web_sys::Element) -> Vec<web_sys::HtmlElement> {
    let nodes = container.query_selector_all(FOCUSABLE_SELECTOR)
        .expect("valid hardcoded CSS selector");

    let window = web_sys::window().expect("window exists in browser context");

    let mut elements = Vec::new();

    for i in 0..nodes.length() {
        if let Some(el) = nodes.get(i).and_then(|n| n.dyn_into::<web_sys::HtmlElement>().ok()) {
            // Filter out hidden or inert elements
            let style = window.get_computed_style(&el)
                .expect("element supports computed style")
                .expect("style object returned");

            let display = style.get_property_value("display").unwrap_or_default();
            let visibility = style.get_property_value("visibility").unwrap_or_default();
            if display != "none" && visibility != "hidden" && !el.has_attribute("inert") {
                elements.push(el);
            }
        }
    }
    elements
}

/// Get the first/last focusable element.
/// Delegates to `get_focusable_elements` to ensure hidden/inert filtering is applied.
pub fn get_first_focusable(container: &web_sys::Element) -> Option<web_sys::HtmlElement> {
    get_focusable_elements(container).into_iter().next()
}

pub fn get_last_focusable(container: &web_sys::Element) -> Option<web_sys::HtmlElement> {
    get_focusable_elements(container).into_iter().last()
}
```

### 3.2 Focus Management

```rust
/// Focus an element, optionally preventing scroll.
pub fn focus_element(el: &web_sys::HtmlElement, prevent_scroll: bool) {
    let opts = web_sys::FocusOptions::new();
    opts.set_prevent_scroll(prevent_scroll);
    el.focus_with_options(&opts).ok();
}

/// Look up a DOM element by ID and downcast to `HtmlElement`.
/// Returns `None` if the element is not found or is not an `HtmlElement`.
pub fn get_html_element_by_id(id: &str) -> Option<web_sys::HtmlElement> {
    web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(id))
        .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok())
}

/// Focus an element by its DOM ID. No-op if the element is not found or not focusable.
/// This is the primary focus API used by component effects (e.g., restoring focus after
/// a popover closes, moving focus to a newly activated tab, etc.).
pub fn focus_element_by_id(id: &str) {
    if let Some(el) = get_html_element_by_id(id) {
        focus_element(&el, false);
    }
}

/// Announce a message to screen readers with polite priority.
///
/// Resolves the `LiveAnnouncer` from the adapter's framework context internally.
/// No-op during SSR or when no `LiveAnnouncerProvider` is in the component tree.
///
/// This is the primary announcement API used by component effects (e.g., announcing
/// a new month after calendar navigation, announcing sort state changes in tables).
/// Components call `ars_dom::announce(&msg)` — they never manage the `LiveAnnouncer`
/// instance directly.
pub fn announce(message: &str) {
    if let Some(announcer) = use_announcer() {
        announcer.borrow_mut().announce(message);
    }
}

/// Announce a message with assertive priority (interrupts current screen reader speech).
/// Use sparingly — unexpected interruptions degrade UX significantly.
pub fn announce_assertive(message: &str) {
    if let Some(announcer) = use_announcer() {
        announcer.borrow_mut().announce_assertive(message);
    }
}

/// Adapter-provided context resolution for the LiveAnnouncer.
/// Each framework adapter implements this via its own context system:
///   - Leptos: `use_context::<Rc<RefCell<LiveAnnouncer>>>()`
///   - Dioxus: `try_consume_context::<Rc<RefCell<LiveAnnouncer>>>()`
/// Returns `None` if no `LiveAnnouncerProvider` is mounted (SSR, tests without provider).
fn use_announcer() -> Option<Rc<RefCell<LiveAnnouncer>>> {
    // Adapter-specific implementation.
    // The adapter's root component (or ArsProvider) mounts a
    // LiveAnnouncerProvider that provides Rc<RefCell<LiveAnnouncer>> via context.
    use_context::<Rc<RefCell<LiveAnnouncer>>>()
}

/// Focus the first tabbable element inside a container identified by ID.
/// Used by modal dialogs to move focus into the dialog content on open.
/// No-op if the container is not found or contains no tabbable elements.
pub fn focus_first_tabbable(container_id: &str) {
    if let Some(container) = get_html_element_by_id(container_id) {
        let tabbables = get_tabbable_elements(&container);
        if let Some(first) = tabbables.first() {
            focus_element(first, false);
        }
    }
}

/// Focus `document.body` as a last-resort fallback (e.g., when a dialog's
/// trigger element was removed from the DOM while the dialog was open).
pub fn focus_body() {
    if let Some(body) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.body())
    {
        focus_element(&body, false);
    }
}

/// Check whether an element is still connected to the document.
/// Returns `false` if the element was removed from the DOM.
/// Used to guard focus-restore logic (e.g., only restore focus to a trigger
/// if it's still in the document).
pub fn document_contains(el: &web_sys::HtmlElement) -> bool {
    web_sys::window()
        .and_then(|w| w.document())
        .map(|d| d.contains(Some(el.as_ref())))
        .unwrap_or(false)
}

/// Best-effort direct inert clearing for siblings of the element identified
/// by `portal_id`.
///
/// This helper removes `inert` from sibling elements without access to the
/// state captured by `set_background_inert()`. It is therefore suitable for
/// stack recalculation and defensive cleanup, but it does NOT replace the
/// cleanup closure returned by `set_background_inert()`. The returned cleanup
/// closure remains the authoritative teardown path for restoring polyfill
/// state such as `aria-hidden`, saved `tabindex` values, and document-level
/// listeners.
pub fn remove_inert_from_siblings(portal_id: &str) {
    let document = match web_sys::window().and_then(|w| w.document()) {
        Some(d) => d,
        None => return,
    };
    let portal = match document.get_element_by_id(portal_id) {
        Some(el) => el,
        None => return,
    };
    if let Some(parent) = portal.parent_element() {
        let children = parent.children();
        for i in 0..children.length() {
            if let Some(sibling) = children.item(i) {
                if sibling != portal {
                    sibling.remove_attribute("inert").ok();
                }
            }
        }
    }
}
```

### 3.3 FocusScope Implementation

> **Note:** The `FocusScopeBehavior` trait, `FocusScopeOptions`, and `FocusTarget` types are defined in `03-accessibility.md` §3.4 (ars-a11y crate). `FocusStrategy` is also defined there but is not needed by the concrete `FocusScope` implementation below.

The concrete `FocusScope` struct lives in **ars-dom** (requires `std` + `web_sys`) and implements the `FocusScopeBehavior` trait from ars-a11y:

```rust
// ars-dom/src/focus/scope.rs

use ars_a11y::{FocusScopeOptions, FocusTarget, FocusScopeBehavior};

/// Manages focus within a bounded DOM region.
///
/// Lifecycle:
///   1. Created with a container element reference and options.
///   2. `activate()` is called when the scope becomes active (e.g., dialog opens).
///   3. `deactivate()` is called when the scope should release focus.
///   4. Implements Drop to deactivate if not already done.
// In ars-dom — implements ars_a11y::FocusScopeBehavior (see impl block below)
pub struct FocusScope {
    options: FocusScopeOptions,
    /// The previously focused element, saved on activation for restoration.
    previously_focused: Option<FocusedElement>,
    /// Whether this scope is currently active.
    active: bool,
    /// The container element. In ars-dom this wraps web_sys::Element.
    container_id: String,
}

/// Platform-agnostic reference to a focused element.
/// Concrete implementations in ars-dom store a web_sys::Element.
pub struct FocusedElement(pub String); // element ID

/// Trait implementation — components interact through `FocusScopeBehavior`.
impl FocusScopeBehavior for FocusScope {
    fn activate(&mut self, focus_target: FocusTarget) {
        FocusScope::activate(self, focus_target);
    }
    fn deactivate(&mut self) {
        FocusScope::deactivate(self);
    }
    fn is_active(&self) -> bool {
        self.active
    }
}

impl FocusScope {
    /// Create a new FocusScope attached to the element with the given ID.
    pub fn new(container_id: impl Into<String>, options: FocusScopeOptions) -> Self {
        Self {
            options,
            previously_focused: None,
            active: false,
            container_id: container_id.into(),
        }
    }

    // ars-dom implementation — requires DOM access (web_sys).
    /// Activate the scope. Saves current focus (for restoration) and
    /// optionally moves focus into the container.
    ///
    /// Called by the component machine when entering the "open" state.
    pub fn activate(&mut self, focus_target: FocusTarget) {
        if self.active { return; }
        // Implementation delegates to ars-dom's DOM focus utilities.
        self.previously_focused = get_currently_focused();
        self.active = true;

        if self.options.auto_focus {
            self.focus_first(focus_target);
        }
    }

    // ars-dom implementation — requires DOM access (web_sys).
    /// Deactivate the scope. Restores focus if configured to do so.
    ///
    /// Called by the component machine when leaving the "open" state.
    ///
    /// **Error recovery**: If `previously_focused` has been removed from the DOM
    /// (e.g., the trigger was conditionally unmounted while the dialog was open),
    /// the implementation MUST NOT silently fail. Fallback chain:
    ///   1. Try to focus `previously_focused`.
    ///   2. If that element is no longer in the DOM, try to focus the FocusScope's
    ///      parent element (the container's parentElement).
    ///   3. If the parent is also unavailable, focus `document.body` as a last resort.
    pub fn deactivate(&mut self) {
        if !self.active { return; }
        self.active = false;

        if self.options.restore_focus {
            if let Some(ref el) = self.previously_focused {
                // Implementation: attempt restore_focus_to(el).
                // If the element is no longer connected to the DOM
                // (el.isConnected == false), fall back:
                //   1. Focus the container's parent element.
                //   2. If unavailable, focus document.body.
                // ars-dom DOM helper signatures (defined in ars-dom/src/focus.rs):
                //   fn is_element_in_dom(el: &FocusedElement) -> bool
                //     — Looks up element by stored ID, returns el.isConnected.
                //   fn focus_element(el: &FocusedElement)
                //     — Looks up element by ID and calls .focus().
                //   fn focus_body()
                //     — Calls document.body.focus().
                //   fn get_parent_element(container_id: &str) -> Option<FocusedElement>
                //     — Returns FocusedElement wrapping the parent element's ID.
                if !ars_dom::is_element_in_dom(el) {
                    // ID-based focus: look up the parent element by container ID,
                    // resolve to an HtmlElement, then focus it.
                    if let Some(parent_el) = ars_dom::get_element_by_id(&self.container_id)
                        .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok())
                        .and_then(|el| el.parent_element())
                        .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok())
                    {
                        ars_dom::focus_element(&parent_el, false);
                    } else {
                        // Last resort: focus the document body
                        if let Some(body) = web_sys::window()
                            .and_then(|w| w.document())
                            .and_then(|d| d.body())
                        {
                            let _ = body.focus();
                        }
                    }
                } else if let Some(html_el) = ars_dom::get_element_by_id(&el.0)
                    .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
                {
                    ars_dom::focus_element(&html_el, false);
                }
            }
        }
    }

    // ars-dom implementation — requires DOM access (web_sys).
    /// Handle a keydown event for Tab/Shift+Tab containment.
    ///
    /// Returns `true` if the event was handled (and should be prevented).
    /// Call this from the container element's keydown handler when `contain` is true.
    pub fn handle_tab_key(&self, shift: bool) -> bool {
        if !self.options.contain || !self.active { return false; }

        // Implementation in ars-dom:
        // 1. Get all tabbable elements within container (sorted by tabindex, then DOM order)
        // 2. Find current focus position in that list
        // 3. If at boundary (first/last), wrap to the other end
        // 4. Return true to signal prevent_default()

        true // placeholder
    }

    // ars-dom implementation — requires DOM access (web_sys).
    /// Move focus to the first tabbable element in the scope.
    pub fn focus_first(&self, target: FocusTarget) {
        let _ = target;
        // ars-dom: get_tabbable_elements(container).first().map(focus_element)
    }

    // ars-dom implementation — requires DOM access (web_sys).
    /// Move focus to the last tabbable element in the scope.
    pub fn focus_last(&self) {
        // ars-dom: get_tabbable_elements(container).last().map(focus_element)
    }

    // ars-dom implementation — requires DOM access (web_sys).
    /// Check if an element is within this scope's container.
    pub fn contains_element(&self, element_id: &str) -> bool {
        // ars-dom: container.contains(element)
        let _ = element_id;
        false // placeholder
    }
}

impl Drop for FocusScope {
    fn drop(&mut self) {
        self.deactivate();
    }
}
```

---

## 4. Scroll Management

### 4.1 Scroll Into View

The scroll-management APIs in this section operate on raw DOM elements and are therefore part of the **web-only** surface of `ars-dom`. They are available when the `web` feature is enabled and are not part of the `ssr` stub contract.

```rust
/// Options for scrolling an element into view within a scrollable container.
pub struct ScrollIntoViewOptions {
    /// Where to align the element vertically within the scrollable area.
    pub block: Option<ScrollLogicalPosition>,
    /// Where to align the element horizontally within the scrollable area.
    pub inline: Option<ScrollLogicalPosition>,
    /// Whether to use smooth scrolling.
    pub smooth: bool,
}

/// Logical scroll positions for alignment within a scrollable container.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScrollLogicalPosition {
    Start,
    Center,
    End,
    Nearest,
}

/// Scroll an element into view within a scrollable container when needed.
pub fn scroll_into_view_if_needed(element: &web_sys::Element, options: ScrollIntoViewOptions) {
    let rect = element.get_bounding_client_rect();
    let parent = nearest_scrollable_ancestor(element);

    match parent {
        Some(scrollable) => {
            let parent_rect = scrollable.get_bounding_client_rect();
            let block = options.block.unwrap_or(ScrollLogicalPosition::Nearest);

            let target_top = match block {
                ScrollLogicalPosition::Start => {
                    rect.top() - parent_rect.top() + scrollable.scroll_top() as f64
                }
                ScrollLogicalPosition::End => {
                    rect.bottom() - parent_rect.bottom() + scrollable.scroll_top() as f64
                }
                ScrollLogicalPosition::Center => {
                    let offset = rect.top() - parent_rect.top()
                        - (parent_rect.height() - rect.height()) / 2.0;
                    scrollable.scroll_top() as f64 + offset
                }
                ScrollLogicalPosition::Nearest => {
                    if rect.top() < parent_rect.top() {
                        // Element above viewport — scroll up
                        rect.top() - parent_rect.top() + scrollable.scroll_top() as f64
                    } else if rect.bottom() > parent_rect.bottom() {
                        // Element below viewport — scroll down
                        rect.bottom() - parent_rect.bottom() + scrollable.scroll_top() as f64
                    } else {
                        // Already visible — no scroll needed
                        return;
                    }
                }
            };

            let behavior = if options.smooth {
                web_sys::ScrollBehavior::Smooth
            } else {
                web_sys::ScrollBehavior::Auto
            };
            let mut opts = web_sys::ScrollToOptions::new();
            opts.set_top(target_top);
            opts.set_behavior(behavior);
            scrollable.scroll_to_with_scroll_to_options(&opts);
        }
        None => {
            // Fallback to native scrollIntoView
            element.scroll_into_view_with_bool(true);
        }
    }
}

fn nearest_scrollable_ancestor(element: &web_sys::Element) -> Option<web_sys::Element> {
    let window = web_sys::window().expect("window exists in browser context");
    let mut current = element.parent_element();
    while let Some(el) = current {
        let style = window.get_computed_style(&el)
            .expect("element supports computed style")
            .expect("style object returned");
        let overflow_x = style.get_property_value("overflow-x").unwrap_or_default();
        let overflow_y = style.get_property_value("overflow-y").unwrap_or_default();
        let is_scrollable_axis = |v: &str| v == "auto" || v == "scroll" || v == "hidden";
        let has_overflow =
            (is_scrollable_axis(&overflow_y) && el.scroll_height() > el.client_height())
            || (is_scrollable_axis(&overflow_x) && el.scroll_width() > el.client_width());
        if has_overflow {
            return Some(el);
        }
        current = el.parent_element();
    }
    None
}
```

`Nearest` is intentionally asymmetric: it preserves the current scroll position
when the element is already fully visible, and otherwise moves only by the
minimum delta required to reveal the clipped edge. `Center` instead compares the
element midpoint to the container midpoint and converts that midpoint delta back
into a scroll offset.

### 4.2 Scrollable Ancestor Detection

`auto_update()` MUST attach `scroll` event listeners (with `{ passive: true, capture: true }`) on all scrollable ancestors of the anchor element, not just the nearest one. Use the `scrollable_ancestors()` utility to collect the full ancestor chain:

```rust
// ars-dom/src/scroll.rs

/// Collect ALL scrollable ancestors of the given element, from nearest to furthest.
/// Extends `nearest_scrollable_ancestor()` by walking the full ancestor chain
/// instead of stopping at the first scrollable container.
///
/// An element is considered scrollable if:
///   - `overflow-x` or `overflow-y` is `auto`, `scroll`, or `hidden`
///   - AND `scrollHeight > clientHeight` or `scrollWidth > clientWidth`
///
/// Always includes `document.documentElement` as the outermost scrollable ancestor.
pub fn scrollable_ancestors(element: &Element) -> Vec<Element> {
    let mut ancestors = Vec::new();
    let mut current = nearest_scrollable_ancestor(element);
    while let Some(ancestor) = current {
        ancestors.push(ancestor.clone());
        current = nearest_scrollable_ancestor(&ancestor);
    }
    // Ensure document root is always included
    if let Some(doc_el) = element.owner_document().and_then(|d| d.document_element()) {
        if ancestors.last().map_or(true, |last| *last != doc_el) {
            ancestors.push(doc_el);
        }
    }
    ancestors
}
```

Both checks are required. An element with `overflow: auto` but no extra content
does not need a scroll listener, and an element with overflowing content but
`overflow: visible` will not trap scrolling on that axis.

In `auto_update()`, iterate the result and attach scroll listeners:

```rust
// In ars-dom auto_update implementation:
let scroll_ancestors = scrollable_ancestors(&reference_element);
for ancestor in &scroll_ancestors {
    let mut opts = web_sys::AddEventListenerOptions::new();
    opts.set_passive(true);
    opts.set_capture(true);
    ancestor.add_event_listener_with_callback_and_add_event_listener_options(
        "scroll",
        reposition_callback.as_ref().unchecked_ref(),
        &opts,
    ).expect("addEventListener on scroll ancestor");
}
// Cleanup must remove scroll listeners from all ancestors
```

---

## 5. Scroll Locking

### 5.1 Overview

When a modal overlay (Dialog, AlertDialog, Drawer) opens, page scrolling should be prevented to keep focus within the overlay and avoid confusing visual shifts. The scroll lock system manages a depth counter to handle nested modals correctly: styles are applied when the first modal opens and restored only when the last modal closes.

### 5.2 ScrollLockManager (Reference-Counted)

> **Note:** ScrollLockManager is the high-level API. It delegates to `acquire()`/`release()` (§5.3) internally.

#### 5.2.1 Reference-Counted Scroll Lock

Overlay components (Dialog, Popover, Drawer, DatePicker) often need to prevent background scrolling while open. When multiple overlays are nested (e.g., a DatePicker inside a Dialog), naive scroll locking breaks — closing the inner overlay restores scrolling even though the outer overlay is still open.

The `ScrollLockManager` uses reference counting to ensure scroll locking remains active as long as any overlay holds a lock, and only restores scrolling when the last overlay releases its lock.

```rust
use std::collections::HashSet;  // ars-dom is std-enabled (web-sys dependency); HashSet is valid here

/// Manages document scroll locking with reference counting.
/// Ensures that nested overlays correctly coordinate scroll lock/unlock.
pub struct ScrollLockManager {
    /// Tracks which overlay instances currently own a lock, keyed by overlay ID.
    /// Prevents double-lock from a single overlay and lets the manager answer
    /// whether it currently owns any locks.
    lock_owners: HashSet<String>,
}

impl ScrollLockManager {
    pub fn new() -> Self {
        Self {
            lock_owners: HashSet::new(),
        }
    }

    /// Acquire a scroll lock for the given overlay.
    /// The first lock captures the current scroll position and sets
    /// `overflow: hidden` on the document body.
    /// Subsequent locks increment the counter without changing scroll state.
    /// Duplicate lock requests from the same overlay ID are ignored.
    pub fn lock(&mut self, overlay_id: &str) {
        // Prevent double-lock from the same overlay
        if self.lock_owners.contains(overlay_id) {
            return;
        }
        self.lock_owners.insert(overlay_id.to_string());
        acquire();
    }

    /// Release the scroll lock for the given overlay.
    /// When the last lock is released (count reaches zero), the original
    /// overflow style and scroll position are restored.
    /// Unlock requests for unknown overlay IDs are ignored.
    pub fn unlock(&mut self, overlay_id: &str) {
        if !self.lock_owners.remove(overlay_id) {
            return; // Not locked by this overlay — no-op
        }
        release();
    }

    /// Returns `true` if scroll locking is currently active.
    pub fn is_locked(&self) -> bool {
        !self.lock_owners.is_empty()
    }
}
```

**Integration with overlay machines:**

- Overlay `PendingEffect::setup` calls `ScrollLockManager::lock(overlay_id)` as part of the open effect.
- The cleanup closure returned by `setup` calls `ScrollLockManager::unlock(overlay_id)`.
- The `ScrollLockManager` instance is stored in a thread-local or adapter-level singleton, shared across all overlay instances.
- The DOM snapshot used to restore scroll position and styles is document-global state owned by the low-level `acquire()` / `release()` machinery, not per-manager state. This is required because all overlays coordinate the same `document.body` and viewport.
- Scroll lock timing is an adapter-level concern. Overlays that perform positioning calculations before locking SHOULD schedule the `lock()` call via `requestAnimationFrame` to avoid measuring stale layout dimensions. The `acquire()` / `release()` low-level API applies styles synchronously — the adapter decides when to call them.

### 5.3 Low-Level API (acquire/release with depth counter)

```rust
// ars-dom/src/scroll_lock.rs

/// Manages scroll locking for modal overlays.
///
/// Uses a depth counter to handle nested modals: the first lock applies
/// body style overrides, and the last unlock restores them. Intermediate
/// lock/unlock calls only adjust the counter.
///
// Platform-conditional synchronization primitives.
// WASM targets are single-threaded (no SharedArrayBuffer needed).
// Native targets (Dioxus Desktop) may run components on different threads.
#[cfg(target_arch = "wasm32")]
use std::cell::{Cell, RefCell};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{AtomicU32, Ordering};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Mutex;

/// Process-global scroll lock depth counter.
#[cfg(target_arch = "wasm32")]
thread_local! {
    static SCROLL_LOCK_DEPTH: Cell<u32> = Cell::new(0);
    static SCROLL_LOCK_SAVED: RefCell<Option<ScrollLockSavedState>> = RefCell::new(None);
}

#[cfg(not(target_arch = "wasm32"))]
static SCROLL_LOCK_DEPTH: AtomicU32 = AtomicU32::new(0);

/// Saved state from before scroll lock was first acquired.
/// Protected by Mutex since multiple threads may acquire/release on native.
/// Uses `unwrap_or_else(|e| e.into_inner())` to handle poisoned mutexes gracefully.
#[cfg(not(target_arch = "wasm32"))]
static SCROLL_LOCK_SAVED: Mutex<Option<ScrollLockSavedState>> = Mutex::new(None);

/// **Threading model exception:** ScrollLock uses `AtomicU32` and `Mutex` instead of the
/// crate's usual `Rc<RefCell<>>` pattern because Dioxus Desktop runs the event loop and
/// rendering on separate threads. This is the only interaction primitive that must be thread-safe.
///
/// **Platform constraint:** The DOM calls in `apply_scroll_lock()` and `restore_scroll_state()`
/// use `web_sys::Window`, which is `!Send`. On multi-threaded Dioxus Desktop, these calls MUST
/// run on the main (UI) thread. Guard the DOM-touching paths with `#[cfg(target_arch = "wasm32")]`
/// for WASM targets; on native desktop targets, use the adapter's main-thread dispatch mechanism
/// (e.g., `dioxus::prelude::spawn_on_main_thread`) to ensure DOM access is never invoked from a
/// background thread.

struct ScrollLockSavedState {
    overflow: String,
    padding_right: String,
    scroll_x: f64,              // horizontal scroll position
    scroll_y: f64,              // vertical scroll position (iOS restore)
    body_top: String,           // iOS only
    html_overflow: String,      // tiered strategy — <html> overflow
    overscroll_behavior: String, // tiered strategy — body overscroll-behavior
}

// cfg-gated function bodies: WASM uses Cell/RefCell via thread_local!,
// native uses AtomicU32/Mutex for thread safety.

#[cfg(not(target_arch = "wasm32"))]
pub fn acquire() {
    let prev = SCROLL_LOCK_DEPTH.fetch_add(1, Ordering::SeqCst);
    if prev == 0 {
        let saved = save_current_scroll_state();
        *SCROLL_LOCK_SAVED.lock().unwrap_or_else(|e| e.into_inner()) = Some(saved);
        apply_scroll_lock();
    }
}

#[cfg(target_arch = "wasm32")]
pub fn acquire() {
    SCROLL_LOCK_DEPTH.with(|depth| {
        let prev = depth.get();
        depth.set(prev + 1);
        if prev == 0 {
            let saved = save_current_scroll_state();
            SCROLL_LOCK_SAVED.with(|s| s.borrow_mut().replace(saved));
            apply_scroll_lock();
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn release() {
    // Use fetch_update to prevent underflow (matches WASM's saturating_sub behavior)
    let prev = SCROLL_LOCK_DEPTH.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| {
        Some(n.saturating_sub(1))
    }).unwrap_or(0);
    if prev == 1 {
        if let Some(saved) = SCROLL_LOCK_SAVED.lock().unwrap_or_else(|e| e.into_inner()).take() {
            restore_scroll_state(saved);
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn release() {
    SCROLL_LOCK_DEPTH.with(|depth| {
        let prev = depth.get();
        depth.set(prev.saturating_sub(1));
        if prev == 1 {
            SCROLL_LOCK_SAVED.with(|s| {
                if let Some(saved) = s.borrow_mut().take() {
                    restore_scroll_state(saved);
                }
            });
        }
    });
}

/// Returns true if scroll locking is currently active (depth > 0).
#[cfg(not(target_arch = "wasm32"))]
pub fn is_locked() -> bool {
    SCROLL_LOCK_DEPTH.load(Ordering::SeqCst) > 0
}

#[cfg(target_arch = "wasm32")]
pub fn is_locked() -> bool {
    SCROLL_LOCK_DEPTH.with(|depth| depth.get() > 0)
}

/// Returns the current nesting depth.
#[cfg(not(target_arch = "wasm32"))]
pub fn depth() -> u32 {
    SCROLL_LOCK_DEPTH.load(Ordering::SeqCst)
}

#[cfg(target_arch = "wasm32")]
pub fn depth() -> u32 {
    SCROLL_LOCK_DEPTH.with(|depth| depth.get())
}
```

### 5.4 Public Aliases

```rust
// ars-dom/src/scroll_lock.rs — public API

/// Prevent page scrolling. Call when a modal overlay opens.
/// Alias for `acquire()` — increments the depth counter and applies lock styles
/// on the first call.
pub fn prevent_scroll() { acquire(); }

/// Restore page scrolling. Call when a modal overlay closes.
/// Alias for `release()` — decrements the depth counter and restores styles
/// when the last overlay closes.
pub fn restore_scroll() { release(); }
```

### 5.5 Nested Dialog Stacking

The depth counter ensures that nested modals are handled correctly without double-applying or prematurely removing lock styles:

1. **Dialog A opens** — `lock()` is called, depth becomes 1. Body styles are saved and `overflow: hidden` is applied. Scroll position is preserved.
2. **Dialog B opens inside A** — `lock()` is called, depth becomes 2. No style changes occur (already locked).
3. **Dialog B closes** — `unlock()` is called, depth becomes 1. No style changes occur (still locked for Dialog A).
4. **Dialog A closes** — `unlock()` is called, depth becomes 0. Body styles are restored to their original values. On iOS, the saved scroll position is restored via `window.scrollTo()`.

This model assumes that modals open and close in stack order (LIFO). If a consumer closes modals out of order, the depth counter still works correctly — styles are only restored when all modals have closed.

### 5.6 iOS Quirk

> Note: This iOS-specific behavior is incorporated into the tiered strategy in §5.9.

On iOS Safari, setting `overflow: hidden` on the `<body>` element does not prevent scrolling. The page remains scrollable because iOS Safari handles overflow on the viewport level differently from other browsers.

The workaround is to set `position: fixed` on the body, which removes it from the normal flow and effectively prevents scrolling:

1. **On lock**: Save `window.scrollY`. Set `body { position: fixed; top: -{scrollY}px; width: 100%; overflow: hidden; }`. The negative `top` value keeps the visual position stable so the page does not jump to the top.
2. **On unlock**: Restore the original `position` and `overflow` styles. Remove the `top` and `width` overrides. Call `window.scrollTo(savedScrollX, savedScrollY)` to restore the exact scroll position.

```rust
/// Detect whether the iOS scroll lock workaround is needed.
/// Returns true on iOS Safari and iOS WebView.
fn needs_ios_workaround() -> bool {
    web_sys::window()
        .and_then(|w| {
            let nav = w.navigator();
            let ua = nav.user_agent().ok()?;
            let platform = nav.platform().ok().unwrap_or_default();
            let max_touch = nav.max_touch_points();

            // Direct iOS user-agent detection
            if ua.contains("iPhone") || ua.contains("iPad") || ua.contains("iPod") {
                return Some(true);
            }
            // iOS 13+ iPad reports as MacIntel but has touch support.
            // Use > 1 to match Platform::detect() in 03-accessibility.md — MacBooks report 1.
            if platform == "MacIntel" && max_touch > 1 {
                return Some(true);
            }
            Some(false)
        })
        .unwrap_or(false)
}
```

> **Scroll Lock Strategy (Tiered)**
>
> 1. **Modern browsers**: `overflow:clip` on `<html>` + `overscroll-behavior:contain` on `<body>`.
>    `overflow:clip` prevents scrolling without creating a new formatting context (unlike `overflow:hidden`).
>    No scroll position jump. No impact on fixed-position children.
> 2. **iOS Safari fallback**: When `overflow:clip` is not supported (Safari < 16), use `position:fixed`
>    on `<body>` with `top: -{scrollY}px`. On unlock, restore `scrollTo(savedScrollX, savedScrollY)`.
>    Note: This approach temporarily breaks other fixed-position elements. Document this limitation.
> 3. **Scrollbar width compensation**: When locking scroll, add `padding-right` equal to the scrollbar
>    width to prevent layout shift.
>
> **Fixed-position element compensation:** The iOS `position: fixed` body workaround causes all elements with `position: fixed` on the page (sticky headers, FABs, chat widgets) to lose their viewport-relative positioning. When applying this workaround, the adapter SHOULD also set `padding-right` compensation on direct children of `<body>` that have `position: fixed` computed style. Alternatively, use `overflow: clip` (Safari 16+) which does not create a new containing block.
>
> **Known Limitation: iOS `position: fixed` body workaround**
>
> Setting `position: fixed` on `<body>` creates a new containing block, which
> breaks all `position: fixed` descendants (sticky headers, FABs, chat widgets).
>
> **Mitigation strategies:**
>
> 1. **Non-iOS browsers**: Use `overflow: hidden` on `<html>` and `<body>` instead
>    of `position: fixed`. This preserves fixed-position children.
> 2. **iOS with fixed children**: Move fixed-position elements to a sibling of
>    `<body>` using the portal pattern, so they are not descendants of the fixed body.
> 3. **Overlay scrim alternative**: Apply `touch-action: none` on the overlay scrim
>    element instead of locking body scroll. This prevents touch scrolling on the
>    scrim without affecting body positioning.
> 4. **Hybrid approach** (recommended): Use `overflow: hidden` as default, fall back
>    to `position: fixed` only on iOS Safari where overflow:hidden doesn't prevent
>    momentum scrolling, and document that fixed children need portal treatment.

### 5.7 Scrollbar Width Compensation

When scroll locking is applied, the vertical scrollbar disappears. On platforms where the scrollbar occupies layout space (Windows, Linux with classic scrollbars), this causes a layout shift: content jumps to the right by the scrollbar width.

To prevent this shift, consumers should add `padding-right` equal to the scrollbar width to the `<body>` element during lock. The library provides a utility to measure the scrollbar width:

```rust
// ars-dom/src/scroll_lock.rs

/// Measure the width of the vertical scrollbar by comparing the window's
/// inner width to the document element's client width.
///
/// Returns 0 on platforms with overlay scrollbars (macOS, iOS, Android).
/// Returns the scrollbar width in pixels on platforms with classic scrollbars
/// (typically 15-17px on Windows/Linux).
///
/// This measurement is performed live and accounts for zoom level.
pub fn scrollbar_width() -> f64 {
    let window = web_sys::window().expect("window must exist in browser context");
    let document = window.document().expect("document must exist in browser context");
    let doc_el = document.document_element().expect("document element must exist");
    // inner_width includes scrollbar; client_width excludes it
    let inner = window.inner_width()
        .expect("inner_width available in browser").as_f64().unwrap_or(0.0);
    let client = doc_el.client_width() as f64;
    (inner - client).max(0.0)
}

/// Apply scrollbar width compensation to the body element.
/// Called internally by ScrollLock::lock() when scroll locking is applied.
///
/// Checks for `scrollbar-gutter: stable` before applying — if the page already
/// reserves scrollbar space via this CSS property, adding padding-right would
/// double-reserve the space.
fn apply_scrollbar_compensation() {
    // If the page uses `scrollbar-gutter: stable`, space is already reserved.
    // Do not add additional padding-right.
    // let gutter = window.getComputedStyle(document_element).scrollbar_gutter;
    // if gutter.contains("stable") { return; }

    let width = scrollbar_width();
    if width > 0.0 {
        // body.style.padding_right = format!("{}px", width)
    }
}

/// Remove scrollbar width compensation from the body element.
/// Called internally by ScrollLock::unlock() when scroll locking is removed.
fn remove_scrollbar_compensation() {
    // body.style.padding_right = saved_padding_right
}
```

> **Scrollbar width measurement.** `window.innerWidth - document.documentElement.clientWidth`
> returns 0 on overlay-scrollbar systems (macOS, mobile). For reliable measurement, create
> a temporary `<div>` with `overflow:scroll`, measure `offsetWidth - clientWidth`, then remove.
> Cache the result per session.
>
> The `.max(0.0)` clamp is deliberate. Overlay-scrollbar platforms often report
> identical inner/client widths, and rounding differences should never produce a
> negative compensation width.
>
> **RTL scroll compensation.** In right-to-left layouts, the vertical scrollbar appears on the left side of the viewport. When applying scrollbar width compensation, implementations MUST check the document direction via `document.dir` or `getComputedStyle(document.documentElement).direction`. In RTL mode, apply the compensation to `padding-left` instead of `padding-right`. The same applies to fixed-position element compensation — adjust the `left` property rather than `right`.

### 5.8 Components That Use Scroll Lock

The following components activate scroll locking when they open:

| Component   | Locks Scroll | Notes                                              |
| ----------- | ------------ | -------------------------------------------------- |
| Dialog      | Modal only   | Non-modal dialogs do not lock scroll               |
| AlertDialog | Always       | Always modal                                       |
| Drawer      | Always       | Overlay drawers always lock; inline drawers do not |

Components that do **not** lock scroll (they float above content but the page remains scrollable):

- Popover
- Tooltip
- Menu (standalone)
- HoverCard
- Toast
- Select (listbox)
- Combobox (listbox)

### 5.9 Full Implementation Details

This section provides the complete, authoritative specification for scroll lock behavior.

#### 5.9.1 Locking Procedure

1. **Measure scroll position.** Read `window.scrollY` (and `window.scrollX` for horizontal scroll) before applying any style changes. Store as `savedScrollY` and `savedScrollX`.

2. **Apply lock styles (tiered strategy):**

    **Tier 1 — Modern browsers:** Apply to both `<html>` and `<body>`:

    ```css
    /* <html> */
    overflow: clip;
    /* <body> */
    overscroll-behavior: contain;
    ```

    `overflow: clip` prevents scrolling without creating a new formatting context (unlike `overflow: hidden`). No scroll position jump. No impact on fixed-position children.

    **Tier 2 — iOS Safari fallback** (when `overflow: clip` is not supported, Safari < 16): Apply to `<body>`:

    ```css
    position: fixed;
    top: -{savedScrollY}px;
    width: 100%;
    overflow: hidden;
    ```

    Setting `position: fixed` on `<body>` removes it from the scroll flow. The `top: -{savedScrollY}px` offset maintains the visual position so the page does not jump to the top. Note: this temporarily breaks other fixed-position elements (see §5.6).

3. **Scrollbar compensation.** Measure scrollbar width and apply `padding-right` (or `padding-left` in RTL) to prevent layout shift (see §5.7).

#### 5.9.2 Unlocking Procedure

1. **Remove lock styles.** Clear `overflow`, `position`, `width`, and `top` overrides from the document element, restoring the saved original values.

2. **Restore scroll position.** Call `window.scrollTo(savedScrollX, savedScrollY)` immediately after removing styles to restore the exact scroll position the user was at before locking.

3. **Remove scrollbar compensation.** Restore the original `padding-right` value.

#### 5.9.3 Nested Scrollable Containers

Nested scrollable containers inside the locked overlay (e.g., a scrollable list within a modal dialog) MUST remain scrollable. The lock styles (`overflow: hidden`) on the document element do not affect descendants with their own `overflow: auto` or `overflow: scroll` — these continue to scroll normally. Implementations MUST NOT apply `overflow: hidden` to the `<body>` or any element that contains scrollable overlay content.

#### 5.9.4 Stacking / Reference Counting

When multiple overlays stack (e.g., a Dialog opens a nested AlertDialog), only the topmost overlay's lock/unlock is effective in terms of style application. The implementation maintains a reference count (`depth` counter):

- **`acquire()`**: Increment depth. If depth transitions from 0 → 1, save state and apply lock styles.
- **`release()`**: Decrement depth. If depth transitions from 1 → 0, restore state and remove lock styles.
- Intermediate calls only adjust the counter — no style changes occur while depth > 1.

This ensures that closing an inner overlay does not prematurely unlock scrolling while an outer overlay is still open.

#### 5.9.5 RTL Considerations

`scrollLeft` normalization varies across browsers when `direction: rtl` is set on the document element:

- Chrome/Edge: `scrollLeft` is 0 at the right edge (start) and negative going left.
- Firefox: `scrollLeft` is 0 at the right edge and positive going left.
- Safari: `scrollLeft` is positive, with maximum at the right edge.

Implementations MUST test scroll lock with `direction: rtl` on the document element. When saving/restoring horizontal scroll position, normalize `scrollLeft` to a consistent convention before storing and convert back when calling `scrollTo()`.

### 5.10 Platform Behavior

**Web (`ars-dom`)**: Apply the full locking procedure as specified in §5.2–5.9.

**Desktop (Dioxus without `web-sys`)**: Scroll lock is a no-op. Desktop windowing systems handle modal behavior natively via the window manager.

**SSR**: Scroll lock is a no-op. No DOM available during server-side rendering.

---

## 6. Z-Index Management

### 6.1 Overview

Overlay components need predictable stacking. When multiple overlays are open simultaneously — or when overlays are nested (e.g., a Dialog containing a Menu) — each overlay must render above the previous one. The library provides an auto-incrementing z-index allocator that guarantees monotonically increasing values without requiring consumers to manually track z-index values.

### 6.2 Strategy

````rust
// ars-dom/src/z_index.rs

use std::cell::{Cell, RefCell};

thread_local! {
    /// Base z-index value. Set high enough to sit above typical application content.
    /// Consumer stylesheets should avoid z-index values at or above this base.
    static NEXT_Z_INDEX: Cell<u32> = Cell::new(1000);
}

/// Allocate the next z-index for an overlay.
/// Each call returns a monotonically increasing value.
/// Single-threaded: consistent with the library's `Rc`-based, WASM-first design.
///
/// # Example
///
/// ```rust
/// let z = next_z_index(); // 1000
/// let z2 = next_z_index(); // 1001
/// assert!(z2 > z);
/// ```
/// Z-index overflow threshold. If the counter exceeds this, it resets to the
/// base value. This prevents wrap-around to 0 on very long-running SPAs that
/// open many overlays (e.g., tooltips, popovers over thousands of interactions).
const Z_INDEX_BASE: u32 = 1000;
const Z_INDEX_CEILING: u32 = u32::MAX - 1000;

pub fn next_z_index() -> u32 {
    NEXT_Z_INDEX.with(|z| {
        let val = z.get();
        if val >= Z_INDEX_CEILING {
            // Return the fresh base now, but store BASE + 1 so the following
            // allocation continues from the new sequence without repeating BASE.
            // Reset to base — existing overlays at high z-indexes will still
            // render above normal content; new overlays start fresh.
            #[cfg(feature = "debug")]
            log::warn!(
                "[ars-dom] z-index counter reached ceiling ({Z_INDEX_CEILING}), \
                 resetting to base ({Z_INDEX_BASE})"
            );
            z.set(Z_INDEX_BASE + 1);
            Z_INDEX_BASE
        } else {
            z.set(val + 1);
            val
        }
    })
}

/// Reset the z-index counter to a given base value.
/// Intended for use in tests to ensure deterministic z-index values.
///
/// # Example
///
/// ```rust
/// reset_z_index(1000);
/// assert_eq!(next_z_index(), 1000);
/// assert_eq!(next_z_index(), 1001);
/// ```
pub fn reset_z_index(base: u32) {
    NEXT_Z_INDEX.with(|z| z.set(base));
}

/// Structured z-index allocator for managing overlay stacking with explicit
/// lifecycle control. Wraps the thread-local counter and adds release tracking
/// to enable future compaction strategies.
pub struct ZIndexAllocator {
    allocated: RefCell<Vec<u32>>,
}

impl ZIndexAllocator {
    pub fn new() -> Self {
        Self {
            allocated: RefCell::new(Vec::new()),
        }
    }

    /// Allocate the next z-index. Delegates to the thread-local counter
    /// and records the value for later release/compaction.
    pub fn allocate(&self) -> u32 {
        let z = next_z_index();
        self.allocated.borrow_mut().push(z);
        z
    }

    /// Release a previously allocated z-index. Removes it from the tracked
    /// set. After release, the value will not be reused — new allocations
    /// always increment. Release enables future compaction: when all values
    /// in a range are released, the allocator can reclaim that range.
    pub fn release(&self, z: u32) {
        self.allocated.borrow_mut().retain(|&v| v != z);
    }

    /// Reset the allocator: clear all tracked allocations and reset the
    /// thread-local counter to the base value. Intended for tests and
    /// application-level teardown (e.g., full-page navigation in an SPA).
    pub fn reset(&self) {
        self.allocated.borrow_mut().clear();
        reset_z_index(Z_INDEX_BASE);
    }
}
````

### 6.3 Usage Pattern

Each overlay component calls `next_z_index()` when it opens (mounts into the DOM). Components with both a backdrop and content element allocate two consecutive z-index values: one for the backdrop (lower) and one for the content (higher). The returned value is applied as an inline style on the overlay's positioner or root element:

```rust
// Inside a Popover's connect function:
let z = next_z_index();
positioner_attrs.set_style(CssProperty::ZIndex, z.to_string());
```

This guarantees that overlays stack in opening order:

1. Dialog A opens → z-index 1000
2. Menu B opens inside Dialog A → z-index 1001 (renders above Dialog A)
3. Tooltip C appears over Menu B → z-index 1002 (renders above Menu B)

When an overlay closes and later reopens, it receives a new (higher) z-index. This avoids stale z-index conflicts when overlays open and close in unpredictable order.

> **Z-index management.** The monotonic counter (`next_z_index()`) always increments; values are
> never reused or compacted. When an overlay closes and reopens, it receives a new, higher value.
> Gaps in the sequence (e.g., [1000, 1002, 1005]) are expected and harmless — they do not cause
> stacking inversions because the opening order is preserved by the counter. The `ZIndexAllocator`
> tracks released values for bookkeeping but does not reassign them.

#### 6.3.1 Z-Index Ranges and Adapter Scope

**Reserved z-index ranges:**

| Range | Purpose                                                                |
| ----- | ---------------------------------------------------------------------- |
| 0–999 | User/application content. The library MUST NOT allocate in this range. |
| 1000+ | Overlay components managed by `ZIndexAllocator`.                       |

**Cross-adapter consistency:** All adapters (Leptos, Dioxus) share the same thread-local `NEXT_Z_INDEX` counter via `ars-dom`. This ensures that overlays opened by different adapter instances on the same page stack correctly.

**Nested overlay ordering:** Nested overlays (e.g., a Menu inside a Dialog) increment from the parent's z-index. The `next_z_index()` function handles this automatically since calls are sequential — the child always receives a higher value than the parent.

**Portal z-index independence:** Overlays rendered through portals (appended to `document.body`) are outside their parent's stacking context. Their z-index is absolute, not relative to the parent component's stacking context. This is by design — portal-based overlays must stack above all non-overlay content regardless of where the trigger component lives in the DOM tree.

### 6.4 Components That Use the Z-Index Allocator

- Dialog (modal and non-modal)
- AlertDialog
- Popover
- Menu
- Tooltip
- Toast
- HoverCard
- Select (listbox overlay)
- Combobox (listbox overlay)
- DatePicker (calendar overlay)
- Drawer

### 6.5 CSS `top-layer` Note

Modern browsers support the CSS `top-layer` via the native `<dialog>` element and the Popover API (`popover` attribute). Elements promoted to the top layer bypass z-index stacking entirely — they are rendered above all other content regardless of z-index values.

Components that render via native `<dialog>` (Dialog, AlertDialog) or the `popover` attribute do not need the z-index allocator when running in browsers that support these APIs. The allocator serves as the fallback for portaled-div overlays in browsers without top-layer support, or when the component intentionally avoids native APIs for flexibility.

The library detects top-layer support at runtime. When available, overlay components skip the `next_z_index()` call and rely on the browser's native stacking. When unavailable, they fall back to the z-index allocator.

```rust
/// Check whether the browser supports the CSS top-layer (native <dialog> or popover).
/// This is detected once and cached.
pub fn supports_top_layer() -> bool {
    thread_local! {
        static CACHED: Cell<Option<bool>> = const { Cell::new(None) };
    }
    CACHED.with(|c| {
        if let Some(v) = c.get() {
            return v;
        }
        let supported = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.create_element("dialog").ok())
            .is_some_and(|el| {
                js_sys::Reflect::has(&el, &"showModal".into()).unwrap_or(false)
            });
        c.set(Some(supported));
        supported
    })
}
```

This wrap behavior is intentionally one-step ahead: the call that detects the
ceiling returns `Z_INDEX_BASE`, and the counter stores `Z_INDEX_BASE + 1` for
the next allocation. That preserves the usual "return current, then advance"
contract even across the wrap boundary.

### 6.6 CSS Containment Interaction

CSS `contain` property values can interfere with z-index stacking and overlay positioning:

- **`contain: layout`** — Creates a new stacking context, which breaks z-index ordering for child overlays. An overlay inside a `contain: layout` container cannot stack above elements outside that container using z-index alone. **Avoid `contain: layout` on elements that contain overlay triggers.**
- **`contain: content`** — Equivalent to `contain: layout style paint`. This fully isolates the element's stacking context. Overlays inside a `contain: content` element are effectively trapped within that context.
- **`contain: paint`** — Clips content to the element's padding box and creates a stacking context. Floating overlays (tooltips, popovers) will be clipped if rendered inside a `contain: paint` ancestor.
- **Portalled elements** — Overlays rendered through portals (appended to `document.body`) are unaffected by ancestor containment because they are DOM-detached. Portal-based overlays MUST use `contain: none` (the default) on the portal container itself.
- **`contain: style`** — Does NOT create a stacking context. Safe to use with overlays.

**Migration path:** If a component uses `contain: layout` or `contain: content` for performance and also renders overlays, refactor to use portals for the overlay content. The trigger element can remain inside the contained ancestor; only the floating content needs to escape.

**Detection:** The positioning engine (§2) should check for containment on ancestors of the floating element. If `contain: layout` or `contain: paint` is detected, log a development-mode warning suggesting portal usage.

---

## 7. Portal Root and Background Inert

All overlay components (Dialog, Popover, Menu, Tooltip, etc.) render into a single shared portal root. This simplifies `inert` management for modal overlays.

```rust
// ars-dom/src/portal.rs

/// Get or create the shared portal root element.
/// All overlay components render into this single container.
/// Having a single portal root simplifies `inert` management.
///
/// When finding an existing element with `id="ars-portal-root"`, verifies
/// the `data-ars-managed` attribute is present. If an element exists without
/// this marker (potential DOM clobbering), creates a new container with a
/// randomized suffix instead.
pub fn get_or_create_portal_root() -> web_sys::Element {
    let document = web_sys::window()
        .expect("window exists in browser context")
        .document()
        .expect("document exists");

    // Look for existing <div id="ars-portal-root">
    if let Ok(Some(existing)) = document.query_selector("#ars-portal-root") {
        if existing.has_attribute("data-ars-managed") {
            return existing;
        }
        // Element exists without our marker — potential DOM clobbering.
        // Create a new container with a randomized suffix instead.
        web_sys::console::warn_1(
            &"ars-ui: found #ars-portal-root without data-ars-managed; creating alternate root".into(),
        );
        let suffix = (js_sys::Math::random() * 1_000_000.0) as u32;
        let root = document.create_element("div").expect("create div element");
        root.set_id(&format!("ars-portal-root-{}", suffix));
        root.set_attribute("data-ars-managed", "").expect("set data-ars-managed attribute");
        document.body().expect("document has body").append_child(&root).expect("append portal root to body");
        return root;
    }

    // Not found — create as direct child of <body>
    let root = document.create_element("div").expect("create div element");
    root.set_id("ars-portal-root");
    root.set_attribute("data-ars-managed", "").expect("set data-ars-managed attribute");
    document.body().expect("document has body").append_child(&root).expect("append portal root to body");
    root
}

/// Get or create the per-instance mount root under the shared portal root.
///
/// The returned mount node carries `data-ars-portal-owner="<owner_id>"` so
/// outside-interaction detection can treat portalled descendants as inside
/// the originating overlay boundary.
///
/// The mount node ID is stable for the owner and uses the format
/// `ars-portal-<owner_id>`.
pub fn ensure_portal_mount_root(owner_id: &str) -> web_sys::Element {
    let portal_root = get_or_create_portal_root();
    let mount_id = format!("ars-portal-{owner_id}");

    if let Some(existing) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(&mount_id))
        .filter(|el| portal_root.contains(Some(el)))
    {
        return existing;
    }

    let document = web_sys::window()
        .expect("window exists in browser context")
        .document()
        .expect("document exists");
    let mount = document.create_element("div").expect("create div element");
    mount.set_id(&mount_id);
    mount.set_attribute("data-ars-managed", "").expect("set data-ars-managed attribute");
    mount
        .set_attribute("data-ars-portal-owner", owner_id)
        .expect("set data-ars-portal-owner attribute");
    portal_root
        .append_child(&mount)
        .expect("append portal mount to shared root");
    mount
}

/// Set `inert` and `aria-hidden="true"` on all siblings of the portal root.
/// Returns a cleanup function that removes the attributes.
pub fn set_background_inert(portal_root_id: &str) -> Box<dyn FnOnce()> {
    let document = web_sys::window().expect("window exists in browser context")
        .document().expect("document exists");
    let body = document.body().expect("document has body");
    let portal_root = document.get_element_by_id(portal_root_id);
    let children = body.children();
    let mut restored: Vec<(web_sys::Element, Option<String>, Option<String>)> = Vec::new();

    for i in 0..children.length() {
        if let Some(child) = children.item(i) {
            // Skip the portal root itself
            if let Some(ref pr) = portal_root {
                if &child == pr || child.contains(Some(pr)) {
                    continue;
                }
            }
            // Save original attribute values for cleanup
            let prev_inert = child.get_attribute("inert");
            let prev_aria_hidden = child.get_attribute("aria-hidden");

            child.set_attribute("inert", "").expect("set inert attribute");
            child.set_attribute("aria-hidden", "true").expect("set aria-hidden attribute");
            restored.push((child, prev_inert, prev_aria_hidden));
        }
    }

    // Return cleanup function
    Box::new(move || {
        for (el, prev_inert, prev_aria_hidden) in restored {
            match prev_inert {
                Some(v) => el.set_attribute("inert", &v).expect("restore inert attribute"),
                None => el.remove_attribute("inert").expect("remove inert attribute"),
            }
            match prev_aria_hidden {
                Some(v) => el.set_attribute("aria-hidden", &v).expect("restore aria-hidden attribute"),
                None => el.remove_attribute("aria-hidden").expect("remove aria-hidden attribute"),
            }
        }
    })
}
```

Usage flow for a modal dialog:

1. Dialog machine transitions to `Open` state.
2. The adapter obtains a stable per-dialog mount node with `ensure_portal_mount_root(dialog_id)`.
3. `TransitionPlan` includes a `PendingEffect` for `"background-inert"`.
4. Adapter calls `set_background_inert("ars-portal-root")` during effect setup.
5. The returned cleanup function is stored and called when the dialog closes.
6. Scroll lock is applied separately via `prevent_scroll()` (§5.4).

---

## 8. Modality Manager

`ars-dom` is the web binding layer for the shared core modality contract. It owns the browser listener lifecycle and fans normalized events out to both `ars_core::ModalityContext` and `ars-a11y::FocusRing` so adapters cannot accidentally update only one side.

```rust
// ars-dom/src/modality.rs

use ars_a11y::FocusRing;
use ars_core::{KeyboardKey, KeyModifiers, ModalityContext, PointerType};

/// Unified modality update — ensures accessibility and interaction consumers
/// stay in sync. Adapters MUST call these methods instead of updating the
/// shared modality context or `FocusRing` independently.
pub struct ModalityManager {
    modality: Arc<dyn ModalityContext>,
    focus_ring: FocusRing,
}

impl ModalityManager {
    pub fn new(modality: Arc<dyn ModalityContext>) -> Self {
        Self { modality, focus_ring: FocusRing::new() }
    }

    /// Call on keydown events. Updates both modality trackers atomically.
    pub fn on_key_down(&self, key: KeyboardKey, modifiers: KeyModifiers) {
        self.modality.on_key_down(key, modifiers);
        self.focus_ring.on_key_down(key, modifiers);
    }

    /// Call on pointerdown/mousedown/touchstart events.
    pub fn on_pointer_down(&self, pointer_type: PointerType) {
        self.modality.on_pointer_down(pointer_type);
        self.focus_ring.on_pointer_down();
    }

    /// Call on virtual or assistive-technology interactions.
    pub fn on_virtual_input(&self) {
        self.modality.on_virtual_input();
        self.focus_ring.on_virtual_input();
    }

    /// Install the browser event listeners owned by this manager.
    pub fn ensure_listeners(&self) {
        // Browser-only implementation:
        // - no-op without a document
        // - attaches keydown, pointerdown, mousedown, touchstart, and focus(capture)
        // - uses refcounted install/remove semantics
    }

    /// Delegate to the inner `FocusRing` for focus-visible checks.
    pub fn focus_ring(&self) -> &FocusRing {
        &self.focus_ring
    }
}
```

> **Note:** `ars-interactions` no longer owns document-level listeners. The adapter's listener closures should call `ModalityManager::on_key_down()` / `on_pointer_down()` / `on_virtual_input()` instead of touching the shared modality context or `FocusRing` directly.

---

## 9. Overlay Stack Registry

### 9.1 Overview

Overlay components (Dialog, Popover, Menu, Tooltip, etc.) register with a global thread-local stack when they mount and deregister when they unmount. The stack determines:

- **Topmost overlay** — only the topmost overlay responds to outside interactions and Escape-key dismissal (`05-interactions.md` §12.8 rule 1).
- **Child overlay membership** — a click inside a child overlay does NOT trigger `InteractOutside` on the parent (`05-interactions.md` §12.8 rule 2).
- **LIFO close ordering** — Escape / outside-click dismisses the topmost overlay first; the parent remains open (`05-interactions.md` §12.8 rule 3).

The overlay stack is distinct from the z-index allocator (§6). The allocator assigns stacking-order values; the overlay stack tracks which overlays are currently open and their nesting relationship. Overlay components use both: `next_z_index()` for visual stacking order and the overlay stack for dismissal logic.

### 9.2 Thread-Local Design

The stack is stored in a `thread_local!` `RefCell<Vec<OverlayEntry>>`, consistent with the z-index allocator (§6.2) and the library's `Rc`-based, single-threaded WASM-first design. Each thread maintains its own overlay stack.

### 9.3 Types and Public API

```rust
// ars-dom/src/overlay_stack.rs

use std::cell::RefCell;

thread_local! {
    /// Per-thread overlay stack. Entries are ordered by mount time: the last
    /// entry is the topmost (most recently opened) overlay.
    static OVERLAY_STACK: RefCell<Vec<OverlayEntry>> = const { RefCell::new(Vec::new()) };
}

/// Metadata for a registered overlay in the global stack.
///
/// Each overlay component creates an `OverlayEntry` when it mounts and passes
/// it to `push_overlay()`. The entry records whether the overlay is modal
/// (triggering scroll lock and background inert) and the allocated z-index
/// (or `None` when using native CSS top-layer).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverlayEntry {
    /// Unique overlay identifier (matches the component's DOM id).
    pub id: String,

    /// Whether this overlay is modal (triggers scroll lock + background inert).
    pub modal: bool,

    /// Allocated z-index from `next_z_index()` (§6), or `None` when the overlay
    /// uses native CSS top-layer (see `supports_top_layer()` in §6.5).
    pub z_index: Option<u32>,
}

/// Register an overlay on the global stack.
///
/// Called when an overlay component mounts. If an entry with the same `id`
/// already exists, the call is a no-op to prevent double-registration from
/// framework re-renders.
pub fn push_overlay(entry: OverlayEntry) {
    OVERLAY_STACK.with(|stack| {
        let mut stack = stack.borrow_mut();
        if !stack.iter().any(|e| e.id == entry.id) {
            stack.push(entry);
        }
    });
}

/// Deregister an overlay from the global stack.
///
/// Removes the entry with the given `id` regardless of its position in the
/// stack (not limited to the topmost entry). Called when an overlay component
/// unmounts. If no entry with the given `id` exists, the call is a no-op
/// (safe to call multiple times).
pub fn remove_overlay(id: &str) {
    OVERLAY_STACK.with(|stack| {
        stack.borrow_mut().retain(|e| e.id != id);
    });
}

/// Return the topmost (most recently opened) overlay, or `None` if the stack
/// is empty.
pub fn topmost_overlay() -> Option<OverlayEntry> {
    OVERLAY_STACK.with(|stack| stack.borrow().last().cloned())
}

/// Check whether the overlay with the given `id` is the topmost overlay.
pub fn is_topmost(id: &str) -> bool {
    OVERLAY_STACK.with(|stack| stack.borrow().last().is_some_and(|entry| entry.id == id))
}

/// Return the IDs of all overlays stacked above the overlay with the given
/// `id`. Used by `InteractOutside` to determine whether a click target is
/// inside a child overlay. Returns an empty `Vec` if `id` is not found.
pub fn overlays_above(id: &str) -> Vec<String> {
    OVERLAY_STACK.with(|stack| {
        let stack = stack.borrow();
        let pos = stack.iter().position(|e| e.id == id);
        match pos {
            Some(idx) => stack[idx + 1..].iter().map(|e| e.id.clone()).collect(),
            None => Vec::new(),
        }
    })
}

/// Check whether an overlay with the given `id` is currently registered.
pub fn contains_overlay(id: &str) -> bool {
    OVERLAY_STACK.with(|stack| stack.borrow().iter().any(|e| e.id == id))
}

/// Check whether `child_id` is stacked above `parent_id`.
///
/// Returns `true` when both IDs are registered and the child appears later
/// in the stack (i.e., was pushed after the parent). Returns `false` when
/// either ID is missing or when `child_id == parent_id`.
///
/// Convenience wrapper for `InteractOutside` suppression: a parent overlay
/// should not fire an outside-interaction event when the click target belongs
/// to a child overlay above it.
pub fn is_above(child_id: &str, parent_id: &str) -> bool {
    OVERLAY_STACK.with(|stack| {
        let stack = stack.borrow();
        let parent_pos = stack.iter().position(|e| e.id == parent_id);
        let child_pos = stack.iter().position(|e| e.id == child_id);
        matches!((parent_pos, child_pos), (Some(p), Some(c)) if c > p)
    })
}

/// Return the number of overlays currently on the stack.
pub fn overlay_count() -> usize {
    OVERLAY_STACK.with(|stack| stack.borrow().len())
}

/// Clear the overlay stack.
///
/// Intended for tests and application-level teardown (e.g., full-page
/// navigation in an SPA). Matches `reset_z_index()` (§6.2) in purpose.
pub fn reset_overlay_stack() {
    OVERLAY_STACK.with(|stack| stack.borrow_mut().clear());
}
```

### 9.4 Usage by Overlay Components

Each overlay component calls `push_overlay()` when it mounts (opens) and `remove_overlay()` when it unmounts (closes):

```rust
// Inside an overlay component's connect function:
let z = if supports_top_layer() {
    None
} else {
    Some(next_z_index())
};

push_overlay(OverlayEntry {
    id: ids.base.clone(),
    modal: true,
    z_index: z,
});

// On unmount (effect cleanup):
remove_overlay(&ids.base);
```

`InteractOutside` (see `05-interactions.md` §12.8) consults the overlay stack via `is_topmost()`, `is_above()`, and `overlays_above()` to implement nested overlay dismissal:

1. Before processing an outside interaction, check `is_topmost(my_id)` — only the topmost overlay responds.
2. Before firing `InteractOutsideEvent::PointerOutside`, use `is_above(child_id, my_id)` to check whether the click target belongs to a child overlay — clicks inside child overlays are not "outside." For bulk queries, `overlays_above(my_id)` returns all child IDs at once.

### 9.5 Components That Use the Overlay Stack

All components listed in §6.4 (Z-Index Allocator users) also register with the overlay stack:

- Dialog (modal and non-modal)
- AlertDialog
- Popover
- Menu
- Tooltip
- Toast
- HoverCard
- Select (listbox overlay)
- Combobox (listbox overlay)
- DatePicker (calendar overlay)
- Drawer

---

## 10. Media Query Utilities

```rust
// crates/ars-dom/src/media.rs
//
// NOTE: These functions live in `ars-dom` (not `ars-a11y`) because they
// depend on `web_sys::window()` which requires std and web_sys.
// Components import directly from ars_dom::media.
// (ars-a11y cannot re-export these because ars-dom depends on ars-a11y,
// which would create a circular dependency.)

/// Detects if the user has enabled forced colors (Windows High Contrast Mode).
/// This value CAN change at runtime (user can toggle via Win+U or Settings).
/// Use `window.matchMedia("(forced-colors: active)")` with `addEventListener("change", ...)`
/// to reactively update, matching the `prefers_reduced_motion()` reactive pattern.
pub fn is_forced_colors_active() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(forced-colors: active)").ok().flatten())
        .map(|mql| mql.matches())
        .unwrap_or(false)
}

/// Detects if the user prefers reduced motion.
/// Updated reactively if the user changes system settings at runtime.
pub fn prefers_reduced_motion() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(prefers-reduced-motion: reduce)").ok().flatten())
        .map(|mql| mql.matches())
        .unwrap_or(false)
}

/// Detects if the user prefers a reduced transparency level.
/// Use to disable backdrop-filter effects.
pub fn prefers_reduced_transparency() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(prefers-reduced-transparency: reduce)").ok().flatten())
        .map(|mql| mql.matches())
        .unwrap_or(false)
}

/// Detects system color scheme preference via `matchMedia("(prefers-color-scheme: dark)")`.
/// Follows the same caching pattern as other media query functions (see note below).
pub fn prefers_color_scheme() -> ColorScheme {
    web_sys::window()
        .and_then(|w| w.match_media("(prefers-color-scheme: dark)").ok().flatten())
        .map(|mql| if mql.matches() { ColorScheme::Dark } else { ColorScheme::Light })
        .unwrap_or(ColorScheme::Light)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorScheme { Light, Dark }
```

> **`matchMedia()` Caching:** Cache the `MediaQueryList` **object** (not the boolean result) in a `thread_local! { static MQL: OnceCell<Option<web_sys::MediaQueryList>> }`. On each call, read `.matches()` from the cached object — this is a live property that always reflects the current state, so no explicit `change` event listener is needed. Caching the object avoids re-parsing the query string on every call (the expensive part) while still tracking runtime changes (e.g., user toggles High Contrast Mode). The same caching pattern applies to all five media query functions.

---

## 11. URL Sanitization

URL sanitization is a connect-layer safety contract, not a DOM-runtime helper.
The canonical definitions for `is_safe_url()`, `sanitize_url()`, `SafeUrl`,
and `UnsafeUrlError` live in `01-architecture.md` §3.1.1.1 because the rule
applies anywhere a component renders `HtmlAttr::Href`, `HtmlAttr::Action`, or
`HtmlAttr::FormAction`, including framework-agnostic machines.

`ars-dom` does not own this contract. DOM-facing crates consume the shared
`ars-core` helpers when they need to render URL-valued attributes.
