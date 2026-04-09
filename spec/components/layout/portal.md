---
component: Portal
category: layout
tier: stateful
foundation_deps: [architecture, accessibility, interactions]
shared_deps: [layout-shared-types]
related: []
references:
    radix-ui: Portal
---

# Portal

`Portal` renders its children into a DOM node outside the parent component's hierarchy. It manages a mount/unmount lifecycle and supports multiple target containers. Used by overlay components (Dialog, Popover, Tooltip, Toast) to escape clipping and stacking contexts.

## 1. State Machine

### 1.1 States

```rust
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum State {
    /// The portal is unmounted.
    #[default]
    Unmounted,
    /// The portal is mounted at its target container.
    Mounted,
}
```

### 1.2 Events

```rust
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Mount the portal after the host component mounts.
    Mount,
    /// Unmount the portal before the host component unmounts.
    Unmount,
    /// The target container became available (for `Id` targets that
    /// may not exist at `Mount` time). Carries the element ID.
    ContainerReady(String),
}
```

### 1.3 Context

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The resolved target container for the portal.
    pub container: PortalTarget,
    /// Whether the portal is mounted.
    pub mounted: bool,
    /// Whether the runtime is in SSR mode.
    pub ssr: bool,
    /// Component IDs.
    pub ids: ComponentIds,
}
```

### 1.4 Props

```rust
/// The target container for the portal.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum PortalTarget {
    /// The dedicated portal root element (`#ars-portal-root`).
    #[default]
    PortalRoot,
    /// The document body.
    Body,
    /// An element with the given ID.
    Id(String),
    /// A direct element ID reference.
    Ref(String),
}

#[derive(Clone, Debug, Default, PartialEq, HasId)]
pub struct Props {
    /// The id of the component.
    pub id: String,
    /// The target container for the portal.
    pub container: PortalTarget,
    /// Whether to render the portal inline during SSR. When `true`, content
    /// is rendered at the declaration site during SSR; the client hydration
    /// layer reattaches it to the target container.
    pub ssr_inline: bool,
}
```

### 1.5 Full Machine Implementation

```rust
pub struct Machine;

/// This component has no translatable strings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Messages;
impl ComponentMessages for Messages {}

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Api<'a> = Api<'a>;

    fn init(props: &Props, _env: &Env, _messages: &Messages) -> (State, Context) {
        let ctx = Context {
            container: props.container.clone(),
            mounted: false,
            ssr: cfg!(feature = "ssr"),
            ids: ComponentIds::from_id(&props.id),
        };
        (State::Unmounted, ctx)
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            (State::Unmounted, Event::Mount) => {
                Some(TransitionPlan::to(State::Mounted).apply(|ctx| {
                    ctx.mounted = true;
                }))
            }
            (State::Mounted, Event::Unmount) => {
                Some(TransitionPlan::to(State::Unmounted).apply(|ctx| {
                    ctx.mounted = false;
                }))
            }
            (State::Unmounted, Event::ContainerReady(id)) => {
                let id = id.clone();
                Some(TransitionPlan::to(State::Mounted).apply(move |ctx| {
                    ctx.container = PortalTarget::Ref(id);
                    ctx.mounted = true;
                }))
            }
            _ => None,
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        ctx: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api { state, ctx, props, send }
    }
}
```

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "portal"]
pub enum Part {
    Root,
}

pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Whether the portal is currently mounted.
    pub fn is_mounted(&self) -> bool {
        *self.state == State::Mounted
    }

    /// The generated portal root element ID, usable for `aria-owns` on triggers.
    pub fn portal_root_id(&self) -> String {
        format!("ars-portal-{}", self.props.id)
    }

    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-portal-id"), &self.props.id);
        attrs.set(HtmlAttr::Data("ars-state"),
            if self.is_mounted() { "mounted" } else { "unmounted" },
        );
        attrs
    }

    pub fn on_mount(&self) { (self.send)(Event::Mount); }
    pub fn on_unmount(&self) { (self.send)(Event::Unmount); }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
Portal
└── Root  <div>  data-ars-scope="portal" data-ars-part="root"
                 data-ars-portal-id="<id>" data-ars-state="unmounted|mounted"
```

| Part | Element | Key Attributes                                              |
| ---- | ------- | ----------------------------------------------------------- |
| Root | `<div>` | `data-ars-portal-id`, `data-ars-state="unmounted\|mounted"` |

Root is the mount point inserted at the portal target container.

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

- No ARIA roles added. Portal content remains in the accessibility tree because it is in the DOM at the target location.
- Screen readers traverse the DOM, not the component tree, so moved nodes are fully accessible.
- **`aria-owns`:** In rare cases where AT does not follow a moved DOM node (very old JAWS versions), add `aria-owns="<portal-root-id>"` to the trigger. `Api::portal_root_id()` provides this value.

### 3.2 Focus Management

Focus management is the responsibility of the overlay component rendered inside the portal (Dialog, Popover, etc.), not Portal itself. The overlay component handles:

1. Moving focus into the portal content on open
2. Trapping focus within the portal while open (via `FocusTrap` from `ars-a11y`)
3. Returning focus to the trigger on close

## 4. Internationalization

- No translatable strings. Labels are consumer-provided via the overlay component.
- `data-ars-state` values are stable API tokens, not localized.

## 5. Library Parity

> Compared against: Radix UI (`Portal`).

### 5.1 Props

| Feature              | ars-ui                          | Radix UI                  | Notes                                                       |
| -------------------- | ------------------------------- | ------------------------- | ----------------------------------------------------------- |
| Target container     | `container` (PortalTarget enum) | `container` (HTMLElement) | ars-ui has richer target system (PortalRoot, Body, Id, Ref) |
| SSR inline rendering | `ssr_inline`                    | --                        | ars-ui addition                                             |

**Gaps:** None.

### 5.2 Anatomy

| Part | ars-ui | Radix UI | Notes |
| ---- | ------ | -------- | ----- |
| Root | `Root` | `Root`   | --    |

**Gaps:** None.

### 5.3 Events

| Callback      | ars-ui                           | Radix UI | Notes                               |
| ------------- | -------------------------------- | -------- | ----------------------------------- |
| Mount/Unmount | `Event::Mount`, `Event::Unmount` | --       | ars-ui manages lifecycle explicitly |

**Gaps:** None. Radix manages mount/unmount implicitly via React lifecycle.

### 5.4 Features

| Feature                             | ars-ui | Radix UI |
| ----------------------------------- | ------ | -------- |
| Render to different DOM node        | Yes    | Yes      |
| Custom target container             | Yes    | Yes      |
| SSR support                         | Yes    | --       |
| Hydration reattachment              | Yes    | --       |
| Z-index layer management            | Yes    | --       |
| MutationObserver for root stability | Yes    | --       |

**Gaps:** None.

### 5.5 Summary

- **Overall:** Full parity. ars-ui significantly exceeds Radix UI's Portal with SSR support, hydration reattachment, z-index management, and MutationObserver-based root stability.
- **Divergences:** ars-ui uses an enum-based `PortalTarget` instead of a raw DOM element reference, making the API portable across frameworks.
- **Recommended additions:** None.

## Appendix A: SSR Considerations

| Scenario                | Behaviour                                                            |
| ----------------------- | -------------------------------------------------------------------- |
| SSR, `ssr_inline=true`  | Content rendered at declaration site; no portal in HTML.             |
| SSR, `ssr_inline=false` | Nothing rendered; client-side JS mounts the content after hydration. |
| Hydration               | Framework reattaches portal to the correct container.                |
| No-JS / progressive     | Inline content is functional; portals degrade gracefully.            |

### A.1 Hydration Reattachment

```rust
/// Called by the framework hydration layer to reattach a portal node
/// from its inline SSR position to its runtime target.
pub fn hydrate_portal(portal_id: &str, target: &PortalTarget) {
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        log::warn!("hydrate_portal: no document available, skipping");
        return;
    };

    // Escape special characters to prevent CSS selector injection.
    let escaped_id = portal_id.replace('\\', "\\\\").replace('\'', "\\'").replace(']', "\\]");
    let selector = format!("[data-ars-portal-id='{}']", escaped_id);
    let node = document.query_selector(&selector).ok().flatten();

    let container: Option<web_sys::Element> = match target {
        PortalTarget::PortalRoot => get_or_create_portal_root(),
        PortalTarget::Body => document.body().map(|b| b.into()),
        PortalTarget::Id(id) => document.get_element_by_id(id),
        PortalTarget::Ref(id) => document.get_element_by_id(id),
    };

    if let (Some(node), Some(container)) = (node, container) {
        if let Some(old_parent) = node.parent_node() {
            let _ = old_parent.remove_child(&node);
        }
        if let Err(e) = container.append_child(&node) {
            log::warn!("hydrate_portal: failed to reattach portal: {:?}", e);
        }
    }
}
```

Reattachment invariants:

1. Remove from old location before appending to new — adapter references to the old parent are invalid after move.
2. If duplicate `portal_id` is found, log an error and abort.
3. Adapters MUST re-attach event listeners after hydration reattachment completes.

## Appendix B: Portal Root Stability

The portal root element (`#ars-portal-root` or a custom `PortalTarget`) may be removed or relocated by application code, third-party scripts, or framework reconciliation. The adapter MUST observe the portal root for unexpected mutations.

**MutationObserver Setup:**

- Observe the portal root's **parent** with `{ childList: true, subtree: true }` to detect when the portal root itself is removed.
- On detecting removal (the portal root is no longer in `document.body`'s subtree), **clear the cached reference** and log a warning: `"Portal root was removed from DOM; cached reference invalidated."`.

**Debouncing:**

- Limit to **one invalidation check per microtask** using `queueMicrotask()`. Rapid DOM mutations (e.g., framework reconciliation batches) should not trigger multiple cache clears.

**SSR Hydration Edge Cases:**

- During hydration, the portal root may not exist in the SSR-rendered HTML. The `get_or_create_portal_root()` function handles lazy creation, but the MutationObserver MUST NOT be attached until after hydration completes (i.e., after `hydrate_portal()` runs).
- If the portal root is relocated (moved to a different parent) rather than removed, treat this as a removal + re-creation: clear the cache and let the next `resolve_portal_target()` call find the element in its new location.

## Appendix C: Positioning and Stacking

**Default target:** `PortalTarget::PortalRoot` renders into `#ars-portal-root`, a dedicated container appended to `document.body`.

**Custom target:** `PortalTarget::Id(String)` or `PortalTarget::Ref(String)` for rendering into a specific application-managed container (both use element IDs).

**Z-index layer scale:**

| Layer      | Z-index range | Usage                          |
| ---------- | ------------- | ------------------------------ |
| `base`     | 0-99          | Normal document flow           |
| `dropdown` | 1000-1099     | Dropdowns, select menus        |
| `sticky`   | 1100-1199     | Sticky headers, footers        |
| `overlay`  | 1300-1399     | Modals, dialogs                |
| `popover`  | 1400-1499     | Popovers, tooltips             |
| `toast`    | 1500-1599     | Toast notifications            |
| `maximum`  | 1600+         | Topmost elements (debug tools) |

- Adapters MAY expose a `z_index_base: Option<u32>` prop to override the default layer for a given portal instance.
- **Multiple portals:** When multiple portals are mounted simultaneously, they stack in creation order with the newest on top (highest z-index within its layer). Portals created later receive a monotonically increasing z-index offset within their layer.
- **Scroll containment:** Portal content MUST NOT cause `document.body` to scroll. Portaled overlays should use `position: fixed` (relative to viewport) or `position: absolute` (relative to the portal root) to avoid influencing body scroll dimensions.
- **Cleanup:** The portal container element is removed from the DOM when the owning component unmounts. If the portal root has no remaining children, it MAY be left in place (to avoid re-creation cost) but MUST NOT interfere with layout.

## Appendix D: Font Loading Race Conditions

Font loading can cause layout shifts that invalidate overlay positioning calculations (popovers, tooltips, dropdowns rendered via Portal). Adapters MUST account for font loading timing.

**Detection:**

- Listen to `document.fonts.status` via `FontFaceSet` API. If `document.fonts.status === "loading"`, defer initial positioning calculations until `document.fonts.ready` resolves.
- After `document.fonts.ready`, trigger a single recalculation of all active overlay positions.

**Ongoing Font Loads:**

- Register a `loadingdone` event listener on `document.fonts` to recalculate overlay positions when new fonts finish loading (e.g., lazy-loaded font faces triggered by newly rendered content in a portal).

**CSS `font-display` Guidance:**

- Recommend `font-display: swap` in documentation. With `swap`, text renders immediately in a fallback font and reflows on font load — the recalculation listener above handles the resulting layout shift.
- `font-display: block` (invisible text until font loads) may cause overlays to position against zero-height content. The `document.fonts.ready` deferral mitigates this.

**SSR Context:**

- `document.fonts` is unavailable during SSR. Guard all `FontFaceSet` access with the appropriate Rust/WASM feature gate (`#[cfg(target_arch = "wasm32")]`). SSR-rendered overlays will be repositioned on hydration.

## Appendix E: Automatic Repositioning on Resize

Popover and tooltip overlays must reposition automatically when their anchor or viewport changes size:

1. **`ResizeObserver` on Anchor**: Attach a `ResizeObserver` to the anchor element. On resize callback, invoke `compute_position()` to recalculate overlay placement.
2. **`window.resize` Event**: Listen for `window` `resize` events, debounced at 100ms, to trigger repositioning. This handles browser window resizing and desktop display changes.
3. **`visualViewport.resize` for Mobile**: On mobile devices, listen for `window.visualViewport` `resize` events to handle virtual keyboard appearance/disappearance. This event is not debounced (keyboard animations are already brief).
4. **Adapter Contract**: The adapter must call `compute_position()` on each resize event. The core library provides the positioning algorithm; the adapter is responsible for wiring up the platform-specific resize observers and invoking the recomputation.

## Appendix F: iframe Boundary Handling

1. Portal must detect if the target container is inside an iframe. If so, and the iframe is cross-origin, the portal falls back to rendering within the current document (logs a warning).
2. Content portaled near iframe boundaries may be clipped — the positioning engine must account for iframe viewport bounds.
3. Z-index stacking: portaled content inside an iframe inherits the iframe's stacking context; it cannot escape the iframe's z-index layer.
4. Portal to `document.body` is always same-origin and safe.
