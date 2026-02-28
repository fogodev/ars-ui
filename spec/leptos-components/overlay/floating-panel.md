---
adapter: leptos
component: floating-panel
category: overlay
source: components/overlay/floating-panel.md
source_foundation: foundation/08-adapter-leptos.md
---

# FloatingPanel -- Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`FloatingPanel`](../../components/overlay/floating-panel.md) contract onto Leptos 0.8.x compound components. The adapter owns pointer-event wiring for drag and resize interactions, portal rendering, z-index allocation via `ZIndexAllocator`, focus management for modal mode, keyboard navigation for panel movement, and the compound component context that distributes machine state and send handles across sub-components. The core machine owns all state transitions, position/size clamping, grid snapping, aspect-ratio locking, and stage management (minimize/maximize/restore).

## 2. Public Adapter API

```rust
#[component]
pub fn FloatingPanel(
    #[prop(optional)] id: Option<String>,
    #[prop(optional)] open: Option<Signal<bool>>,
    #[prop(optional)] default_open: Option<bool>,
    #[prop(optional)] initial_position: Option<(f64, f64)>,
    #[prop(optional)] initial_size: Option<(f64, f64)>,
    #[prop(optional)] min_size: Option<(f64, f64)>,
    #[prop(optional)] max_size: Option<(f64, f64)>,
    #[prop(optional)] resizable: Option<bool>,
    #[prop(optional)] draggable: Option<bool>,
    #[prop(optional)] closable: Option<bool>,
    #[prop(optional)] minimizable: Option<bool>,
    #[prop(optional)] maximizable: Option<bool>,
    #[prop(optional)] modal: Option<bool>,
    #[prop(optional)] constrain_to_viewport: Option<bool>,
    #[prop(optional)] close_on_escape: Option<bool>,
    #[prop(optional)] allow_overflow: Option<bool>,
    #[prop(optional)] lock_aspect_ratio: Option<bool>,
    #[prop(optional)] grid_size: Option<f64>,
    #[prop(optional)] persist_rect: Option<bool>,
    #[prop(optional)] lazy_mount: Option<bool>,
    #[prop(optional)] unmount_on_exit: Option<bool>,
    #[prop(optional)] on_open_change: Option<Callback<bool>>,
    #[prop(optional)] on_position_change: Option<Callback<(f64, f64)>>,
    #[prop(optional)] on_position_change_end: Option<Callback<(f64, f64)>>,
    #[prop(optional)] on_size_change: Option<Callback<(f64, f64)>>,
    #[prop(optional)] on_size_change_end: Option<Callback<(f64, f64)>>,
    #[prop(optional)] on_stage_change: Option<Callback<Stage>>,
    #[prop(optional)] messages: Option<floating_panel::Messages>,
    #[prop(optional)] locale: Option<Locale>,
    children: Children,
) -> impl IntoView

#[component]
pub fn Trigger(
    children: Children,
) -> impl IntoView

#[component]
pub fn Content(
    children: Children,
) -> impl IntoView

#[component]
pub fn DragHandle(
    children: Children,
) -> impl IntoView

#[component]
pub fn ResizeHandle(
    handle: ResizeHandle,
) -> impl IntoView

#[component]
pub fn Header(
    children: Children,
) -> impl IntoView

#[component]
pub fn Title(
    children: Children,
) -> impl IntoView

#[component]
pub fn Footer(
    children: Children,
) -> impl IntoView

#[component]
pub fn CloseTrigger(
    children: Children,
) -> impl IntoView

#[component]
pub fn MinimizeTrigger(
    children: Children,
) -> impl IntoView

#[component]
pub fn MaximizeTrigger(
    children: Children,
) -> impl IntoView

#[component]
pub fn StageTrigger(
    children: Children,
) -> impl IntoView
```

The adapter surfaces the full core prop set on `FloatingPanel`. Sub-components consume machine state and send handles through the compound component context.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core `Props`, including all drag/resize/stage configuration, mount control (`lazy_mount`, `unmount_on_exit`), boundary constraints, and i18n messages.
- Event parity: `DragStart`, `DragMove`, `DragEnd`, `ResizeStart`, `ResizeMove`, `ResizeEnd`, `Minimize`, `Maximize`, `Restore`, `Close`, `BringToFront`, `Focus`, `Blur`, `CloseOnEscape`, and `SetZIndex` are all adapter-driven.
- Part parity: all 11 core part types (Root, Header, DragHandle, Title, Content, Footer, ResizeHandle, CloseTrigger, MinimizeTrigger, MaximizeTrigger, StageTrigger) are mapped to compound sub-components.
- Core machine ownership: `use_machine::<floating_panel::Machine>(...)` in `FloatingPanel` remains the single source of truth for state, position, size, stage, and z-index.

## 4. Part Mapping

| Core part / structure | Required?   | Adapter rendering target                                               | Ownership     | Attr source                       | Notes                                                                  |
| --------------------- | ----------- | ---------------------------------------------------------------------- | ------------- | --------------------------------- | ---------------------------------------------------------------------- |
| `Root`                | required    | `<div>` with `role="dialog"`, `position:fixed`, inline position/size/z | adapter-owned | `api.root_attrs()`                | Rendered inside portal. Receives all state data attrs.                 |
| `Header`              | required    | `<div>` containing drag handle, title, and window controls             | adapter-owned | `api.header_attrs()`              | Serves as the title bar area.                                          |
| `DragHandle`          | required    | `<div>` inside Header with `cursor:grab` when draggable                | adapter-owned | `api.drag_handle_attrs()`         | Pointer events initiate drag. Disabled when maximized.                 |
| `Title`               | required    | `<h2>` or `<div>` with ID for `aria-labelledby`                        | adapter-owned | `api.title_attrs()`               | Consumer provides title content as children.                           |
| `Content`             | required    | `<div>` hidden when minimized                                          | adapter-owned | `api.content_attrs()`             | Consumer content lives here. Hidden via `hidden` attr when minimized.  |
| `Footer`              | conditional | `<div>` hidden when minimized                                          | adapter-owned | `api.footer_attrs()`              | Optional structural region for panel footer content.                   |
| `ResizeHandle`        | repeated    | `<div>` per handle (up to 8), with directional cursor and aria-label   | adapter-owned | `api.resize_handle_attrs(handle)` | Each instance parameterized by `ResizeHandle` enum variant.            |
| `CloseTrigger`        | conditional | `<button type="button">` with `aria-label`                             | adapter-owned | `api.close_trigger_attrs()`       | Rendered when `closable=true`. Sends `Event::Close`.                   |
| `MinimizeTrigger`     | conditional | `<button type="button">` with `aria-label` (Minimize / Restore)        | adapter-owned | `api.minimize_trigger_attrs()`    | Rendered when `minimizable=true`. Label toggles based on stage.        |
| `MaximizeTrigger`     | conditional | `<button type="button">` with `aria-label` (Maximize / Restore)        | adapter-owned | `api.maximize_trigger_attrs()`    | Rendered when `maximizable=true`. Label toggles based on stage.        |
| `StageTrigger`        | conditional | `<button type="button">` with `aria-label`, `data-ars-state`           | adapter-owned | `api.stage_trigger_attrs()`       | Cycles Normal -> Minimized -> Normal. Alternative to separate buttons. |

## 5. Attr Merge and Ownership Rules

| Target node       | Core attrs                                                                       | Adapter-owned attrs                                                                  | Consumer attrs                      | Merge order                                                          | Ownership notes                                                      |
| ----------------- | -------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ | ----------------------------------- | -------------------------------------------------------------------- | -------------------------------------------------------------------- |
| `Root`            | `api.root_attrs()` (role, ARIA, state data attrs, inline position/size/z styles) | pointer event handlers (`pointerdown` for BringToFront), `keydown` for Escape/arrows | consumer root decoration only       | core state/ARIA/position attrs win; `class`/`style` merge additively | adapter-owned; consumer must not override positioning styles         |
| `Header`          | `api.header_attrs()` (scope/part data attrs)                                     | structural wrapper attrs                                                             | consumer decoration                 | core data attrs remain; consumer classes decorate additively         | adapter-owned                                                        |
| `DragHandle`      | `api.drag_handle_attrs()` (aria-label, cursor)                                   | `pointerdown`/`pointermove`/`pointerup` handlers for drag                            | consumer decoration only            | core aria-label and cursor win; consumer classes decorate            | adapter-owned; consumer must not override cursor or pointer handlers |
| `Title`           | `api.title_attrs()` (id for aria-labelledby)                                     | structural wrapper attrs                                                             | consumer title content as children  | core id must remain stable                                           | adapter-owned; consumer provides content only                        |
| `Content`         | `api.content_attrs()` (hidden when minimized)                                    | structural wrapper attrs                                                             | consumer main content as children   | core hidden attr wins                                                | adapter-owned                                                        |
| `Footer`          | `api.footer_attrs()` (hidden when minimized)                                     | structural wrapper attrs                                                             | consumer footer content as children | core hidden attr wins                                                | adapter-owned                                                        |
| `ResizeHandle`    | `api.resize_handle_attrs(handle)` (aria-label, data-ars-handle, cursor)          | `pointerdown`/`pointermove`/`pointerup` handlers for resize                          | no consumer attrs                   | core aria-label and cursor win                                       | adapter-owned; not exposed for consumer decoration                   |
| `CloseTrigger`    | `api.close_trigger_attrs()` (type, aria-label)                                   | `click` handler sending `Event::Close`                                               | consumer button content as children | core button semantics win                                            | adapter-owned                                                        |
| `MinimizeTrigger` | `api.minimize_trigger_attrs()` (type, aria-label)                                | `click` handler sending `Event::Minimize` or `Event::Restore`                        | consumer button content as children | core button semantics and aria-label win                             | adapter-owned                                                        |
| `MaximizeTrigger` | `api.maximize_trigger_attrs()` (type, aria-label)                                | `click` handler sending `Event::Maximize` or `Event::Restore`                        | consumer button content as children | core button semantics and aria-label win                             | adapter-owned                                                        |
| `StageTrigger`    | `api.stage_trigger_attrs()` (type, aria-label, data-ars-state)                   | `click` handler calling `api.on_stage_trigger_click()`                               | consumer button content as children | core button semantics and stage label win                            | adapter-owned                                                        |

- Consumer must not override `position`, `left`, `top`, `width`, `height`, or `z-index` styles on Root; these are machine-owned.
- Drag and resize pointer handlers must not be overridden by consumer event handlers.
- `class` and `style` (non-positioning) merge additively on all parts.

## 6. Composition / Context Contract

`FloatingPanel` provides a compound component context consumed by all sub-components:

```rust
#[derive(Clone, Copy)]
struct Context {
    machine: MachineHandle<floating_panel::Machine>,
}
```

Sub-components retrieve this via `use_context::<Context>().expect("...")`. The context carries the machine handle from which sub-components derive attrs and send events.

External composition:

- When `modal=true`, the adapter composes with `FocusScope` to trap focus within the panel and sets `inert` on background content.
- Portal rendering uses the shared portal infrastructure to render Root outside the DOM tree.
- Z-index allocation uses `ZIndexAllocator` via `resolve_z_allocator()` for `BringToFront`.
- Drag interaction composes conceptually with the `use_move` interaction pattern from `05-interactions.md`.

## 7. Prop Sync and Event Mapping

The `open` prop supports controlled mode. When `open` is `Some`, the consumer drives open/close state; the adapter syncs changes via an effect. All other props are non-reactive after initialization unless explicitly noted.

| Adapter prop  | Mode                      | Sync trigger              | Machine event / update path               | Visible effect                                         | Notes                                                  |
| ------------- | ------------------------- | ------------------------- | ----------------------------------------- | ------------------------------------------------------ | ------------------------------------------------------ |
| `open`        | controlled                | signal change after mount | controlled open-state sync                | opens or closes the panel                              | when `None`, uses `default_open` for uncontrolled mode |
| `resizable`   | non-reactive adapter prop | render time only          | included in Props passed to machine       | enables/disables resize handles and resize transitions | post-mount changes require machine reinitialization    |
| `draggable`   | non-reactive adapter prop | render time only          | included in Props passed to machine       | enables/disables drag on title bar / drag handle       | post-mount changes require machine reinitialization    |
| `closable`    | non-reactive adapter prop | render time only          | included in Props passed to machine       | shows/hides CloseTrigger                               | conditional rendering, not runtime toggle              |
| `minimizable` | non-reactive adapter prop | render time only          | included in Props passed to machine       | shows/hides MinimizeTrigger                            | conditional rendering, not runtime toggle              |
| `maximizable` | non-reactive adapter prop | render time only          | included in Props passed to machine       | shows/hides MaximizeTrigger                            | conditional rendering, not runtime toggle              |
| `modal`       | non-reactive adapter prop | render time only          | adapter-level composition with FocusScope | traps focus, sets inert on background                  | client-only behavior                                   |

| UI event                      | Preconditions                            | Machine event / callback path                                   | Ordering notes                                                | Notes                                                    |
| ----------------------------- | ---------------------------------------- | --------------------------------------------------------------- | ------------------------------------------------------------- | -------------------------------------------------------- |
| `pointerdown` on DragHandle   | `draggable=true` and not maximized       | `DragStart`; begin pointermove/pointerup tracking on document   | sets pointer capture before first DragMove                    | document-level listeners for move/end                    |
| `pointermove` during drag     | DragStart has fired, pointer captured    | `DragMove(dx, dy)` with delta from last position                | fires on each animation frame or pointer event                | `on_position_change` callback fires after context update |
| `pointerup` ending drag       | DragStart has fired                      | `DragEnd`; release pointer capture, remove document listeners   | `on_position_change_end` fires after final position committed | cleanup must run even if pointer leaves viewport         |
| `pointerdown` on ResizeHandle | `resizable=true` and not maximized       | `ResizeStart(handle)`; begin pointermove/pointerup tracking     | sets pointer capture before first ResizeMove                  | document-level listeners for move/end                    |
| `pointermove` during resize   | ResizeStart has fired, pointer captured  | `ResizeMove(dx, dy)` with delta from last position              | fires on each animation frame or pointer event                | `on_size_change` callback fires after context update     |
| `pointerup` ending resize     | ResizeStart has fired                    | `ResizeEnd`; release pointer capture, remove document listeners | `on_size_change_end` fires after final size committed         | cleanup must run even if pointer leaves viewport         |
| `click` on CloseTrigger       | `closable=true`                          | `Close`; `on_open_change(false)` callback                       | callback fires after state transition                         | standard button activation                               |
| `click` on MinimizeTrigger    | `minimizable=true`                       | `Minimize` or `Restore`; `on_stage_change` callback             | callback fires after state transition                         | toggles based on current minimized state                 |
| `click` on MaximizeTrigger    | `maximizable=true`                       | `Maximize` or `Restore`; `on_stage_change` callback             | callback fires after state transition                         | toggles based on current maximized state                 |
| `click` on StageTrigger       | rendered                                 | `api.on_stage_trigger_click()` cycling stages                   | callback fires after state transition                         | cycles Normal -> Minimized -> Normal                     |
| `keydown` Escape on Root      | `close_on_escape=true` and panel focused | `CloseOnEscape`; `on_open_change(false)` callback               | fires before blur                                             | client-only                                              |
| `keydown` Arrow on Root       | panel root focused                       | `DragStart` (keyboard-based movement nudge)                     | fires after focus verification                                | keyboard movement for accessibility                      |
| `pointerdown` on Root         | panel rendered                           | `BringToFront`; triggers z-index allocation effect              | z-index update fires asynchronously via `SetZIndex`           | every interaction brings panel to front                  |
| `focus` on Root               | panel rendered                           | `Focus { is_keyboard }`                                         | after pointer modality normalization                          | tracks focus-visible for styling                         |
| `blur` on Root                | panel had focus                          | `Blur`                                                          | before any late cleanup                                       | clears focus and focus-visible state                     |
| `dblclick` on Header          | `maximizable=true`                       | `Maximize` or `Restore` (toggle)                                | same as MaximizeTrigger click                                 | double-click title bar toggles maximize                  |

## 8. Registration and Cleanup Contract

| Registered entity               | Registration trigger              | Identity key       | Cleanup trigger                            | Cleanup action                                    | Notes                                            |
| ------------------------------- | --------------------------------- | ------------------ | ------------------------------------------ | ------------------------------------------------- | ------------------------------------------------ |
| document pointermove listener   | `DragStart` or `ResizeStart`      | component instance | `DragEnd`/`ResizeEnd` or component cleanup | remove document listener, release pointer capture | must not leak if panel closes during drag/resize |
| document pointerup listener     | `DragStart` or `ResizeStart`      | component instance | `DragEnd`/`ResizeEnd` or component cleanup | remove document listener                          | must fire cleanup even if pointer exits viewport |
| z-index allocation              | `BringToFront` event              | component instance | component cleanup                          | release allocated z-index slot                    | uses `ZIndexAllocator` shared context            |
| portal mount                    | panel opens (first or subsequent) | component instance | panel closes or component cleanup          | remove portal node from DOM                       | respects `lazy_mount` and `unmount_on_exit`      |
| FocusScope (modal mode)         | `modal=true` and panel opens      | component instance | panel closes or component cleanup          | release focus trap, remove inert from background  | client-only; not registered during SSR           |
| compound context                | `FloatingPanel` mount             | component instance | component cleanup                          | context goes out of scope                         | sub-components lose access on cleanup            |
| keyboard event listener on Root | panel opens and Root mounts       | component instance | panel closes or component cleanup          | remove keydown handler                            | handles Escape and arrow keys                    |

## 9. Ref and Node Contract

| Target part / node  | Ref required? | Ref owner     | Node availability                 | Composition rule                                                | Notes                                                                |
| ------------------- | ------------- | ------------- | --------------------------------- | --------------------------------------------------------------- | -------------------------------------------------------------------- |
| Root                | yes           | adapter-owned | required after mount              | compose with consumer ref if a wrapper needs the live root node | pointer event delegation and focus management require a concrete ref |
| DragHandle          | yes           | adapter-owned | required after mount              | no composition; adapter-only                                    | pointer capture requires a concrete node handle                      |
| ResizeHandle (each) | yes           | adapter-owned | required after mount              | no composition; adapter-only                                    | pointer capture requires a concrete node handle per instance         |
| Title               | no            | adapter-owned | always structural handle optional | no composition                                                  | ID-based reference for `aria-labelledby` is sufficient               |
| Content             | no            | adapter-owned | always structural handle optional | no composition                                                  | structural only                                                      |
| Header              | no            | adapter-owned | always structural handle optional | no composition                                                  | structural only                                                      |
| Footer              | no            | adapter-owned | always structural handle optional | no composition                                                  | structural only                                                      |
| CloseTrigger        | no            | adapter-owned | always structural handle optional | no composition                                                  | standard button, no ref needed                                       |
| MinimizeTrigger     | no            | adapter-owned | always structural handle optional | no composition                                                  | standard button, no ref needed                                       |
| MaximizeTrigger     | no            | adapter-owned | always structural handle optional | no composition                                                  | standard button, no ref needed                                       |
| StageTrigger        | no            | adapter-owned | always structural handle optional | no composition                                                  | standard button, no ref needed                                       |
| Portal container    | yes           | adapter-owned | client-only                       | compose with ars-provider portal target if available            | server-safe absent; created on client mount                          |

## 10. State Machine Boundary Rules

- machine-owned state: position, size, z-index, open, minimized, maximized, focused, focus-visible, pre-maximize snapshot, active resize handle, stage transitions, grid snapping, aspect-ratio enforcement, viewport clamping.
- adapter-local derived bookkeeping: pointer capture state, last pointer position for delta calculation during drag/resize, pointer-versus-keyboard modality tracking, document-level listener handles, portal mount state, `lazy_mount` first-opened tracking.
- forbidden local mirrors: do not keep local signals for position, size, open, minimized, maximized, or z-index that can diverge from machine context. All reads must come from machine derivations.
- allowed snapshot-read contexts: pointer event handlers (to compute deltas), render derivation (to read position/size for inline styles), cleanup (to release pointer capture), and callback invocation (to read final position/size for `on_position_change_end`/`on_size_change_end`).

## 11. Callback Payload Contract

| Callback                 | Payload source           | Payload shape | Timing                                                    | Cancelable? | Notes                                                        |
| ------------------------ | ------------------------ | ------------- | --------------------------------------------------------- | ----------- | ------------------------------------------------------------ |
| `on_open_change`         | machine-derived snapshot | `bool`        | after Close or CloseOnEscape transition completes         | no          | observational; consumer cannot veto close from this callback |
| `on_position_change`     | machine-derived snapshot | `(f64, f64)`  | after each DragMove context update                        | no          | fires on every movement frame during drag                    |
| `on_position_change_end` | machine-derived snapshot | `(f64, f64)`  | after DragEnd transition completes                        | no          | fires once with the final committed position                 |
| `on_size_change`         | machine-derived snapshot | `(f64, f64)`  | after each ResizeMove context update                      | no          | fires on every resize frame                                  |
| `on_size_change_end`     | machine-derived snapshot | `(f64, f64)`  | after ResizeEnd transition completes                      | no          | fires once with the final committed size                     |
| `on_stage_change`        | machine-derived snapshot | `Stage`       | after Minimize, Maximize, or Restore transition completes | no          | `Stage::Default`, `Stage::Minimized`, or `Stage::Maximized`  |

## 12. Failure and Degradation Rules

| Condition                                          | Policy             | Notes                                                                              |
| -------------------------------------------------- | ------------------ | ---------------------------------------------------------------------------------- |
| Root ref missing after mount                       | fail fast          | Pointer event delegation and focus management cannot function without a root node. |
| DragHandle or ResizeHandle ref missing after mount | degrade gracefully | Panel renders but drag/resize interactions are disabled for the missing handle.    |
| ZIndexAllocator context missing                    | warn and ignore    | Panel uses a static z-index; BringToFront becomes a no-op.                         |
| Portal target unavailable                          | degrade gracefully | Fall back to inline rendering without portal; log a debug warning.                 |
| FocusScope unavailable for modal mode              | degrade gracefully | Panel opens without focus trapping; log a debug warning.                           |
| Browser pointer APIs absent during SSR             | no-op              | Render structure only; all pointer interactions are client-only.                   |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source  | Duplicates allowed?                       | DOM order must match registration order? | SSR/hydration stability                                  | Notes                                              |
| -------------------------------- | ---------------- | ----------------------------------------- | ---------------------------------------- | -------------------------------------------------------- | -------------------------------------------------- |
| panel instance                   | instance-derived | not applicable                            | not applicable                           | root ID and title ID must remain stable across hydration | machine identity is tied to the component instance |
| resize handle instances          | data-derived     | yes (8 distinct handles per ResizeHandle) | no                                       | stable across hydration                                  | identity is the `ResizeHandle` enum variant        |
| portal mount                     | instance-derived | not applicable                            | not applicable                           | server-safe absent; client-only mount                    | portal identity follows the panel instance         |

## 14. SSR and Client Boundary Rules

- SSR renders the panel structure (Root, Header, Title, Content, Footer, trigger buttons) based on `default_open` or initial controlled `open` value. All inline position/size/z-index styles are included in the server-rendered output.
- Resize handles render as structural divs during SSR but pointer interactions are client-only.
- Drag, resize, z-index allocation, focus trapping, and Escape/arrow keyboard handlers are all client-only behaviors.
- Portal rendering is client-only; during SSR the panel structure renders inline at its declaration site.
- The Root ref is server-safe absent and becomes required after mount.
- `lazy_mount`: when true, panel content is not rendered on the server if the panel starts closed.
- `unmount_on_exit`: when true, panel content DOM is removed after closing on the client.
- No callbacks (`on_open_change`, `on_position_change`, etc.) fire during SSR.

## 15. Performance Constraints

- Root attrs (including inline position/size/z-index styles) must be derived via memoization, not rebuilt from scratch on every render.
- Document-level pointer listeners for drag/resize must only be attached during active drag/resize operations and removed immediately on end.
- Resize handle attrs should be memoized per-handle and only recomputed when relevant state changes (resizable flag, maximized state, or handle cursor).
- Position and size callback invocations during drag/resize should be throttled to animation frame boundaries to avoid excessive callback pressure.
- Z-index allocation should be a single allocation per BringToFront event, not per-render.
- Portal mount/unmount must follow `lazy_mount` and `unmount_on_exit` semantics to avoid unnecessary DOM operations.
- Compound context should use `Copy` semantics (wrapping a `MachineHandle`) to avoid allocation on sub-component access.

## 16. Implementation Dependencies

| Dependency          | Required?   | Dependency type         | Why it must exist first                                                            | Notes                                                          |
| ------------------- | ----------- | ----------------------- | ---------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| `z-index-allocator` | required    | context contract        | BringToFront behavior requires z-index allocation from the shared allocator.       | Without it, z-index management falls back to a static value.   |
| `ars-provider`      | recommended | context contract        | Portal target resolution and environment scoping simplify DOM boundary management. | Especially relevant for portal rendering.                      |
| `focus-scope`       | required    | behavioral prerequisite | Modal mode requires focus trapping within the panel.                               | Only required when `modal=true`; otherwise a no-op dependency. |
| `presence`          | recommended | composition contract    | Animation support for open/close transitions.                                      | Composes around the panel Root for enter/exit animations.      |
| `dismissable`       | recommended | behavioral prerequisite | Modal panels may compose with Dismissable for outside-click behavior.              | Only relevant when `modal=true`.                               |

## 17. Recommended Implementation Sequence

1. Initialize the machine with core Props built from adapter props and establish the compound context via `provide_context`.
2. Render the Root container with portal rendering and inline position/size/z-index styles from machine-derived attrs.
3. Render Header with DragHandle, Title, and window control triggers (Close, Minimize, Maximize, StageTrigger) using machine-derived attrs.
4. Render Content and Footer regions with minimized-hidden behavior.
5. Render up to 8 ResizeHandle instances (when `resizable=true`) with per-handle directional cursors and aria-labels.
6. Wire drag interaction: `pointerdown` on DragHandle sets pointer capture, attaches document-level `pointermove`/`pointerup` listeners, sends `DragStart`; `pointermove` computes deltas and sends `DragMove`; `pointerup` sends `DragEnd` and cleans up.
7. Wire resize interaction: same pattern as drag but on ResizeHandle nodes with `ResizeStart(handle)`/`ResizeMove`/`ResizeEnd`.
8. Wire BringToFront on any `pointerdown` on Root.
9. Wire keyboard handlers: Escape for close, arrow keys for position nudge.
10. Wire focus/blur handlers on Root with pointer-modality tracking.
11. Add controlled `open` sync effect and callback invocation for all callbacks.
12. Add modal mode composition (FocusScope, inert background) when `modal=true`.
13. Add `lazy_mount` / `unmount_on_exit` conditional rendering logic.
14. Verify cleanup: document listeners, pointer capture, portal, focus trap, z-index slot.

## 18. Anti-Patterns

- Do not store position or size in local signals separate from the machine context.
- Do not attach document-level pointermove/pointerup listeners persistently; they must only exist during active drag or resize.
- Do not apply `cursor:grab` or `cursor:resize` styles when the panel is maximized.
- Do not allow drag to start when the panel is maximized.
- Do not allow resize to start when the panel is maximized.
- Do not fire `on_position_change_end` or `on_size_change_end` during an active drag or resize; these fire only on `DragEnd`/`ResizeEnd`.
- Do not use HTML `disabled` on window control buttons; use `aria-disabled` for buttons that are contextually inactive.
- Do not register pointer event listeners during SSR.
- Do not omit cleanup of document-level listeners if the panel closes or unmounts during an active drag/resize.
- Do not compute deltas from absolute pointer coordinates; use the delta from the previous pointer position to avoid drift with viewport clamping.

## 19. Consumer Expectations and Guarantees

- Consumers may assume that all position/size/z-index updates are driven by the machine and reflected in inline styles on Root.
- Consumers may assume that `on_position_change` and `on_size_change` fire on every movement frame during drag/resize.
- Consumers may assume that `on_position_change_end` and `on_size_change_end` fire exactly once after drag/resize completes.
- Consumers may assume that `on_stage_change` fires after each minimize/maximize/restore transition.
- Consumers may assume that sub-components (Trigger, Content, Header, etc.) can be used inside `FloatingPanel` in any order.
- Consumers may assume resize handles have at least 44x44px touch targets per WCAG 2.5.5.
- Consumers must not assume the panel renders at the declaration site; it may be portaled.
- Consumers must not assume they can override inline position/size/z-index styles on Root.
- Consumers must not assume drag or resize is available when the panel is maximized.

## 20. Platform Support Matrix

| Capability / behavior               | Browser client | SSR          | Notes                                                                           |
| ----------------------------------- | -------------- | ------------ | ------------------------------------------------------------------------------- |
| structural rendering (all parts)    | full support   | full support | SSR renders initial structure with inline styles; interactions are client-only. |
| drag interaction (pointer events)   | full support   | no-op        | document-level pointer listeners are client-only                                |
| resize interaction (pointer events) | full support   | no-op        | document-level pointer listeners are client-only                                |
| keyboard move/close                 | full support   | no-op        | keydown handlers attach after mount                                             |
| z-index allocation (BringToFront)   | full support   | no-op        | ZIndexAllocator is a client-only runtime concern                                |
| portal rendering                    | full support   | no-op        | SSR renders inline; portal mount happens on client                              |
| modal focus trapping                | full support   | no-op        | FocusScope composes on client only                                              |
| callbacks (all)                     | full support   | no-op        | no callbacks fire during SSR                                                    |

## 21. Debug Diagnostics and Production Policy

| Condition                                            | Debug build behavior | Production behavior | Notes                                                    |
| ---------------------------------------------------- | -------------------- | ------------------- | -------------------------------------------------------- |
| Root ref missing after mount                         | fail fast            | fail fast           | interactions cannot function without a root node         |
| ZIndexAllocator context not found                    | debug warning        | warn and ignore     | falls back to static z-index; BringToFront is a no-op    |
| portal target unavailable                            | debug warning        | degrade gracefully  | falls back to inline rendering                           |
| FocusScope unavailable for modal                     | debug warning        | degrade gracefully  | panel opens without focus trapping                       |
| DragHandle ref missing after mount                   | debug warning        | degrade gracefully  | drag interaction is disabled                             |
| document listener leaked after component cleanup     | debug warning        | no-op               | indicates cleanup logic bug; production silently ignores |
| `open` controlled prop changes between Some and None | debug warning        | warn and ignore     | controlled/uncontrolled switching is not supported       |

## 22. Shared Adapter Helper Notes

| Helper concept                | Required?   | Responsibility                                                                   | Reused by                                        | Notes                                                                 |
| ----------------------------- | ----------- | -------------------------------------------------------------------------------- | ------------------------------------------------ | --------------------------------------------------------------------- |
| pointer-capture drag helper   | required    | Manage pointer capture, document listener attachment, delta computation for drag | `floating-panel`, `splitter`, `slider`           | Encapsulates pointerdown/move/up lifecycle with cleanup               |
| pointer-capture resize helper | required    | Same as drag helper but parameterized by ResizeHandle direction                  | `floating-panel`                                 | May share implementation with drag helper using a direction parameter |
| portal rendering helper       | recommended | Mount/unmount portal containers and manage lifecycle                             | `floating-panel`, `dialog`, `popover`, `tooltip` | Shared across overlay components                                      |
| z-index allocation helper     | required    | Interface with `ZIndexAllocator` context for allocation/release                  | `floating-panel`, `dialog`, `popover`, `tooltip` | Wraps `resolve_z_allocator().allocate()` with cleanup                 |
| platform capability helper    | recommended | Normalize pointer event API assumptions for the active browser runtime           | `floating-panel`, `dismissable`, `drop-zone`     | Should surface capability caveats for pointer capture support         |

## 23. Framework-Specific Behavior

Leptos 0.8.x allows the adapter to keep machine-derived attrs in memos and spread them into rendered nodes via `{..attrs.get()}`. Document-level listeners for drag/resize should be attached using `window().add_event_listener_with_callback()` from `web_sys` during client-only effects and cleaned up via `on_cleanup`. Pointer capture uses `set_pointer_capture()`/`release_pointer_capture()` on the concrete DOM node obtained from `NodeRef`. The compound context is provided via `provide_context` and consumed via `use_context`. Portal rendering should use a `document().body()` append during a client-only effect. `on_cleanup` handles all teardown including document listeners, pointer capture release, portal removal, and focus trap release.

## 24. Canonical Implementation Sketch

```rust
use leptos::prelude::*;

#[component]
pub fn FloatingPanel(
    #[prop(optional)] id: Option<String>,
    #[prop(optional)] initial_position: Option<(f64, f64)>,
    #[prop(optional)] initial_size: Option<(f64, f64)>,
    #[prop(optional)] resizable: Option<bool>,
    #[prop(optional)] draggable: Option<bool>,
    #[prop(optional)] on_position_change: Option<Callback<(f64, f64)>>,
    #[prop(optional)] on_size_change: Option<Callback<(f64, f64)>>,
    #[prop(optional)] on_open_change: Option<Callback<bool>>,
    children: Children,
) -> impl IntoView {
    let props = floating_panel::Props {
        id: id.unwrap_or_default(),
        initial_position: initial_position.unwrap_or((100.0, 100.0)),
        initial_size: initial_size.unwrap_or((400.0, 300.0)),
        resizable: resizable.unwrap_or(true),
        draggable: draggable.unwrap_or(true),
        ..Default::default()
    };

    let machine = use_machine::<floating_panel::Machine>(props);
    provide_context(Context { machine });

    let root_ref = NodeRef::<html::Div>::new();
    let root_attrs = machine.derive(|api| api.root_attrs());
    let is_open = machine.derive(|api| api.is_open());

    let last_pointer = StoredValue::new(false);

    view! {
        <Show when=move || is_open.get()>
            <div
                node_ref=root_ref
                {..root_attrs.get()}
                on:pointerdown=move |_| {
                    last_pointer.set_value(true);
                    machine.send.run(floating_panel::Event::BringToFront);
                }
                on:focus=move |_| {
                    let is_keyboard = !last_pointer.get_value();
                    last_pointer.set_value(false);
                    machine.send.run(floating_panel::Event::Focus { is_keyboard });
                }
                on:blur=move |_| machine.send.run(floating_panel::Event::Blur)
                on:keydown=move |ev| {
                    machine.with_api_snapshot(|api| api.on_keydown(
                        &KeyboardEventData::from_event(&ev),
                    ));
                }
            >
                {children()}
            </div>
        </Show>
    }
}

#[component]
pub fn DragHandle(children: Children) -> impl IntoView {
    let ctx = use_context::<Context>()
        .expect("floating_panel::DragHandle must be used within FloatingPanel");
    let handle_ref = NodeRef::<html::Div>::new();
    let attrs = ctx.machine.derive(|api| api.drag_handle_attrs());

    // Client-only: attach pointer-capture drag lifecycle
    Effect::new(move |_| {
        if let Some(node) = handle_ref.get() {
            // Wire pointerdown -> set capture -> document pointermove/pointerup
            // Send DragStart, DragMove(dx, dy), DragEnd to ctx.machine.send
            let _ = node; // placeholder for pointer capture wiring
        }
    });

    view! {
        <div node_ref=handle_ref {..attrs.get()}>
            {children()}
        </div>
    }
}

#[component]
pub fn ResizeHandle(handle: ResizeHandle) -> impl IntoView {
    let ctx = use_context::<Context>()
        .expect("floating_panel::ResizeHandle must be used within FloatingPanel");
    let handle_ref = NodeRef::<html::Div>::new();
    let attrs = ctx.machine.derive(move |api| api.resize_handle_attrs(handle));

    // Client-only: attach pointer-capture resize lifecycle
    // Send ResizeStart(handle), ResizeMove(dx, dy), ResizeEnd to ctx.machine.send

    view! {
        <div node_ref=handle_ref {..attrs.get()} />
    }
}

#[component]
pub fn Header(children: Children) -> impl IntoView {
    let ctx = use_context::<Context>()
        .expect("floating_panel::Header must be used within FloatingPanel");
    let attrs = ctx.machine.derive(|api| api.header_attrs());

    view! {
        <div {..attrs.get()}>
            {children()}
        </div>
    }
}

#[component]
pub fn Title(children: Children) -> impl IntoView {
    let ctx = use_context::<Context>()
        .expect("floating_panel::Title must be used within FloatingPanel");
    let attrs = ctx.machine.derive(|api| api.title_attrs());

    view! {
        <div {..attrs.get()}>
            {children()}
        </div>
    }
}

#[component]
pub fn Content(children: Children) -> impl IntoView {
    let ctx = use_context::<Context>()
        .expect("floating_panel::Content must be used within FloatingPanel");
    let attrs = ctx.machine.derive(|api| api.content_attrs());

    view! {
        <div {..attrs.get()}>
            {children()}
        </div>
    }
}

#[component]
pub fn CloseTrigger(children: Children) -> impl IntoView {
    let ctx = use_context::<Context>()
        .expect("floating_panel::CloseTrigger must be used within FloatingPanel");
    let attrs = ctx.machine.derive(|api| api.close_trigger_attrs());

    view! {
        <button {..attrs.get()} on:click=move |_| {
            ctx.machine.send.run(floating_panel::Event::Close);
        }>
            {children()}
        </button>
    }
}

#[component]
pub fn MinimizeTrigger(children: Children) -> impl IntoView {
    let ctx = use_context::<Context>()
        .expect("floating_panel::MinimizeTrigger must be used within FloatingPanel");
    let attrs = ctx.machine.derive(|api| api.minimize_trigger_attrs());
    let is_minimized = ctx.machine.derive(|api| api.is_minimized());

    view! {
        <button {..attrs.get()} on:click=move |_| {
            if is_minimized.get() {
                ctx.machine.send.run(floating_panel::Event::Restore);
            } else {
                ctx.machine.send.run(floating_panel::Event::Minimize);
            }
        }>
            {children()}
        </button>
    }
}

#[component]
pub fn MaximizeTrigger(children: Children) -> impl IntoView {
    let ctx = use_context::<Context>()
        .expect("floating_panel::MaximizeTrigger must be used within FloatingPanel");
    let attrs = ctx.machine.derive(|api| api.maximize_trigger_attrs());
    let is_maximized = ctx.machine.derive(|api| api.is_maximized());

    view! {
        <button {..attrs.get()} on:click=move |_| {
            if is_maximized.get() {
                ctx.machine.send.run(floating_panel::Event::Restore);
            } else {
                ctx.machine.send.run(floating_panel::Event::Maximize);
            }
        }>
            {children()}
        </button>
    }
}
```

## 25. Reference Implementation Skeleton

```rust
let props = build_core_props_from_adapter_props();
let machine = use_machine::<floating_panel::Machine>(props);
provide_context(Context { machine });

let root_ref = create_root_ref();
let root_attrs = derive_root_attrs(machine);         // includes position/size/z-index
let header_attrs = derive_header_attrs(machine);
let drag_handle_attrs = derive_drag_handle_attrs(machine);
let title_attrs = derive_title_attrs(machine);
let content_attrs = derive_content_attrs(machine);
let footer_attrs = derive_footer_attrs(machine);
let resize_handle_attrs = |h| derive_resize_handle_attrs(machine, h);
let close_attrs = derive_close_trigger_attrs(machine);
let minimize_attrs = derive_minimize_trigger_attrs(machine);
let maximize_attrs = derive_maximize_trigger_attrs(machine);
let stage_attrs = derive_stage_trigger_attrs(machine);

// Client-only effects
setup_drag_pointer_capture(drag_handle_ref, machine);
setup_resize_pointer_capture(resize_handle_refs, machine);
setup_bring_to_front(root_ref, machine);
setup_keyboard_handlers(root_ref, machine);
setup_focus_blur_tracking(root_ref, machine);
setup_portal_mount(root_ref);

if props.modal {
    setup_focus_trap(root_ref);
    setup_inert_background();
}

sync_controlled_open(props.open, machine);
wire_callbacks(machine, props);

render_portal({
    render_root(root_ref, root_attrs, {
        render_header(header_attrs, {
            render_drag_handle(drag_handle_attrs);
            render_title(title_attrs);
            render_stage_trigger(stage_attrs);
            render_minimize_trigger(minimize_attrs);
            render_maximize_trigger(maximize_attrs);
            render_close_trigger(close_attrs);
        });
        render_content(content_attrs);
        render_footer(footer_attrs);
        render_resize_handles(resize_handle_attrs);
    });
});

on_cleanup(|| {
    remove_document_pointer_listeners();
    release_pointer_capture();
    release_z_index_slot();
    remove_portal();
    release_focus_trap();
    remove_inert();
});
```

## 26. Adapter Invariants

- Position, size, and z-index must always be driven by machine context, reflected as inline styles on Root. The adapter must never maintain a parallel position/size state.
- Document-level pointer listeners for drag and resize must only be attached during active operations and must be cleaned up on DragEnd/ResizeEnd or component unmount, whichever comes first.
- Pointer capture must be set on drag/resize start and released on end; failure to release capture must not prevent cleanup of other resources.
- All window control buttons (Close, Minimize, Maximize, StageTrigger) must use `<button type="button">` with proper `aria-label` from Messages.
- The aria-label on MinimizeTrigger and MaximizeTrigger must change dynamically based on current stage (showing "Restore" when in the corresponding minimized/maximized state).
- Content and Footer must be hidden (via `hidden` attribute) when the panel is minimized, not removed from the DOM.
- Resize handles must not render interaction affordances (cursor, pointer handlers) when the panel is maximized.
- DragHandle must not render drag affordances when the panel is maximized.
- When `modal=true`, focus must be trapped within the panel and background content must be marked `inert`.
- Callbacks must fire after the machine transition that triggered them, not before.
- `on_position_change_end` and `on_size_change_end` must fire exactly once per drag/resize operation, on DragEnd/ResizeEnd respectively.
- The compound context must be provided before any sub-component renders.

## 27. Accessibility and SSR Notes

- Root has `role="dialog"` and `aria-labelledby` pointing to the Title ID.
- When `modal=true`, Root additionally has `aria-modal="true"` and the adapter composes FocusScope for focus trapping.
- All resize handles must have directional `aria-label` from Messages (e.g., "Resize bottom-right").
- Resize handles must meet the 44x44px minimum touch target per WCAG 2.5.5.
- DragHandle has an `aria-label` for the move action from Messages.
- Window control buttons have `aria-label` values that change based on current state (Minimize/Restore, Maximize/Restore).
- Keyboard navigation: Tab cycles through interactive elements within the panel; Escape closes; arrow keys nudge position when the root is focused.
- SSR renders the structural dialog with all ARIA attributes and inline styles. All interaction behaviors (drag, resize, keyboard, focus) are client-only.
- `data-ars-state`, `data-ars-minimized`, `data-ars-maximized`, `data-ars-dragging`, `data-ars-resizing`, `data-ars-stage`, and `data-ars-focus-visible` data attributes are rendered during SSR based on initial state.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core prop, part, event, and callback parity. All 11 part types are mapped to compound sub-components. All 6 callbacks are wired. All state transitions (drag, resize, minimize, maximize, restore, close, bring-to-front, focus, blur, escape) are covered.

Intentional deviations: none. The compound component pattern (separate sub-components connected via context) is a structural adaptation to Leptos composition patterns, not a behavioral deviation.

Traceability note: This adapter spec makes explicit the adapter-owned concerns for pointer-capture drag/resize wiring, document-level listener lifecycle, z-index allocation integration, portal rendering, modal focus trapping, keyboard navigation, compound context distribution, and cleanup ordering.

## 29. Test Scenarios

- panel opens with correct initial position, size, and z-index inline styles
- drag interaction: pointerdown on DragHandle starts drag, pointermove updates position, pointerup ends drag
- resize interaction: pointerdown on each ResizeHandle variant starts resize, pointermove updates size, pointerup ends resize
- minimize/restore cycle via MinimizeTrigger
- maximize/restore cycle via MaximizeTrigger
- stage trigger cycles through stages correctly
- close via CloseTrigger
- close via Escape key when `close_on_escape=true`
- BringToFront updates z-index on pointerdown
- Content and Footer hidden when minimized
- drag and resize disabled when maximized
- `on_position_change` fires on each drag move; `on_position_change_end` fires once on drag end
- `on_size_change` fires on each resize move; `on_size_change_end` fires once on resize end
- `on_stage_change` fires on minimize, maximize, and restore
- `on_open_change` fires on close
- modal mode traps focus and sets inert on background
- keyboard arrow keys nudge position when root is focused
- resize handles have correct directional cursors and aria-labels
- document pointer listeners cleaned up on drag/resize end and on component unmount
- SSR renders initial structure with correct attrs and inline styles
- `lazy_mount` defers content rendering until first open
- `unmount_on_exit` removes content DOM after close

## 30. Test Oracle Notes

| Behavior                            | Preferred oracle type | Notes                                                                                       |
| ----------------------------------- | --------------------- | ------------------------------------------------------------------------------------------- |
| position/size inline styles on Root | DOM attrs             | Assert `left`, `top`, `width`, `height`, `z-index` in `style` attribute.                    |
| state data attributes               | DOM attrs             | Assert `data-ars-state`, `data-ars-minimized`, `data-ars-maximized`, `data-ars-stage`.      |
| drag/resize callback order          | callback order        | Verify `on_position_change` fires during drag, `on_position_change_end` fires exactly once. |
| compound context availability       | context registration  | Sub-components must find `Context` or panic with documented message.                        |
| structural rendering (all parts)    | rendered structure    | Assert Root, Header, DragHandle, Title, Content, trigger buttons all present.               |
| hydration stability                 | hydration structure   | Verify server and client render produce identical initial DOM.                              |
| document listener cleanup           | cleanup side effects  | Verify no document-level pointer listeners remain after unmount.                            |

Cheap verification recipe:

1. Render the panel and assert all structural parts are present with correct attrs.
2. Simulate pointerdown/pointermove/pointerup on DragHandle and verify position changes via inline styles and callback invocations.
3. Simulate pointerdown/pointermove/pointerup on each ResizeHandle variant and verify size changes.
4. Click MinimizeTrigger, verify Content/Footer are hidden and `data-ars-stage="minimized"`.
5. Click MaximizeTrigger, verify Root position/size update and `data-ars-stage="maximized"`.
6. Press Escape, verify panel closes and `on_open_change(false)` fires.
7. Unmount and verify no document listeners or portal nodes remain.

## 31. Implementation Checklist

- [ ] FloatingPanel initializes machine and provides compound context.
- [ ] Root renders with correct `role="dialog"`, `aria-labelledby`, and inline position/size/z-index styles.
- [ ] All 11 part types are mapped to sub-components with correct machine-derived attrs.
- [ ] Drag interaction uses pointer capture with document-level listeners; sends DragStart/DragMove/DragEnd.
- [ ] Resize interaction uses pointer capture with document-level listeners; sends ResizeStart/ResizeMove/ResizeEnd.
- [ ] BringToFront fires on any pointerdown on Root and allocates a new z-index.
- [ ] Window control buttons (Close, Minimize, Maximize, StageTrigger) send correct events and have dynamic aria-labels.
- [ ] Content and Footer are hidden when minimized (not removed).
- [ ] Drag and resize are disabled when panel is maximized.
- [ ] Keyboard: Escape closes, arrows nudge position.
- [ ] Modal mode composes FocusScope for focus trapping and sets inert on background.
- [ ] Controlled `open` prop syncs correctly.
- [ ] All callbacks fire at documented timing with correct payloads.
- [ ] Document-level pointer listeners are cleaned up on drag/resize end and component unmount.
- [ ] Portal rendering is client-only with correct lifecycle.
- [ ] Z-index allocation slot is released on cleanup.
- [ ] `lazy_mount` and `unmount_on_exit` are respected.
- [ ] SSR renders correct initial structure and attrs without attaching listeners.
- [ ] Focus/blur tracking with pointer-modality normalization.
- [ ] Resize handles have 44x44px minimum touch targets per WCAG 2.5.5.
