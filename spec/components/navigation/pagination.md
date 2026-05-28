---
component: Pagination
category: navigation
tier: stateful
foundation_deps: [architecture, accessibility, interactions]
shared_deps: []
related: []
references:
    ark-ui: Pagination
---

# Pagination

Page navigation controls for breaking long content into discrete pages. Generates page ranges
with ellipsis for large page counts.

## 1. State Machine

### 1.1 States

`Pagination` is effectively stateless — its only meaningful state is the current page, tracked
in context. A single `Idle` state is used.

The effective current page is always clamped into `1..=page_count`, including controlled
`page` props. `Context::page.get()` and `Api::current_page()` must not expose an out-of-range
page when `page_count` changes.

| State  | Description                                                 |
| ------ | ----------------------------------------------------------- |
| `Idle` | The only machine state; current page is in `Context::page`. |

### 1.2 Events

| Event                       | Payload               | Description                                           |
| --------------------------- | --------------------- | ----------------------------------------------------- |
| `GoToPage(u32)`             | page number (1-based) | Navigate to a specific page.                          |
| `NextPage`                  | —                     | Advance to page + 1 (capped at `page_count`).         |
| `PrevPage`                  | —                     | Go back to page - 1 (floored at 1).                   |
| `GoToFirstPage`             | —                     | Jump to page 1.                                       |
| `GoToLastPage`              | —                     | Jump to `page_count`.                                 |
| `SetPageSize(NonZero<u32>)` | new page size         | Change items-per-page; resets or clamps current page. |
| `SyncProps`                 | —                     | Synchronize controlled props into context.            |

### 1.3 Context

```rust
use ars_core::Bindable;

/// Context for the `Pagination` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Current page — controlled or uncontrolled. 1-based.
    pub page: Bindable<u32>,
    /// Number of items per page.
    pub page_size: NonZero<u32>,
    /// Total number of items being paginated.
    pub total_items: u32,
    /// Number of page buttons shown on each side of the current page.
    pub sibling_count: u32,
    /// Number of always-visible page buttons at each boundary.
    pub boundary_count: u32,
    /// Derived: `ceil(total_items / page_size)`, always >= 1.
    pub page_count: u32,
    /// Resolved locale for i18n.
    pub locale: Locale,
    /// Generated element IDs.
    pub ids: ComponentIds,
    /// Resolved messages for accessibility labels.
    pub messages: Messages,
}

impl Context {
    /// Compute the derived page_count from total_items / page_size.
    pub fn compute_page_count(total_items: u32, page_size: NonZero<u32>) -> u32 {
        total_items.div_ceil(page_size.get()).max(1)
    }

    /// Returns the effective one-based page, clamped into `1..=page_count`.
    ///
    /// `page.get()` may hold an out-of-range value when `page_count` shrinks
    /// (especially for a controlled `Bindable`, whose `get()` returns the
    /// unclamped parent value). This accessor is the single source of truth for
    /// the rendered page, satisfying the §1.1 mandate that no out-of-range page
    /// is ever exposed.
    pub fn current_page(&self) -> u32 {
        clamp_page(*self.page.get(), self.page_count)
    }

    /// Generate the list of pages to display, inserting `None` for ellipsis.
    ///
    /// Example (page=5, page_count=10, sibling_count=1, boundary_count=1):
    ///   [Some(1), None, Some(4), Some(5), Some(6), None, Some(10)]
    pub fn page_range(&self) -> Vec<Option<u32>> {
        page_range(self.current_page(), self.page_count, self.sibling_count, self.boundary_count)
    }
}
```

### 1.4 Props

```rust
/// Props for the `Pagination` component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Unique component identifier.
    pub id: String,
    /// Controlled current page (1-based).
    pub page: Option<u32>,
    /// Default page (1-based).
    pub default_page: u32,
    /// Number of items per page.
    pub page_size: NonZero<u32>,
    /// Total number of items being paginated.
    pub total_items: u32,
    /// Pages shown on each side of current page in the range.
    pub sibling_count: u32,
    /// Number of always-visible page buttons at the start and end of the range.
    /// Default: 1.
    pub boundary_count: u32,
    /// Visual size variant for pagination controls.
    /// Affects button dimensions, font size, and spacing.
    /// Default: `Size::Medium`.
    pub size: Size,
    /// Optional URL generator for link-based pagination. When `Some`, page
    /// buttons render as `<a href="...">` instead of `<button>`, enabling
    /// progressive enhancement and SEO-friendly pagination. The callback
    /// receives a 1-based page number and returns the URL string.
    pub get_page_url: Option<Callback<dyn Fn(u32) -> String + Send + Sync>>,
    /// Callback fired after the current page changes.
    pub on_page_change: Option<Callback<dyn Fn(u32) + Send + Sync>>,
}

/// Visual size variants for Pagination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Size {
    /// Compact pagination with smaller buttons. Suitable for dense UIs.
    Compact,
    /// Standard pagination size.
    #[default]
    Medium,
    /// Large pagination with bigger touch targets. Suitable for mobile.
    Large,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            page: None,
            default_page: 1,
            page_size: NonZero::new(10).expect("non-zero"),
            total_items: 0,
            sibling_count: 1,
            boundary_count: 1,
            size: Size::default(),
            get_page_url: None,
            on_page_change: None,
        }
    }
}
```

### 1.5 Full Machine Implementation

```rust
use ars_core::{TransitionPlan, PendingEffect, Bindable, AttrMap};

/// States for the `Pagination` component.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// The pagination is in the idle state.
    Idle,
}

/// Events for the `Pagination` component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Navigate to a specific page.
    GoToPage(u32),
    /// Advance to page + 1 (capped at `page_count`).
    NextPage,
    /// Go back to page - 1 (floored at 1).
    PrevPage,
    /// Jump to page 1.
    GoToFirstPage,
    /// Jump to `page_count`.
    GoToLastPage,
    /// Change items-per-page; resets or clamps current page.
    SetPageSize(NonZero<u32>),
    /// Synchronize controlled props into context.
    SyncProps,
}

/// Effects for the `Pagination` component.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// The current page changed.
    PageChange,
}

/// Machine for the `Pagination` component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State   = State;
    type Event   = Event;
    type Context = Context;
    type Props   = Props;
    type Effect  = Effect;
    type Api<'a> = Api<'a>;
    type Messages = Messages;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let page_count = Context::compute_page_count(props.total_items, props.page_size);
        let initial_page = props
            .page
            .unwrap_or(props.default_page)
            .max(1)
            .min(page_count);
        let page = match props.page {
            Some(_) => Bindable::controlled(initial_page),
            None    => Bindable::uncontrolled(initial_page),
        };
        let ids = ComponentIds::from_id(&props.id);
        let locale = env.locale.clone();
        let messages = messages.clone();
        (State::Idle, Context {
            page,
            page_size: props.page_size,
            total_items: props.total_items,
            sibling_count: props.sibling_count,
            boundary_count: props.boundary_count,
            page_count,
            locale,
            ids,
            messages,
        })
    }

    fn transition(
        _state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        // Always navigate relative to the *clamped* current page, never the raw
        // `page.get()` (which may be out of range — see §1.1 and `current_page`).
        let current = ctx.current_page();
        match event {
            Event::GoToPage(p) => {
                page_change_plan(ctx, props, clamp_page(*p, ctx.page_count))
            }
            Event::NextPage => {
                page_change_plan(ctx, props, clamp_page(current.saturating_add(1), ctx.page_count))
            }
            Event::PrevPage => {
                page_change_plan(ctx, props, clamp_page(current.saturating_sub(1), ctx.page_count))
            }
            Event::GoToFirstPage => page_change_plan(ctx, props, 1),
            Event::GoToLastPage => page_change_plan(ctx, props, ctx.page_count),
            Event::SetPageSize(size) => {
                let new_size  = *size;
                let new_count = Context::compute_page_count(ctx.total_items, new_size);
                // For a controlled `page`, `page.get()` reflects only the
                // internal value; the parent's intended page lives in
                // `props.page`. Clamp *that* across the resize so a shrink ->
                // expand round-trip restores the parent's page rather than the
                // value it was temporarily clamped to.
                let raw_current = props.page.unwrap_or(*ctx.page.get());
                let target = clamp_page(raw_current, new_count);
                let emit = target != current;
                let mut plan = TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.page_size  = new_size;
                    ctx.page_count = new_count;
                    if ctx.page.is_controlled() {
                        ctx.page.sync_controlled(Some(target));
                    }
                    ctx.page.set(target);
                });
                // Emit `PageChange` only when the rendered page actually moves.
                if emit {
                    plan = with_page_change_effect(plan, target);
                }
                Some(plan)
            }
            Event::SyncProps => {
                let page_size = props.page_size;
                let total_items = props.total_items;
                let sibling_count = props.sibling_count;
                let boundary_count = props.boundary_count;
                let page_count = Context::compute_page_count(total_items, page_size);
                let controlled = props.page.map(|page| clamp_page(page, page_count));
                let target = controlled.unwrap_or_else(|| clamp_page(current, page_count));
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.page_size = page_size;
                    ctx.total_items = total_items;
                    ctx.sibling_count = sibling_count;
                    ctx.boundary_count = boundary_count;
                    ctx.page_count = page_count;
                    ctx.page.sync_controlled(controlled);
                    ctx.page.set(target);
                }))
            }
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
        // Re-sync context whenever any page-affecting prop changes (controlled
        // `page`, `default_page`, `page_size`, `total_items`, or the range-shape
        // counts). Anything else (e.g. `size`, callbacks) needs no transition.
        if old.page != new.page
            || old.default_page != new.default_page
            || old.page_size != new.page_size
            || old.total_items != new.total_items
            || old.sibling_count != new.sibling_count
            || old.boundary_count != new.boundary_count
        {
            vec![Event::SyncProps]
        } else {
            Vec::new()
        }
    }
}

/// Clamp a one-based page into `1..=page_count` (treating `page_count` as
/// at least 1).
fn clamp_page(page: u32, page_count: u32) -> u32 {
    page.max(1).min(page_count.max(1))
}

/// Build the transition for any page-mutating event. Returns `None` (no
/// transition, no effect) when the clamped target equals the current page, so
/// `on_page_change` never fires for a no-op move. When the page does change,
/// the plan carries the `PageChange` effect.
fn page_change_plan(ctx: &Context, _props: &Props, target: u32) -> Option<TransitionPlan<Machine>> {
    if target == ctx.current_page() {
        return None;
    }
    Some(with_page_change_effect(
        TransitionPlan::context_only(move |ctx: &mut Context| {
            ctx.page.set(target);
        }),
        target,
    ))
}

/// Attach the `PageChange` effect, firing `on_page_change(target)`.
///
/// The callback receives `target` — the *requested* page — not
/// `ctx.page.get()`. For a controlled `Bindable`, `set()` updates only the
/// internal value while `get()` keeps returning the unchanged parent value, so
/// reading `get()` here would report the stale page. Capturing `target` keeps
/// the callback correct for both controlled and uncontrolled bindings.
fn with_page_change_effect(
    mut plan: TransitionPlan<Machine>,
    target: u32,
) -> TransitionPlan<Machine> {
    plan = plan.with_effect(PendingEffect::new(
        Effect::PageChange,
        move |_ctx: &Context, props: &Props, _send| {
            if let Some(callback) = &props.on_page_change {
                (callback)(target);
            }
            no_cleanup()
        },
    ));
    plan
}
```

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "pagination"]
pub enum Part {
    Root,
    PrevTrigger,
    NextTrigger,
    PageTrigger { page_number: u32 },
    Ellipsis,
}

/// API for the `Pagination` component.
pub struct Api<'a> {
    /// The state of the component.
    state: &'a State,
    /// The context of the component.
    ctx:   &'a Context,
    /// The props of the component.
    props: &'a Props,
    /// The send function to send events to the component.
    send:  &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Get the current page number.
    pub fn current_page(&self) -> u32  { *self.ctx.page.get() }

    /// Get the total number of pages.
    pub fn page_count(&self)   -> u32  { self.ctx.page_count }

    /// Check if the current page is the first page.
    pub fn is_first_page(&self) -> bool { self.current_page() == 1 }

    /// Check if the current page is the last page.
    pub fn is_last_page(&self)  -> bool { self.current_page() == self.page_count() }

    /// Attrs for the `<nav>` root element.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "navigation");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.root_label)(&self.ctx.locale));
        attrs.set(HtmlAttr::Data("ars-size"), self.props.size.as_str());
        attrs
    }

    /// Attrs for the previous-page button.
    pub fn prev_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let disabled = self.is_first_page();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::PrevTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.prev_label)(&self.ctx.locale));
        if disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }
        attrs
    }

    /// Handles a click event on the previous-page button.
    pub fn on_prev_trigger_click(&self) {
        if !self.is_first_page() { (self.send)(Event::PrevPage); }
    }

    /// Attrs for the next-page button.
    pub fn next_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let disabled = self.is_last_page();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::NextTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.next_label)(&self.ctx.locale));
        if disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }
        attrs
    }

    /// Handles a click event on the next-page button.
    pub fn on_next_trigger_click(&self) {
        if !self.is_last_page() { (self.send)(Event::NextPage); }
    }

    /// Attrs for an individual page number button.
    ///
    /// `page_number` — the 1-based page this button represents.
    pub fn page_trigger_attrs(&self, page_number: u32) -> AttrMap {
        let mut attrs = AttrMap::new();
        let is_current = self.current_page() == page_number;
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::PageTrigger { page_number }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);

        // When get_page_url is set, render as <a> for progressive enhancement.
        // Otherwise render as <button>.
        if let Some(ref get_url) = self.props.get_page_url {
            attrs.set(HtmlAttr::Href, sanitize_url(&get_url(page_number)));
        } else {
            attrs.set(HtmlAttr::Type, "button");
        }

        attrs.set(HtmlAttr::Data("ars-index"), page_number.to_string());
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.page_label)(page_number as usize, &self.ctx.locale));
        if is_current {
            attrs.set(HtmlAttr::Aria(AriaAttr::Current), "page");
            attrs.set_bool(HtmlAttr::Data("ars-current"), true);
        }
        attrs
    }

    /// Handles a click event on an individual page number button.
    pub fn on_page_trigger_click(&self, page_number: u32) {
        (self.send)(Event::GoToPage(page_number));
    }

    /// Attrs for an ellipsis element (non-interactive gap indicator).
    ///
    /// The ellipsis MUST NOT be focusable or interactive. The adapter renders it as:
    ///   `<span aria-hidden="true">…</span>`
    /// to hide the visual "…" character from screen readers. Alongside each ellipsis,
    /// the adapter MUST render a visually hidden summary of the skipped page range
    /// for screen reader users:
    ///   `<span class="sr-only">Pages {start} through {end}</span>`
    /// This ensures assistive technology users understand which pages are omitted.
    pub fn ellipsis_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Ellipsis.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs.set(HtmlAttr::Role, "separator");
        attrs
    }

    /// Generate the page range for rendering: `None` entries represent ellipsis.
    pub fn page_range(&self) -> Vec<Option<u32>> {
        self.ctx.page_range()
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::PrevTrigger => self.prev_trigger_attrs(),
            Part::NextTrigger => self.next_trigger_attrs(),
            Part::PageTrigger { page_number } => self.page_trigger_attrs(page_number),
            Part::Ellipsis => self.ellipsis_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
Pagination
├── Root                   <nav> aria-label="Pagination"
├── PrevTrigger            <button> aria-label="Go to previous page"
├── PageTrigger (×N)       <button> data-ars-index, aria-current="page"
├── Ellipsis               <span> aria-hidden="true"
└── NextTrigger            <button> aria-label="Go to next page"
```

| Part          | Element    | Key Attributes                                                                                                                                                           |
| ------------- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `Root`        | `<nav>`    | `data-ars-scope="pagination"`, `data-ars-part="root"`, `role="navigation"`, `aria-label="Pagination"`                                                                    |
| `PrevTrigger` | `<button>` | `data-ars-scope="pagination"`, `data-ars-part="prev-trigger"`, `aria-label="Go to previous page"`, `aria-disabled`, `data-ars-disabled`                                  |
| `NextTrigger` | `<button>` | `data-ars-scope="pagination"`, `data-ars-part="next-trigger"`, `aria-label="Go to next page"`, `aria-disabled`, `data-ars-disabled`                                      |
| `PageTrigger` | `<button>` | `data-ars-scope="pagination"`, `data-ars-part="page-trigger"`, `data-ars-index="{n}"`, `aria-label="Page {n}"`, `aria-current="page"` (current only), `data-ars-current` |
| `Ellipsis`    | `<span>`   | `data-ars-scope="pagination"`, `data-ars-part="ellipsis"`, `aria-hidden="true"`, `role="separator"`                                                                      |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part          | Role                       | Properties                                                               |
| ------------- | -------------------------- | ------------------------------------------------------------------------ |
| `Root`        | `navigation` (via `<nav>`) | `aria-label="Pagination"` (localized)                                    |
| `PrevTrigger` | `button` (native)          | `aria-label="Go to previous page"`, `aria-disabled="true"` on first page |
| `NextTrigger` | `button` (native)          | `aria-label="Go to next page"`, `aria-disabled="true"` on last page      |
| `PageTrigger` | `button` (native)          | `aria-label="Page {n}"`, `aria-current="page"` for the active page       |
| `Ellipsis`    | `separator`                | `aria-hidden="true"`                                                     |

> **`disabled` vs `aria-disabled`:** Use `aria-disabled="true"` on non-native elements (e.g., `<a>`, `<span>`) to keep them focusable for screen reader discoverability. Use the `disabled` HTML attribute only on native `<button>` elements. When pagination triggers are rendered as links (`<a>`), always prefer `aria-disabled="true"` over the `disabled` attribute, which has no effect on anchors.

### 3.2 Keyboard Interaction

Pagination uses standard browser tab navigation — each button is in the natural tab order.
No custom keyboard shortcuts are needed beyond native button behavior (Enter/Space activate
the focused button).

## 4. Internationalization

### 4.1 Messages

```rust
#[derive(Clone, Debug)]
pub struct Messages {
    /// Root label (default: "Pagination")
    pub root_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Previous trigger label (default: "Go to previous page")
    pub prev_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Next trigger label (default: "Go to next page")
    pub next_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Page trigger label template (default: "Page {n}")
    pub page_label: MessageFn<dyn Fn(usize, &Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            root_label: MessageFn::static_str("Pagination"),
            prev_label: MessageFn::static_str("Go to previous page"),
            next_label: MessageFn::static_str("Go to next page"),
            page_label: MessageFn::new(|n, _locale| format!("Page {n}")),
        }
    }
}

impl ComponentMessages for Messages {}
```

All `aria-label` values in Pagination triggers MUST read from this struct.

### 4.2 Page Change Announcements

When the user navigates to a different page, the adapter MUST announce the new page to screen readers via a `LiveRegion` with `aria-live="polite"`. The announcement text uses the `page_change_announcement` message.

**Implementation:** The adapter renders a visually-hidden `LiveRegion` element inside the Pagination root. When the `SetPage` event transitions to a new page, the adapter inserts the announcement text using the two-step pattern defined in `spec/components/utility/live-region.md` (clear → 100ms delay → insert).

```rust,no_check
// Additional Messages field:
/// Page change announcement (default: "Page {current} of {total}")
pub page_change_announcement: MessageFn<dyn Fn(usize, usize, &Locale) -> String + Send + Sync>,
```

```rust,no_check
// In Default impl, add:
page_change_announcement: MessageFn::new(|current, total, _locale| {
    format!("Page {current} of {total}")
}),
```

The announcement is triggered by the `SetPage` effect, not by the initial render. The first page load does not announce (the page content itself is the announcement context on load).

- **Labels**: All labels are provided via `Messages`. The `root_props()` and trigger prop methods
  read from this struct.
- **Numeric formatting**: Page numbers are formatted using the locale's numeral system (e.g.,
  Arabic-Indic digits in Arabic locales). Delegated to `ars-i18n::format_number`.
- **RTL**: The visual order of prev/next buttons reverses in RTL layouts via CSS
  (`[dir="rtl"]`); the machine and API are direction-agnostic.

## 5. Library Parity

> Compared against: Ark UI (`Pagination`).

### 5.1 Props

| Feature             | ars-ui           | Ark UI                     | Notes                                   |
| ------------------- | ---------------- | -------------------------- | --------------------------------------- |
| Total items         | `total_items`    | `count`                    | Same concept                            |
| Controlled page     | `page`           | `page`                     | Full match                              |
| Default page        | `default_page`   | `defaultPage`              | Full match                              |
| Page size           | `page_size`      | `pageSize`                 | Full match                              |
| Sibling count       | `sibling_count`  | `siblingCount`             | Full match                              |
| Boundary count      | `boundary_count` | `boundaryCount`            | Full match                              |
| URL generator       | `get_page_url`   | `getPageUrl`               | Full match                              |
| Type (button/link)  | --               | `type: 'button' \| 'link'` | ars-ui uses `get_page_url` to determine |
| Translations / i18n | `messages`       | `translations`             | Full match                              |
| Size variant        | `size`           | --                         | ars-ui addition                         |
| Default page size   | --               | `defaultPageSize`          | See below                               |

**Gaps:**

- **`defaultPageSize`**: Ark UI supports uncontrolled page size with a `defaultPageSize` prop. ars-ui's `page_size` is always a direct prop (not Bindable). This is a low-value gap since page size changes are typically controlled by the consumer. Not worth adding Bindable for page size.

### 5.2 Anatomy

| Part          | ars-ui           | Ark UI         | Notes      |
| ------------- | ---------------- | -------------- | ---------- |
| Root          | `Root` (`<nav>`) | `Root`         | Full match |
| Prev trigger  | `PrevTrigger`    | `PrevTrigger`  | Full match |
| Next trigger  | `NextTrigger`    | `NextTrigger`  | Full match |
| Page trigger  | `PageTrigger`    | `Item`         | Full match |
| Ellipsis      | `Ellipsis`       | `Ellipsis`     | Full match |
| First trigger | --               | `FirstTrigger` | See below  |
| Last trigger  | --               | `LastTrigger`  | See below  |

**Gaps:**

- **`FirstTrigger` / `LastTrigger`**: Ark UI has dedicated first-page and last-page trigger parts. ars-ui has `GoToFirstPage` and `GoToLastPage` events but no dedicated anatomy parts for them. The consumer can render custom first/last buttons using the existing event handlers. Low priority.

### 5.3 Events

| Callback         | ars-ui              | Ark UI             | Notes                        |
| ---------------- | ------------------- | ------------------ | ---------------------------- |
| Page change      | `Bindable` onChange | `onPageChange`     | ars-ui uses Bindable pattern |
| Page size change | `SetPageSize` event | `onPageSizeChange` | Full match                   |

**Gaps:** None.

### 5.4 Features

| Feature                    | ars-ui       | Ark UI               |
| -------------------------- | ------------ | -------------------- |
| Page range with ellipsis   | Yes          | Yes                  |
| First/Last page navigation | Yes (events) | Yes (events + parts) |
| Page size change           | Yes          | Yes                  |
| Boundary count             | Yes          | Yes                  |
| Sibling count              | Yes          | Yes                  |
| URL generation (SEO)       | Yes          | Yes                  |
| Live region announcement   | Yes          | No                   |
| Size variants              | Yes          | No                   |

**Gaps:** None.

### 5.5 Summary

- **Overall:** Full parity.
- **Divergences:** Ark UI has dedicated `FirstTrigger`/`LastTrigger` anatomy parts; ars-ui provides the events but leaves part rendering to the consumer. Ark UI uses `type: 'button' | 'link'` to switch rendering; ars-ui infers this from `get_page_url` presence.
- **Recommended additions:** None. Adding `FirstTrigger` / `LastTrigger` parts would be a minor ergonomic improvement but is not a functional gap.
