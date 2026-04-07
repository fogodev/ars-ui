---
component: FocusScope
category: utility
tier: stateful
foundation_deps: [architecture, accessibility]
shared_deps: []
related: []
references:
    react-aria: FocusScope
---

# FocusScope

FocusScope constrains keyboard Tab focus within a container, enabling focus trapping for
modal dialogs, drawers, and other overlay components that must prevent focus from escaping.

## 1. State Machine

### 1.1 States

| State                      | Description                                                    |
| -------------------------- | -------------------------------------------------------------- |
| `Inactive`                 | Focus scope is idle; Tab behavior is unmodified.               |
| `Active { trapped: bool }` | Focus scope is active. `trapped=true` means Tab cannot escape. |

### 1.2 Events

| Event          | Payload               | Description                                                      |
| -------------- | --------------------- | ---------------------------------------------------------------- |
| `Activate`     | `trapped: bool`       | Activate the focus scope, optionally trapping focus.             |
| `Deactivate`   | `restore_focus: bool` | Deactivate the scope and optionally restore previous focus.      |
| `TrapFocus`    | —                     | Enable focus trapping on an active scope.                        |
| `ReleaseTrap`  | —                     | Disable focus trapping on an active scope.                       |
| `RestoreFocus` | —                     | Restore focus to the element that was focused before activation. |
| `FocusFirst`   | —                     | Move focus to the first tabbable element in the container.       |
| `FocusLast`    | —                     | Move focus to the last tabbable element in the container.        |

### 1.3 Context

```rust
/// The states for the `FocusScope` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Focus scope is idle; Tab behavior is unmodified.
    Inactive,
    /// Focus scope is active. `trapped=true` means Tab cannot escape.
    Active {
        /// When true, Tab cannot escape the container.
        trapped: bool,
    },
}

/// The events for the `FocusScope` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Activate the focus scope, optionally trapping focus.
    /// The adapter captures the currently focused element ID before sending this event
    /// (via `platform.active_element_id()`) so the machine can store it for later restoration.
    Activate {
        /// When true, Tab cannot escape the container.
        trapped: bool,
        /// ID of the element that had focus before activation (for restore-on-deactivate).
        /// Captured by the adapter via `platform.active_element_id()`.
        saved_focus_id: Option<String>,
    },
    /// Deactivate the scope and optionally restore previous focus.
    Deactivate {
        /// When true, restore focus to the previously focused element.
        restore_focus: bool,
    },
    /// Enable focus trapping on an active scope.
    TrapFocus,
    /// Disable focus trapping on an active scope.
    ReleaseTrap,
    /// Restore focus to the element that was focused before activation.
    /// Only processed during deactivation (via `then_send` from Deactivate).
    /// Ignored if the scope is still Active.
    RestoreFocus,
    /// Move focus to the first tabbable element in the container.
    FocusFirst,
    /// Move focus to the last tabbable element in the container.
    FocusLast,
}

// FocusScope props (`trapped`, `contain`, `auto_focus`, `restore_focus`) are read
// at activation time and are immutable during the active lifecycle. To change
// trapping behavior, deactivate and reactivate the scope.

/// The context for the `FocusScope` component.
///
/// **Note:** `active` and `trapped` are NOT stored in context. They are derived
/// from `State` in the connect API:
/// - `is_active()` → `matches!(state, State::Active { .. })`
/// - `is_trapped()` → `matches!(state, State::Active { trapped: true })`
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The element that had focus before the scope was activated.
    /// Restored on `Deactivate` if `restore_focus=true`.
    pub saved_focus: Option<String>,
    /// The DOM ID of the container element that scopes focus.
    pub container_id: Option<String>,
}
```

### 1.4 Props

```rust
/// Props for the `FocusScope` component.
#[derive(Clone, Debug, PartialEq, Eq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Prevent Tab from moving focus outside the container.
    pub trapped: bool,
    /// Alias for trapped (clearer naming in some contexts).
    pub contain: bool,
    /// On activation, automatically move focus to the first tabbable element
    /// (or the element with autofocus attribute if present).
    pub auto_focus: bool,
    /// On deactivation, restore focus to the previously focused element.
    pub restore_focus: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            trapped: false,
            contain: false,
            auto_focus: true,
            restore_focus: true,
        }
    }
}
```

### 1.5 Transitions

```text
Inactive + Activate { trapped }
  → Active { trapped: trapped || contain }
  action: save currently focused element → ctx.saved_focus
  effect: "focus-trap-listener" (attaches keydown handler to intercept Tab)
  then_send: FocusFirst (if auto_focus=true)

Active + Deactivate { restore_focus }
  → Inactive
  action: clear ctx.saved_focus reference
  cleanup effect: remove keydown handler
  then_send: RestoreFocus (if restore_focus=true)

Active { trapped: false } + TrapFocus
  → Active { trapped: true }

Active { trapped: true } + ReleaseTrap
  → Active { trapped: false }

Inactive + RestoreFocus
  → Inactive (stay)
  action: restore focus from ctx.saved_focus via restore_focus_safely()

Active + RestoreFocus
  → None (ignored — RestoreFocus is only meaningful after Deactivate)

Active + FocusFirst
  → Active (stay)
  effect: focus first tabbable element in container

Active + FocusLast
  → Active (stay)
  effect: focus last tabbable element in container

When `contain` is true and no tabbable elements exist within the scope, FocusScope MUST:
  (1) keep focus on the container element (which has `tabindex="-1"`),
  (2) suppress Tab/Shift+Tab key events entirely (`preventDefault()`),
  (3) re-scan for tabbable elements on each Tab press to detect dynamically added
      content (e.g., lazy-loaded dialog body).
  (handled entirely in effect, no state change needed)
```

### 1.6 Full Machine Implementation

```rust
use ars_core::{TransitionPlan, PendingEffect, ConnectApi, AttrMap};

/// The machine for the `FocusScope` component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Api<'a> = Api<'a>;

    fn init(props: &Props) -> (State, Context) {
        (
            State::Inactive,
            Context {
                saved_focus: None,
                container_id: None,
            },
        )
    }

    fn transition(
        state: &State,
        event: &Event,
        ctx: &Context,
        props: &Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            // ── Activation ──────────────────────────────────────────────
            (State::Inactive, Event::Activate { trapped, saved_focus_id }) => {
                let trap = *trapped || props.contain;
                let auto_focus = props.auto_focus;
                let saved = saved_focus_id.clone();
                let mut plan = TransitionPlan::to(State::Active { trapped: trap })
                    .apply(move |ctx| {
                        ctx.saved_focus = saved;
                    })
                    .with_named_effect("focus-trap-listener", |ctx, _props, send| {
                        let platform = use_platform_effects();
                        let container_id = ctx.ids.part("container");
                        platform.attach_focus_trap(&container_id, Box::new(move || {
                            send.call_if_alive(Event::Deactivate { restore_focus: true });
                        }))
                    });
                if auto_focus {
                    plan = plan.then(Event::FocusFirst);
                }
                Some(plan)
            }

            // ── Deactivation ────────────────────────────────────────────
            (State::Active { .. }, Event::Deactivate { restore_focus }) => {
                let restore = *restore_focus;
                let mut plan = TransitionPlan::to(State::Inactive);
                if restore {
                    // RestoreFocus will read saved_focus then clear it.
                    plan = plan.then(Event::RestoreFocus);
                } else {
                    plan = plan.apply(|ctx| {
                        ctx.saved_focus = None; // Clear to prevent stale DOM references
                    });
                }
                Some(plan)
            }

            // ── Trap / Release ──────────────────────────────────────────
            (State::Active { trapped: false }, Event::TrapFocus) => {
                Some(TransitionPlan::to(State::Active { trapped: true }))
            }
            (State::Active { trapped: true }, Event::ReleaseTrap) => {
                Some(TransitionPlan::to(State::Active { trapped: false }))
            }

            // ── RestoreFocus ────────────────────────────────────────────
            // Only processed when Inactive (after Deactivate via then_send).
            (State::Inactive, Event::RestoreFocus) => {
                Some(TransitionPlan::context_only(|ctx| {
                    if let Some(ref target) = ctx.saved_focus {
                        restore_focus_safely(target.clone(), &[]);
                    }
                    ctx.saved_focus = None;
                }))
            }
            // If RestoreFocus arrives while Active, ignore it.
            (State::Active { .. }, Event::RestoreFocus) => None,

            // ── Focus Navigation ────────────────────────────────────────
            (State::Active { .. }, Event::FocusFirst) => {
                Some(TransitionPlan::new()
                    .with_named_effect("focus_first", |ctx, _props, _send| {
                        let platform = use_platform_effects();
                        if let Some(ref id) = ctx.container_id {
                            platform.focus_first_tabbable(id);
                        }
                    }))
            }
            (State::Active { .. }, Event::FocusLast) => {
                Some(TransitionPlan::new()
                    .with_named_effect("focus_last", |ctx, _props, _send| {
                        let platform = use_platform_effects();
                        if let Some(ref id) = ctx.container_id {
                            platform.focus_last_tabbable(id);
                        }
                    }))
            }

            _ => None,
        }
    }

    // **Modality coordination:** programmatic focus restoration MUST preserve the
    // shared `ModalityContext` state rather than forcing pointer modality.
    // This ensures `data-ars-focus-visible` remains correct — programmatic focus
    // should only show a focus ring when the prior interaction was not pointer-driven.

    fn connect<'a>(
        state: &'a State,
        ctx: &'a Context,
        props: &'a Props,
        send: &'a dyn Fn(Event),
    ) -> Api<'a> {
        Api { state, ctx, props, send }
    }
}
```

#### 1.6.1 Focus Restoration Safety

````rust
// Guard against restoring focus to a removed or unfocusable element.
//
// Checks performed before restoring focus:
// 1. Element is in the DOM (document.contains)
// 2. Element is visible (not visibility:hidden, not display:none)
// 3. Element is not inside a closed <details> element
// 4. Element is not already the active element (document.activeElement)
// 5. Element can receive focus (is tabbable or has tabindex)
// 6. Element has layout (offsetParent != null)
//
// If the target fails any check, try each fallback in order, then the
// nearest focusable ancestor, then the document body.
//
// The `fallbacks` parameter supports nested dialog scenarios where the
// original trigger may have been removed. Callers pass a prioritized list
// (e.g., [parent_dialog_last_focused, parent_dialog_container,
// parent_dialog_first_focusable]) so the function can gracefully degrade.
fn restore_focus_safely(target_id: &str, fallback_ids: &[&str]) {
    let platform = use_platform_effects();
    if platform.can_restore_focus(target_id) {
        platform.focus_element_by_id(target_id);
        return;
    }
    // Try fallback elements in order
    for id in fallback_ids {
        if platform.can_restore_focus(id) {
            platform.focus_element_by_id(id);
            return;
        }
    }
    // Walk up to nearest focusable ancestor of the original target.
    if let Some(ancestor_id) = platform.nearest_focusable_ancestor_id(target_id) {
        platform.focus_element_by_id(&ancestor_id);
    } else {
        // Last resort — focus document body
        platform.focus_body();
    }
}

// ── Orientation Change Focus Audit ──────────────────────────────────────
//
// When the viewport orientation changes (e.g., portrait → landscape on mobile),
// CSS media queries may hide or show elements. If FocusScope has trapped focus
// on an element that is now hidden by `display: none`, the element stays in
// the DOM but is invisible and has `offsetParent == null`.
//
// The FocusScope Tab handler MUST check `offsetParent !== null` before allowing
// focus on any element. Additionally, the adapter MUST register a
// `matchMedia('(orientation: portrait)')` change listener that triggers a
// focus audit when orientation changes:
//
//   1. Check if the currently focused element has `offsetParent == null`.
//   2. If hidden, move focus to the first visible tabbable element in the scope.
//   3. Update the saved_focus reference if the restore target is now hidden.
//
// ```rust
// fn audit_focus_on_orientation_change(scope: &FocusScopeContext) {
//     let platform = use_platform_effects();
//     if let Some(active_id) = platform.active_element_id() {
//         if !platform.has_layout(&active_id) {
//             // Focused element is hidden — move to first visible tabbable
//             if let Some(ref container_id) = scope.container_id {
//                 platform.focus_first_tabbable(container_id);
//             }
//         }
//     }
//     // Also validate the saved_focus restore target
//     if let Some(ref saved) = scope.saved_focus {
//         if !platform.has_layout(saved) {
//             // Restore target is now hidden; clear it so fallback chain is used
//             scope.saved_focus = None;
//         }
//     }
// }
// ```
````

### 1.7 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "focus-scope"]
pub enum Part {
    Container,
}

/// The API for the `FocusScope` component.
pub struct Api<'a> {
    /// The current state of the focus scope.
    state: &'a State,
    /// The context of the focus scope.
    ctx: &'a Context,
    /// The props of the focus scope.
    props: &'a Props,
    /// The send function for the focus scope.
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Whether the focus scope is active.
    pub fn is_active(&self) -> bool {
        matches!(self.state, State::Active { .. })
    }

    /// Whether the focus scope is trapped.
    pub fn is_trapped(&self) -> bool {
        matches!(self.state, State::Active { trapped: true })
    }

    /// Props for the container element that scopes focus.
    pub fn container_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Container.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if self.is_active() {
            attrs.set_bool(HtmlAttr::Data("ars-active"), true);

            // tabindex="-1" allows the container itself to be focused programmatically,
            // which is needed as a focus target when no tabbable children exist yet.
            // Only set when active — an inactive scope's container should not appear
            // as a programmatic focus target to screen readers.
            attrs.set(HtmlAttr::TabIndex, "-1");
        }
        if self.is_trapped() {
            attrs.set_bool(HtmlAttr::Data("ars-trapped"), true);
        }
        // Event handlers (keydown for Tab trapping) are typed methods on the Api struct.
        attrs
    }

    /// Imperatively activate the focus scope.
    /// `saved_focus_id` is the ID of the currently focused element, captured by
    /// the adapter via `platform.active_element_id()` before calling this.
    pub fn activate(&self, trapped: bool, saved_focus_id: Option<String>) {
        (self.send)(Event::Activate { trapped, saved_focus_id });
    }

    /// Imperatively deactivate the focus scope.
    pub fn deactivate(&self, restore_focus: bool) {
        (self.send)(Event::Deactivate { restore_focus });
    }

    /// Move focus to the first tabbable element within the container.
    /// Used by framework adapters when auto_focus=true.
    pub fn focus_first(&self) {
        let platform = use_platform_effects();
        if let Some(ref id) = self.ctx.container_id {
            platform.focus_first_tabbable(id);
        }
    }

    /// Move focus to the last tabbable element within the container.
    pub fn focus_last(&self) {
        let platform = use_platform_effects();
        if let Some(ref id) = self.ctx.container_id {
            platform.focus_last_tabbable(id);
        }
    }

    /// Return IDs of all currently tabbable elements in the container.
    pub fn get_tabbable_elements(&self) -> Vec<String> {
        let platform = use_platform_effects();
        self.ctx.container_id
            .as_ref()
            .map(|id| platform.tabbable_element_ids(id))
            .unwrap_or_default()
    }

    /// Move focus to the next focusable element within the scope.
    /// Returns `true` if focus was moved, `false` if no valid target exists.
    /// Used by Toolbar, ActionGroup, and TreeView for programmatic sequential
    /// focus movement beyond roving tabindex.
    pub fn focus_next(&self, _opts: FocusNavigationOptions) -> bool {
        // IMPL: traverse tabbable elements forward within scope
        false
    }

    /// Move focus to the previous focusable element within the scope.
    /// Returns `true` if focus was moved, `false` if no valid target exists.
    pub fn focus_previous(&self, _opts: FocusNavigationOptions) -> bool {
        // IMPL: traverse tabbable elements backward within scope
        false
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Container => self.container_attrs(),
        }
    }
}

/// Options for focus navigation.
#[derive(Clone)]
pub struct FocusNavigationOptions {
    /// Wrap around at boundaries.
    pub wrap: bool,
    /// Element ID to start from (default: currently focused element).
    pub from: Option<String>,
    /// Only consider tabbable elements (tabindex >= 0).
    pub tabbable: bool,
    /// Custom filter predicate that receives an element ID.
    /// Uses `Rc` so the options struct can be cloned.
    pub accept: Option<Rc<dyn Fn(&str) -> bool>>,
}
```

#### Focus Manager Context

The adapter SHOULD provide the FocusScope's navigation methods (`focus_next`, `focus_previous`, `focus_first`, `focus_last`) via framework context, allowing child components to programmatically manage focus without prop drilling. In Leptos: `provide_context(FocusManager { ... })`. In Dioxus: `use_context_provider(|| FocusManager { ... })`. This mirrors React Aria's `useFocusManager()` hook pattern.

## 2. Anatomy

```text
FocusScope
└── Container    <div> (or any element)    data-ars-scope="focus-scope"
                                           data-ars-part="container"
                                           data-ars-active (when active)
                                           data-ars-trapped (when trapped)
```

| Part      | Element          | Key Attributes                                                                                     |
| --------- | ---------------- | -------------------------------------------------------------------------------------------------- |
| Container | `<div>` (or any) | `data-ars-scope="focus-scope"`, `data-ars-part="container"`, `data-ars-active`, `data-ars-trapped` |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

- No ARIA role is set on the container — FocusScope is a behavioral utility, not a semantic landmark.
- `tabindex="-1"` is set on the container when active, allowing programmatic focus when no tabbable children exist.
- `data-ars-active` and `data-ars-trapped` are data attributes for styling hooks; they are not ARIA attributes.

### 3.2 Focus Management

- A focus trap is required for modal dialogs per ARIA 1.2 (APG Modal Dialog pattern).
- The scope must include a way to close it reachable by keyboard (typically Escape key handled
  by the parent component, not by `FocusScope` itself).
- `auto_focus` moves focus into the dialog on open, which is required for screen reader users
  to know the dialog has appeared.
- `restore_focus` returns focus to the trigger on close, maintaining orientation in the page.

## 4. Internationalization

Label text is consumer-provided. `data-ars-*` attribute values are stable API tokens, not localized. RTL: no special handling needed — focus order follows DOM order regardless of text direction.

## 5. Tabbable Element Detection

The tabbable element query matches (in DOM order):

```css
a[href]:not([tabindex="-1"]):not([disabled]),
button:not([tabindex="-1"]):not([disabled]),
input:not([tabindex="-1"]):not([disabled]),
select:not([tabindex="-1"]):not([disabled]),
textarea:not([tabindex="-1"]):not([disabled]),
[contenteditable]:not([tabindex="-1"]),
[tabindex]:not([tabindex="-1"])
```

Elements with `visibility:hidden`, `display:none`, or inside a `<details>` (closed) are excluded.

## 6. Usage by ars-ui Components

| Component          | Props                                                  |
| ------------------ | ------------------------------------------------------ |
| `Dialog`           | `trapped=true, restore_focus=true, auto_focus=true`    |
| `AlertDialog`      | `trapped=true, restore_focus=true, auto_focus=true`    |
| `Drawer`           | `trapped=true, restore_focus=true, auto_focus=true`    |
| `Popover`          | `trapped=false, restore_focus=true, auto_focus=true`   |
| `Combobox` listbox | `trapped=false, restore_focus=false, auto_focus=false` |

### Composition with Dismissable

When a component uses both FocusScope (trapping focus) and Dismissable (providing DismissButton), the DismissButton elements MUST be rendered as children of the FocusScope container element. Placing DismissButton as a sibling of the FocusScope container makes it unreachable by Tab when focus is trapped.

**Correct:**

```html
<div data-ars-scope="dialog">
    <!-- FocusScope container -->
    <DismissButton />
    <!-- Inside trap — reachable -->
    <div data-ars-part="content">...</div>
    <DismissButton />
    <!-- Inside trap — reachable -->
</div>
```

**Incorrect:**

```html
<DismissButton />
<!-- Outside trap — unreachable! -->
<div data-ars-scope="dialog">
    <div data-ars-part="content">...</div>
</div>
```

## 7. Focus Restoration Fallbacks

1. If the original focus target has been removed from the DOM, focus moves to `document.body` and a console warning is logged.
2. If the focus target is in a different document (iframe), focus stays in the current document — cross-document focus restoration is not attempted.
3. If the document has no focusable elements, `document.body.focus()` is called as final fallback.
4. Focus restoration is always synchronous (in the same microtask as trap release).

## 8. Nested `FocusScope` Restoration

When multiple `FocusScope`s are nested (e.g., a dialog opens a confirmation popover), focus
restoration must follow a strict LIFO (last-in, first-out) order. Each scope pushes its
restore target onto a shared stack when activated and pops it on deactivation.

### 8.1 Focus Restoration Stack

The stack is shared via `thread_local!` with `RefCell` for web targets (single-threaded). Each document gets its own stack instance. In multi-document environments (iframes), the `ArsProvider` context determines which document's stack to use.

```rust
/// Global (per-document) stack tracking nested FocusScope restore targets.
/// Managed by the adapter's FocusScope effect layer, not by individual
/// state machines.
#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct FocusRestorationStack {
    /// The entries in the focus restoration stack.
    entries: Vec<FocusRestoreEntry>,
}

/// An entry in the focus restoration stack.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FocusRestoreEntry {
    /// The FocusScope instance that pushed this entry.
    pub scope_id: String,
    /// The element that held focus before this scope activated.
    pub saved_focus: Option<ElementId>,
    /// The element that triggered this scope's activation (e.g., the button
    /// that opened a dialog). Used as the primary restore target.
    pub trigger_element: Option<ElementId>,
}

impl FocusRestorationStack {
    /// Creates a new focus restoration stack.
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Called when a FocusScope activates. Records the current focus target.
    pub fn push(&mut self, scope_id: String, trigger: Option<ElementId>) {
        let saved = document().active_element_id();
        self.entries.push(FocusRestoreEntry {
            scope_id,
            saved_focus: saved,
            trigger_element: trigger,
        });
    }

    /// Called when a FocusScope deactivates. Returns the element to restore
    /// focus to, applying the fallback chain.
    pub fn pop(&mut self, scope_id: &str) -> Option<ElementId> {
        // Find and remove the entry for this scope.
        let idx = self.entries.iter().rposition(|e| e.scope_id == scope_id)?;
        let entry = self.entries.remove(idx);

        // Out-of-order deactivation check: if this scope is NOT the top of
        // the stack, its saved target may reside inside a now-inactive outer
        // scope. In that case, skip restoration entirely — the outer scope
        // will handle it when it deactivates.
        if idx < self.entries.len() {
            // Inner scope deactivating after an outer scope was already removed.
            // The saved target is likely invalid. Return None to let the caller
            // fall through to the fallback chain.
            return None;
        }

        // Fallback chain:
        // 1. Saved target, if it still exists in the DOM and is focusable.
        if let Some(ref el_id) = entry.saved_focus {
            if is_valid_restore_target(el_id) {
                return Some(el_id.clone());
            }
        }
        // 2. Trigger element, if valid.
        if let Some(ref trigger_id) = entry.trigger_element {
            if is_valid_restore_target(trigger_id) {
                return Some(trigger_id.clone());
            }
        }
        // 3. First tabbable element in the parent scope (caller handles).
        // 4. document.body (caller handles as final fallback).
        None
    }
}

/// Validates that an element still exists in the DOM and is focusable.
fn is_valid_restore_target(el_id: &ElementId) -> bool {
    let Some(el) = document().get_element_by_id(el_id) else {
        return false;
    };
    // Element must be connected to the DOM and not hidden/disabled.
    el.is_connected() && is_focusable(&el)
}
```

**SSR safety:** The `FocusRestorationStack` is a client-only construct. Adapters MUST gate its initialization:

- **Leptos:** Wrap in `#[cfg(not(feature = "ssr"))]` or guard with `leptos::is_server()` check
- **Dioxus:** Initialize inside `use_effect` (which only runs on the client)

If the stack is accidentally created on the server (e.g., via a global `thread_local!`), it could retain stale state across SSR requests in multi-tenant server environments.

**Nested restoration priority:** When both an inner and outer scope deactivate (e.g., a
confirmation popover closes followed by its parent dialog), the inner scope restores first,
then the outer scope restores — each popping from the stack in LIFO order.

**`saved_focus` validation:** Before restoring focus to a saved element, the adapter MUST
verify that the element (a) still exists in the DOM via `is_connected()` and (b) is
focusable (not `disabled`, not `display:none`, has valid `tabindex`). If validation fails,
the fallback chain proceeds.

**Out-of-order deactivation:** If an inner scope deactivates after its outer scope has
already been removed (e.g., both close simultaneously but cleanup runs in arbitrary order),
the inner scope's saved target may point to an element inside the now-destroyed outer scope.
In this case, the stack skips the inner scope's restoration and defers to the outer scope's
fallback chain.

**Fallback chain** (in priority order):

1. The saved focus target, if still valid in the DOM and focusable
2. The scope's trigger element (the element that caused the scope to activate)
3. The first tabbable element in the parent scope
4. `document.body` as the final fallback

## 9. Platform Notes

> **Dioxus focus operations:** Focus operations (`focus_first`, `focus_last`,
> `restore_focus_safely`) use `PlatformEffects` trait methods (see `01-architecture.md`
> section 2.2.7). For Dioxus Desktop/Mobile, the adapter provides a platform implementation
> that routes these through native focus APIs for cross-platform compatibility.
>
> **Cleanup timing:** Leptos uses `on_cleanup` and Dioxus uses `use_drop` for teardown.
> In HMR/hot-reload scenarios, timing may differ — ensure the `FocusRestorationStack`
> is cleared on both cleanup and re-mount.

## 10. Library Parity

> Compared against: React Aria (`FocusScope`).

### 10.1 Props

| Feature       | ars-ui                | React Aria     | Notes                                    |
| ------------- | --------------------- | -------------- | ---------------------------------------- |
| Auto-focus    | `auto_focus`          | `autoFocus`    | Both libraries                           |
| Contain/trap  | `trapped` / `contain` | `contain`      | Both libraries; ars-ui offers both names |
| Restore focus | `restore_focus`       | `restoreFocus` | Both libraries                           |

**Gaps:** None.

### 10.2 Anatomy

| Part      | ars-ui      | React Aria    | Notes                                  |
| --------- | ----------- | ------------- | -------------------------------------- |
| Container | `Container` | (wrapper div) | Both libraries use a container element |

**Gaps:** None.

### 10.3 Features

| Feature                  | ars-ui                              | React Aria              |
| ------------------------ | ----------------------------------- | ----------------------- |
| Focus trapping           | Yes                                 | Yes                     |
| Focus restoration        | Yes                                 | Yes                     |
| Auto-focus first element | Yes                                 | Yes                     |
| FocusManager context     | Yes (`focus_next`/`focus_previous`) | Yes (`useFocusManager`) |
| Nested scope stack       | Yes (`FocusRestorationStack`)       | Yes (internal)          |
| Focus navigation options | Yes (`FocusNavigationOptions`)      | Yes (`wrap`, etc.)      |

**Gaps:** None.

### 10.4 Summary

- **Overall:** Full parity.
- **Divergences:** ars-ui exposes both `trapped` and `contain` prop aliases. ars-ui explicitly defines `FocusRestorationStack` for nested scopes; React Aria handles this internally.
- **Recommended additions:** None.
