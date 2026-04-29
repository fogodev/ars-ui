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
    /// may not exist at `Mount` time). Carries the element ID and is honored
    /// only when it matches the current `PortalTarget::Id`.
    ContainerReady(String),
    /// Synchronize the target container after props change.
    SetContainer(PortalTarget),
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
    /// Runtime render mode resolved by the adapter.
    pub render_mode: RenderMode,
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
    /// A declarative element ID target that the adapter must resolve.
    Id(String),
    /// An element ID target that the adapter has confirmed exists.
    ///
    /// This is still stable string identity, not a live DOM element or
    /// framework handle. Adapters own native refs separately.
    ResolvedId(String),
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

    fn init(props: &Props, env: &Env, _messages: &Messages) -> (State, Context) {
        let ctx = Context {
            container: props.container.clone(),
            mounted: false,
            render_mode: env.render_mode,
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
            (State::Unmounted, Event::ContainerReady(id))
                if matches!(&ctx.container, PortalTarget::Id(target_id) if target_id == id) =>
            {
                let id = id.clone();
                Some(TransitionPlan::to(State::Mounted).apply(move |ctx| {
                    ctx.container = PortalTarget::ResolvedId(id);
                    ctx.mounted = true;
                }))
            }
            (_, Event::SetContainer(target)) => {
                let target = target.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.container = target;
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

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        assert_eq!(
            old.id, new.id,
            "Portal id cannot change after initialization"
        );

        if old.container == new.container {
            Vec::new()
        } else {
            vec![Event::SetContainer(new.container.clone())]
        }
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

    /// Returns the currently resolved portal target.
    pub const fn target(&self) -> &PortalTarget {
        &self.ctx.container
    }

    /// Returns the runtime render mode resolved by the adapter.
    pub const fn render_mode(&self) -> RenderMode {
        self.ctx.render_mode
    }

    /// Returns whether SSR should render portal content inline.
    pub const fn ssr_inline(&self) -> bool {
        self.props.ssr_inline
    }

    /// Returns the stable portal owner ID used by outside-interaction helpers.
    pub fn owner_id(&self) -> &str {
        self.ctx.ids.id()
    }

    /// Returns whether portal content should render inline at the declaration
    /// site for the current runtime mode.
    pub const fn should_render_inline(&self) -> bool {
        self.props.ssr_inline && self.ctx.render_mode.is_server()
    }

    /// The generated portal root element ID, usable for `aria-owns` on triggers.
    pub fn portal_root_id(&self) -> String {
        format!("ars-portal-{}", self.ctx.ids.id())
    }

    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.portal_root_id())
            .set(HtmlAttr::Data("ars-portal-id"), self.ctx.ids.id())
            .set(HtmlAttr::Data("ars-portal-owner"), self.ctx.ids.id())
            .set(
                HtmlAttr::Data("ars-state"),
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
└── Root  <div>  id="ars-portal-<id>" data-ars-scope="portal"
                 data-ars-part="root" data-ars-portal-id="<id>"
                 data-ars-portal-owner="<id>" data-ars-state="unmounted|mounted"
```

| Part | Element | Key Attributes                                                                                               |
| ---- | ------- | ------------------------------------------------------------------------------------------------------------ |
| Root | `<div>` | `id="ars-portal-<id>"`, `data-ars-portal-id`, `data-ars-portal-owner`, `data-ars-state="unmounted\|mounted"` |

Root is the per-instance mount node inserted at the portal target container.
Adapters MUST apply `Api::root_attrs()` to this owned mount node itself. They
MUST NOT create a separate child wrapper with the same root attrs, because that
would duplicate `id="ars-portal-<id>"`.

When `PortalTarget::PortalRoot` is used, the web adapter MUST create or reuse
the per-instance mount node through
`ars_dom::ensure_portal_mount_root(api.owner_id())`, then merge
`Api::root_attrs()` onto that same node. The component root ID stays stable as
`ars-portal-<id>` and carries `data-ars-portal-owner="<id>"` for
outside-interaction boundary detection.

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

| Feature              | ars-ui                          | Radix UI                  | Notes                                                              |
| -------------------- | ------------------------------- | ------------------------- | ------------------------------------------------------------------ |
| Target container     | `container` (PortalTarget enum) | `container` (HTMLElement) | ars-ui has richer target system (PortalRoot, Body, Id, ResolvedId) |
| SSR inline rendering | `ssr_inline`                    | --                        | ars-ui addition                                                    |

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

| Feature                              | ars-ui core contract                                               | Adapter / DOM responsibility                                     | Radix UI |
| ------------------------------------ | ------------------------------------------------------------------ | ---------------------------------------------------------------- | -------- |
| Render to different DOM node         | `PortalTarget` and mount state                                     | Resolve target and move/render content                           | Yes      |
| Custom target container              | `PortalTarget::Body`, `Id`, `ResolvedId`, guarded `ContainerReady` | Locate target and report late matching IDs                       | Yes      |
| SSR inline decision                  | `RenderMode`, `ssr_inline`, `Api::should_render_inline()`          | Set `Env::render_mode` from the framework runtime                | --       |
| Hydration reattachment               | Stable `id`, owner marker, and render-mode introspection           | Reattach/move DOM nodes during hydration                         | --       |
| Outside-interaction portal ownership | `data-ars-portal-owner` on `Api::root_attrs()`                     | Preserve owner markers on any intermediate mount nodes           | --       |
| Root stability                       | Stable target identity and prop-change events                      | Cache invalidation, root creation, MutationObserver, and cleanup | --       |
| Z-index layer management             | No z-index state in Portal                                         | Overlay/positioning layer owns stacking                          | --       |

**Gaps:** None against Radix's portable portal surface. ars-ui intentionally
splits the contract: `ars-components` owns state and adapter-facing metadata;
framework adapters and `ars-dom` own all DOM mutation and platform observation.

### 5.5 Summary

- **Overall:** Full parity for the portal primitive while preserving framework-agnostic layering.
- **Divergences:** ars-ui uses an enum-based `PortalTarget` instead of a raw DOM element reference, making the API portable across frameworks. Native element references stay in adapter APIs and are never stored in `ars-components`.
- **Recommended additions:** None.

## Appendix A: SSR and Hydration Contract

`ars-components::layout::portal` does not inspect crate features directly.
Framework adapters set `Env::render_mode`; Portal exposes
`Api::should_render_inline()` as the portable decision point.

| Scenario                                 | Core decision                     | Adapter behavior                                                        |
| ---------------------------------------- | --------------------------------- | ----------------------------------------------------------------------- |
| `RenderMode::Server`, `ssr_inline=true`  | `should_render_inline() == true`  | Render content at the declaration site with `Api::root_attrs()`.        |
| `RenderMode::Server`, `ssr_inline=false` | `should_render_inline() == false` | Omit portal content from server HTML.                                   |
| `RenderMode::Hydrating`                  | `should_render_inline() == false` | Reattach or move existing server-rendered nodes to the resolved target. |
| `RenderMode::Client`                     | `should_render_inline() == false` | Mount content into the resolved target after receiving `Event::Mount`.  |

Hydration reattachment invariants:

1. Select the portal node by its stable `id="ars-portal-<id>"` or matching `data-ars-portal-id`.
2. If duplicate portal nodes for the same ID are found, log an error and abort reattachment for that instance.
3. Remove from the old location before appending to the target container.
4. Preserve `data-ars-portal-owner="<id>"` on any moved root or intermediate mount node.
5. Reattach framework event listeners according to the adapter runtime's hydration rules.

## Appendix B: Portal Root Stability

Portal core stores target identity and emits `SetContainer` when the container
prop changes. DOM root creation, cache invalidation, and cleanup live in
`ars-dom` and framework adapters.

Adapter and DOM invariants:

1. `PortalTarget::PortalRoot` resolves to the shared host root managed by `ars-dom`.
2. `PortalTarget::Body` resolves to the current document body.
3. `PortalTarget::Id(id)` resolves by element ID. If the element is unavailable, the adapter may wait and dispatch `ContainerReady(id)` only when that exact ID appears.
4. `PortalTarget::ResolvedId(id)` records that the adapter has confirmed the element ID exists. It is not a native element reference and must not be replaced by a different ID.
5. If a cached target is removed or relocated, clear the cache and resolve the target again before the next mount or move.

## Appendix C: Positioning and Stacking

Portal does not own z-index, placement, or layout measurement. Overlay
components rendered inside Portal own positioning policy and use shared
positioning utilities for measurement and auto-update.

Positioning invariants for overlays that use Portal:

1. Use the resolved portal target only as the render container.
2. Compute overlay placement relative to the trigger/anchor and viewport, not relative to the logical component tree.
3. Preserve scroll containment by using positioning strategies that do not expand `document.body`.
4. Keep z-index policy in overlay or layer-management components, not in Portal state.

## Appendix D: Font Loading Race Conditions

Font loading invalidates overlay measurements, but it does not affect the
Portal state machine. Adapters or overlay positioning utilities must trigger
recalculation when font metrics change.

Platform invariants:

1. On web targets with `FontFaceSet`, defer initial overlay positioning when `document.fonts.status == "loading"`.
2. Recalculate active overlay positions once `document.fonts.ready` resolves.
3. Recalculate on later `loadingdone` events for lazy-loaded fonts.
4. Guard all font API access behind the appropriate web-target checks.

## Appendix E: Automatic Repositioning on Resize

Resize handling belongs to overlay positioning utilities, not Portal. For
portaled popovers, tooltips, menus, and similar overlays:

1. Observe anchor size changes.
2. Observe viewport size changes.
3. Observe mobile visual viewport changes where available.
4. Recompute placement through the shared positioning engine.
5. Clean up observers when the owning overlay unmounts.

## Appendix F: iframe Boundary Handling

iframe behavior is a DOM target-resolution concern:

1. Same-document targets are safe to resolve normally.
2. Same-origin iframe targets may be resolved by adapter/DOM utilities that have access to that frame's document.
3. Cross-origin iframe targets cannot be inspected or moved into; adapters must keep content in the current document and log a warning.
4. Portaled content inside an iframe inherits that iframe's stacking and clipping boundaries.
