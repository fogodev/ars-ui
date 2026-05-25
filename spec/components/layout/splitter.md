---
component: Splitter
category: layout
tier: stateful
foundation_deps: [architecture, accessibility, interactions]
shared_deps: [layout-shared-types]
related: []
references:
    ark-ui: Splitter
---

# Splitter

`Splitter` renders a group of panels separated by drag handles. Users can resize adjacent panels by dragging a handle or using keyboard shortcuts. Supports horizontal and vertical orientations, per-panel min/max sizes (pixels or percentages), collapsible panels with snap-to-collapsed behaviour, keyboard-driven resize via arrow keys, and RTL-aware horizontal layout. The component is headless: it manages state and produces data/ARIA attributes. All visual presentation is the consumer's CSS.

## 1. State Machine

### 1.1 States

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// No drag in progress.
    Idle,
    /// User is dragging the handle at the given index (0-based,
    /// between panels `handle_index` and `handle_index + 1`).
    Dragging { handle_index: usize },
}

impl Default for State {
    fn default() -> Self { State::Idle }
}
```

### 1.2 Events

```rust
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Pointer pressed on handle; `pos` is client coordinate along split axis.
    DragStart { handle_index: usize, pos: f64 },
    /// Pointer moved while dragging.
    DragMove { pos: f64 },
    /// Pointer released or cancelled.
    DragEnd,
    /// Key pressed on handle.
    KeyDown { handle_index: usize, event: KeyboardEvent },
    /// Handle received focus.
    HandleFocus { handle_index: usize },
    /// Handle lost focus.
    HandleBlur,
    /// Programmatically collapse a panel.
    CollapsePanel { panel_index: usize },
    /// Programmatically expand a collapsed panel.
    ExpandPanel { panel_index: usize },
    /// Programmatically set all sizes.
    SetSizes { sizes: Vec<f64> },
    /// Synchronize context-backed props after a runtime prop change.
    SyncProps { props: Props },
}

/// Keyboard event mirror (only the fields needed by the splitter).
#[derive(Clone, Debug, PartialEq)]
pub struct KeyboardEvent {
    pub key: KeyboardKey,
    pub shift: bool,
    pub alt: bool,
    pub ctrl: bool,
    pub meta: bool,
}
```

### 1.3 Context

```rust
use crate::{Bindable, Duration};
use ars_i18n::{Orientation, Direction};

/// Definition for a single panel within the splitter.
#[derive(Clone, Debug, PartialEq)]
pub struct Panel {
    /// Stable identifier for this panel.
    pub id: String,
    /// Minimum size in the configured unit.
    pub min_size: f64,
    /// Optional hard maximum size. `None` means unconstrained.
    pub max_size: Option<f64>,
    /// Initial size when no external value is provided.
    pub default_size: f64,
    /// Whether this panel can be collapsed entirely.
    pub collapsible: bool,
    /// Size when collapsed. Defaults to `0.0`.
    pub collapsed_size: f64,
    /// Fraction of `min_size` at which panel snaps to `collapsed_size`.
    /// Must be in `0.0..=1.0`. Defaults to `0.5`.
    pub collapse_threshold: f64,
}

impl Default for Panel {
    fn default() -> Self {
        Panel {
            id: String::new(), min_size: 0.0, max_size: None,
            default_size: 100.0, collapsible: false,
            collapsed_size: 0.0, collapse_threshold: 0.5,
        }
    }
}

/// Unit for panel sizes.
#[derive(Clone, Debug, Copy, PartialEq, Eq, Default)]
pub enum SizeUnit {
    #[default]
    Percent,
    Pixels,
}

/// Mutable runtime context for `Splitter`.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Current size of each panel.
    pub sizes: Bindable<Vec<f64>>,
    /// Static panel definitions.
    pub panels: Vec<Panel>,
    /// Split orientation.
    pub orientation: Orientation,
    /// Text direction (drives RTL arrow key delta inversion).
    pub dir: Direction,
    /// Unit for all sizes.
    pub size_unit: SizeUnit,
    /// Sizes at drag start (for computing deltas without floating-point error).
    pub drag_start_sizes: Vec<f64>,
    /// Last expanded sizes remembered for collapsible panels.
    pub collapsed_restore_sizes: Vec<Option<f64>>,
    /// Pointer coordinate at drag start.
    pub drag_start_pos: f64,
    /// Keyboard resize step size. Defaults to `10.0` (percent) or `20.0` (pixels).
    pub keyboard_step: f64,
    /// Index of the handle with keyboard focus.
    pub focused_handle: Option<usize>,
    /// Resolved locale for message formatting.
    pub locale: Locale,
    /// Resolved translatable messages.
    pub messages: Messages,
    /// Component IDs.
    pub ids: ComponentIds,
    /// CSS zoom / transform scale factor (adapter-computed on DragStart).
    pub drag_scale_factor: f64,
}

impl Context {
    pub fn from_props(props: &Props, env: &Env, messages: &Messages) -> Self {
        let sizes = initial_sizes(props);
        let normalized_sizes = clamp_all(&sizes, &props.panels);
        let locale = env.locale.clone();
        let messages = messages.clone();
        Context {
            sizes: props.sizes.as_ref().map_or_else(
                || Bindable::uncontrolled(normalized_sizes.clone()),
                |sizes| Bindable::controlled(clamp_all(sizes.get(), &props.panels)),
            ),
            panels: props.panels.clone(),
            orientation: props.orientation,
            dir: props.dir.unwrap_or(Direction::Ltr),
            size_unit: props.size_unit,
            drag_start_sizes: Vec::new(),
            collapsed_restore_sizes: vec![None; props.panels.len()],
            drag_start_pos: 0.0,
            keyboard_step: keyboard_step_for(props),
            focused_handle: None,
            locale,
            messages,
            ids: ComponentIds::from_id(&props.id),
            drag_scale_factor: 1.0,
        }
    }
}
```

### 1.4 Props

```rust
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The id of the component.
    pub id: String,
    /// Panel definitions.
    pub panels: Vec<Panel>,
    /// Split orientation.
    pub orientation: Orientation,
    /// Text direction.
    pub dir: Option<Direction>,
    /// Size unit for all panels.
    pub size_unit: SizeUnit,
    /// Controlled sizes; `None` for uncontrolled.
    pub sizes: Option<Bindable<Vec<f64>>>,
    /// Initial sizes for uncontrolled mode.
    pub default_sizes: Option<Vec<f64>>,
    /// Total pixel length when `size_unit == Pixels` and no `default_sizes`.
    pub initial_total_px: Option<f64>,
    /// Keyboard resize step size. `Shift+Arrow` uses `5 * keyboard_step`.
    pub keyboard_step: Option<f64>,
    /// Key for persisting sizes to localStorage across sessions.
    pub storage_key: Option<String>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            panels: Vec::new(),
            orientation: Orientation::Horizontal,
            dir: None,
            size_unit: SizeUnit::Percent,
            sizes: None,
            default_sizes: None,
            initial_total_px: None,
            keyboard_step: None,
            storage_key: None,
        }
    }
}
```

### 1.5 Resize Algorithm

```rust
/// Compute new panel sizes after dragging handle at `handle_index` by `delta` units.
pub fn compute_resize(
    sizes: &[f64], handle_index: usize, delta: f64, panels: &[Panel],
) -> Vec<f64> {
    if !valid_handle(handle_index, sizes.len(), panels.len()) || !delta.is_finite() {
        return sizes.to_vec();
    }

    let mut new_sizes = sizes.to_vec();
    let (left, right) = (handle_index, handle_index + 1);

    let left_growth = panels[left].max_size.map(|m| m - new_sizes[left]).unwrap_or(f64::INFINITY);
    let right_shrink = new_sizes[right] - effective_min(&panels[right]);
    let left_shrink = new_sizes[left] - effective_min(&panels[left]);
    let right_growth = panels[right].max_size.map(|m| m - new_sizes[right]).unwrap_or(f64::INFINITY);

    let (left_delta, right_delta) = if delta > 0.0 {
        let d = delta.min(left_growth).min(right_shrink).max(0.0);
        (d, -d)
    } else {
        let d = (-delta).min(left_shrink).min(right_growth).max(0.0);
        (-d, d)
    };

    new_sizes[left] += left_delta;
    new_sizes[right] += right_delta;
    snap_if_collapsible(&mut new_sizes, left, panels, delta < 0.0);
    snap_if_collapsible(&mut new_sizes, right, panels, delta > 0.0);
    new_sizes
}

fn effective_min(panel: &Panel) -> f64 {
    if panel.collapsible { panel.collapsed_size } else { panel.min_size }
}

fn snap_if_collapsible(sizes: &mut Vec<f64>, index: usize, panels: &[Panel], moving_toward_collapse: bool) {
    let p = &panels[index];
    if !p.collapsible { return; }
    let threshold = p.min_size * p.collapse_threshold + p.collapsed_size * (1.0 - p.collapse_threshold);
    if moving_toward_collapse && sizes[index] < threshold {
        sizes[index] = p.collapsed_size;
    } else if sizes[index] < p.min_size && sizes[index] > p.collapsed_size {
        sizes[index] = p.min_size;
    }
}

fn clamp_all(sizes: &[f64], panels: &[Panel]) -> Vec<f64> {
    clamp_all_with_current(sizes, panels, sizes)
}

fn clamp_all_with_current(sizes: &[f64], panels: &[Panel], current: &[f64]) -> Vec<f64> {
    let normalized = normalize_sizes(sizes, panels, current);
    normalized.iter().zip(panels.iter()).map(|(&s, p)| {
        s.max(effective_min(p)).min(p.max_size.unwrap_or(f64::INFINITY))
    }).collect()
}

fn rebalance_to_total(mut sizes: Vec<f64>, panels: &[Panel], total: f64) -> Vec<f64> {
    if !total.is_finite() || total <= 0.0 {
        return sizes;
    }
    let current_total = sizes.iter().sum::<f64>();
    let mut delta = current_total - total;
    if delta > 0.0 {
        for index in (0..sizes.len()).rev() {
            let floor = panels.get(index).map_or(0.0, effective_min);
            let reduction = (sizes[index] - floor).max(0.0).min(delta);
            sizes[index] -= reduction;
            delta -= reduction;
            if delta <= f64::EPSILON {
                break;
            }
        }
    } else if delta < 0.0 {
        let mut remaining = -delta;
        for index in (0..sizes.len()).rev() {
            let ceiling = panels
                .get(index)
                .and_then(|panel| panel.max_size)
                .unwrap_or(f64::INFINITY);
            let increase = (ceiling - sizes[index]).max(0.0).min(remaining);
            sizes[index] += increase;
            remaining -= increase;
            if remaining <= f64::EPSILON {
                break;
            }
        }
    }
    sizes
}

fn normalize_sizes(sizes: &[f64], panels: &[Panel], current: &[f64]) -> Vec<f64> {
    panels.iter().enumerate().map(|(index, panel)| {
        let candidate = sizes
            .get(index)
            .copied()
            .or_else(|| current.get(index).copied())
            .unwrap_or(panel.default_size);
        if candidate.is_finite() {
            candidate
        } else {
            current
                .get(index)
                .copied()
                .filter(|size| size.is_finite())
                .unwrap_or(panel.default_size)
        }
    }).collect()
}

fn initial_sizes(props: &Props) -> Vec<f64> {
    if let Some(default_sizes) = &props.default_sizes {
        return default_sizes.clone();
    }
    let len = props.panels.len();
    if len == 0 {
        return Vec::new();
    }
    let share = match props.size_unit {
        SizeUnit::Percent => 100.0 / len as f64,
        SizeUnit::Pixels => props.initial_total_px.unwrap_or(600.0) / len as f64,
    };
    vec![share; len]
}

fn valid_handle(handle_index: usize, sizes_len: usize, panels_len: usize) -> bool {
    if let Some(next) = handle_index.checked_add(1) {
        next < sizes_len && next < panels_len
    } else {
        false
    }
}

fn keyboard_step_for(props: &Props) -> f64 {
    let default = match props.size_unit {
        SizeUnit::Percent => 10.0,
        SizeUnit::Pixels => 20.0,
    };
    finite_positive(props.keyboard_step.unwrap_or(default), default)
}

fn finite_positive(value: f64, fallback: f64) -> f64 {
    if value.is_finite() && value > 0.0 { value } else { fallback }
}

fn collapse_panel(sizes: &mut [f64], index: usize, panels: &[Panel]) {
    if index >= sizes.len() || index >= panels.len() || !panels[index].collapsible {
        return;
    }
    let collapsed_size = panels[index].collapsed_size;
    let freed = (sizes[index] - collapsed_size).max(0.0);

    if sizes.len() == 1 {
        sizes[index] = collapsed_size;
        return;
    }

    let mut remaining = freed;

    for recipient in (index + 1..sizes.len()).chain((0..index).rev()) {
        let capacity = panels
            .get(recipient)
            .and_then(|panel| panel.max_size)
            .map_or(remaining, |max_size| {
                (max_size - sizes[recipient]).max(0.0).min(remaining)
            });

        sizes[recipient] += capacity;
        remaining -= capacity;

        if remaining <= 0.0 {
            break;
        }
    }

    sizes[index] = collapsed_size + remaining;
}

fn expand_panel(sizes: &mut [f64], index: usize, panels: &[Panel], restore_size: Option<f64>) -> bool {
    if index >= sizes.len() || index >= panels.len() { return false; }
    let p = &panels[index];
    if !p.collapsible { return false; }
    if sizes[index] > p.collapsed_size { return false; }
    let target_size = restore_size
        .filter(|size| size.is_finite())
        .unwrap_or(p.default_size)
        .min(p.max_size.unwrap_or(f64::INFINITY));
    let need = (target_size - sizes[index]).max(0.0);
    let donor = if index + 1 < sizes.len() { Some(index + 1) }
        else if index > 0 { Some(index - 1) }
        else { None };
    if let Some(d) = donor {
        let actual = need.min((sizes[d] - effective_min(&panels[d])).max(0.0));
        sizes[index] += actual;
        sizes[d] -= actual;
        return actual > 0.0;
    }
    false
}

fn remember_collapse_size(ctx: &mut Context, index: usize) {
    if index >= ctx.panels.len() || index >= ctx.sizes.get().len() {
        return;
    }
    if !ctx.panels[index].collapsible {
        return;
    }
    let size = ctx.sizes.get()[index];
    if size > ctx.panels[index].collapsed_size {
        if ctx.collapsed_restore_sizes.len() != ctx.panels.len() {
            ctx.collapsed_restore_sizes.resize(ctx.panels.len(), None);
        }
        ctx.collapsed_restore_sizes[index] = Some(size);
    }
}

fn commit_sizes(ctx: &mut Context, sizes: Vec<f64>) {
    if sizes.len() != ctx.panels.len() || !sizes.iter().all(|s| s.is_finite()) {
        return;
    }
    ctx.sizes.set(sizes.clone());
    if ctx.sizes.is_controlled() {
        ctx.sizes.sync_controlled(Some(sizes));
    }
}

fn handle_keyboard(ctx: &mut Context, handle_index: usize, event: &KeyboardEvent) {
    if !valid_handle(handle_index, ctx.sizes.get().len(), ctx.panels.len()) {
        return;
    }
    let step = if event.shift { ctx.keyboard_step * 5.0 } else { ctx.keyboard_step };
    let is_rtl = ctx.dir == Direction::Rtl;
    match (&event.key, &ctx.orientation) {
        (KeyboardKey::ArrowRight, Orientation::Horizontal)
        | (KeyboardKey::ArrowDown, Orientation::Vertical) => {
            let delta = rtl_adjusted_delta(step, ctx.orientation, is_rtl);
            let new = compute_resize(&ctx.sizes.get().to_vec(), handle_index, delta, &ctx.panels);
            commit_sizes(ctx, new);
        }
        (KeyboardKey::ArrowLeft, Orientation::Horizontal)
        | (KeyboardKey::ArrowUp, Orientation::Vertical) => {
            let delta = rtl_adjusted_delta(-step, ctx.orientation, is_rtl);
            let new = compute_resize(&ctx.sizes.get().to_vec(), handle_index, delta, &ctx.panels);
            commit_sizes(ctx, new);
        }
        (KeyboardKey::Home, _) => {
            let sizes = ctx.sizes.get().to_vec();
            let delta = effective_min(&ctx.panels[handle_index]) - sizes[handle_index];
            let new = compute_resize(&sizes, handle_index, delta, &ctx.panels);
            commit_sizes(ctx, new);
        }
        (KeyboardKey::End, _) => {
            let sizes = ctx.sizes.get().to_vec();
            let total: f64 = sizes.iter().sum();
            let max = ctx.panels[handle_index].max_size.unwrap_or(total);
            let delta = max - sizes[handle_index];
            let new = compute_resize(&sizes, handle_index, delta, &ctx.panels);
            commit_sizes(ctx, new);
        }
        (KeyboardKey::Enter, _) | (KeyboardKey::Space, _) => {
            let p = &ctx.panels[handle_index];
            if p.collapsible {
                let mut sizes = ctx.sizes.get().to_vec();
                if sizes[handle_index] <= p.collapsed_size {
                    let restore_size = ctx.collapsed_restore_sizes.get(handle_index).copied().flatten();
                    let expanded = expand_panel(&mut sizes, handle_index, &ctx.panels, restore_size);
                    if expanded
                        && let Some(restore_size) = ctx.collapsed_restore_sizes.get_mut(handle_index)
                    {
                        *restore_size = None;
                    }
                } else {
                    remember_collapse_size(ctx, handle_index);
                    collapse_panel(&mut sizes, handle_index, &ctx.panels);
                }
                commit_sizes(ctx, sizes);
            }
        }
        _ => {}
    }
}

fn rtl_adjusted_delta(delta: f64, orientation: Orientation, is_rtl: bool) -> f64 {
    if is_rtl && orientation == Orientation::Horizontal { -delta } else { delta }
}
```

### 1.6 Full Machine Implementation

```rust
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Effect = NoEffect;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        (State::Idle, Context::from_props(props, env, messages))
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            (State::Idle, Event::DragStart { handle_index, pos })
                if pos.is_finite() && valid_handle(*handle_index, ctx.sizes.get().len(), ctx.panels.len()) =>
            {
                let (hi, p) = (*handle_index, *pos);
                Some(TransitionPlan::to(State::Dragging { handle_index: hi }).apply(move |ctx| {
                    ctx.drag_start_sizes = ctx.sizes.get().to_vec();
                    ctx.drag_start_pos = p;
                }))
            }
            (State::Idle, Event::DragStart { .. }) => None,
            (State::Idle, Event::KeyDown { handle_index, event }) => {
                let (hi, ev) = (*handle_index, event.clone());
                Some(TransitionPlan::context_only(move |ctx| { handle_keyboard(ctx, hi, &ev); }))
            }
            (State::Idle, Event::HandleFocus { handle_index }) => {
                let hi = *handle_index;
                if valid_handle(hi, ctx.sizes.get().len(), ctx.panels.len()) {
                    Some(TransitionPlan::context_only(move |ctx| { ctx.focused_handle = Some(hi); }))
                } else {
                    Some(TransitionPlan::new())
                }
            }
            (State::Idle, Event::HandleBlur) => {
                Some(TransitionPlan::context_only(|ctx| { ctx.focused_handle = None; }))
            }
            (State::Idle, Event::CollapsePanel { panel_index }) => {
                let pi = *panel_index;
                Some(TransitionPlan::context_only(move |ctx| {
                    let mut sizes = ctx.sizes.get().to_vec();
                    remember_collapse_size(ctx, pi);
                    collapse_panel(&mut sizes, pi, &ctx.panels);
                    commit_sizes(ctx, sizes);
                }))
            }
            (State::Idle, Event::ExpandPanel { panel_index }) => {
                let pi = *panel_index;
                Some(TransitionPlan::context_only(move |ctx| {
                    let mut sizes = ctx.sizes.get().to_vec();
                    let restore_size = ctx.collapsed_restore_sizes.get(pi).copied().flatten();
                    let expanded = expand_panel(&mut sizes, pi, &ctx.panels, restore_size);
                    if expanded
                        && let Some(restore_size) = ctx.collapsed_restore_sizes.get_mut(pi)
                    {
                        *restore_size = None;
                    }
                    commit_sizes(ctx, sizes);
                }))
            }
            (State::Idle, Event::SetSizes { sizes }) => {
                let (panels, s, unit, total) = (
                    ctx.panels.clone(),
                    sizes.clone(),
                    ctx.size_unit,
                    ctx.sizes.get().iter().sum::<f64>(),
                );
                Some(TransitionPlan::context_only(move |ctx| {
                    let mut next = clamp_all(&s, &panels);
                    if unit == SizeUnit::Percent {
                        next = rebalance_to_total(next, &panels, total);
                    }
                    commit_sizes(ctx, next);
                }))
            }
            (State::Dragging { handle_index }, Event::DragMove { pos }) => {
                let (hi, p) = (*handle_index, *pos);
                let (start_pos, start_sizes, panels, scale) =
                    (ctx.drag_start_pos, ctx.drag_start_sizes.clone(), ctx.panels.clone(), ctx.drag_scale_factor);
                Some(TransitionPlan::context_only(move |ctx| {
                    let delta = (p - start_pos) / finite_positive(scale, 1.0);
                    commit_sizes(ctx, compute_resize(&start_sizes, hi, delta, &panels));
                }))
            }
            (State::Dragging { .. }, Event::DragEnd) => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.drag_start_sizes.clear(); ctx.drag_start_pos = 0.0;
                }))
            }
            (State::Dragging { handle_index }, Event::KeyDown { event, .. }) => {
                let hi = *handle_index;
                match &event.key {
                    KeyboardKey::Escape => {
                        let pre_drag = ctx.drag_start_sizes.clone();
                        Some(TransitionPlan::to(State::Idle).apply(move |ctx| {
                            commit_sizes(ctx, pre_drag);
                            ctx.drag_start_sizes.clear(); ctx.drag_start_pos = 0.0;
                        }))
                    }
                    KeyboardKey::ArrowLeft | KeyboardKey::ArrowRight
                    | KeyboardKey::ArrowUp | KeyboardKey::ArrowDown
                    | KeyboardKey::Home | KeyboardKey::End
                    | KeyboardKey::Enter | KeyboardKey::Space => {
                        let ev = event.clone();
                        Some(TransitionPlan::context_only(move |ctx| { handle_keyboard(ctx, hi, &ev); }))
                    }
                    _ => None,
                }
            }
            (_, Event::SyncProps { props }) => {
                let props = props.clone();
                let exit_dragging = matches!(state, State::Dragging { .. });
                let plan = if exit_dragging {
                    TransitionPlan::to(State::Idle)
                } else {
                    TransitionPlan::new()
                };
                Some(plan.apply(move |ctx| {
                    let current = ctx.sizes.get().to_vec();
                    let panels_changed = ctx.panels != props.panels;
                    ctx.panels = props.panels.clone();
                    ctx.orientation = props.orientation;
                    ctx.dir = props.dir.unwrap_or(Direction::Ltr);
                    ctx.size_unit = props.size_unit;
                    ctx.keyboard_step = keyboard_step_for(&props);
                    if panels_changed {
                        ctx.collapsed_restore_sizes = vec![None; ctx.panels.len()];
                    } else {
                        ctx.collapsed_restore_sizes.resize(ctx.panels.len(), None);
                    }
                    ctx.sizes.sync_controlled(
                        props
                            .sizes
                            .clone()
                            .map(|sizes| clamp_all_with_current(sizes.get(), &ctx.panels, &current)),
                    );
                    if !ctx.sizes.is_controlled() {
                        commit_sizes(ctx, clamp_all_with_current(&current, &ctx.panels, &current));
                    }
                    if let Some(focused) = ctx.focused_handle
                        && !valid_handle(focused, ctx.sizes.get().len(), ctx.panels.len())
                    {
                        ctx.focused_handle = None;
                    }
                    if exit_dragging {
                        ctx.drag_start_sizes.clear();
                        ctx.drag_start_pos = 0.0;
                    }
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
        assert_eq!(old.id, new.id, "Splitter id cannot change after initialization");

        if old == new {
            Vec::new()
        } else {
            vec![Event::SyncProps { props: new.clone() }]
        }
    }
}
```

### 1.7 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "splitter"]
pub enum Part {
    Root,
    Panel { index: usize },
    Handle { index: usize },
}

pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-orientation"), match self.ctx.orientation {
            Orientation::Horizontal => "horizontal",
            Orientation::Vertical => "vertical",
        });
        attrs.set(HtmlAttr::Data("ars-state"), match self.state {
            State::Idle => "idle",
            State::Dragging { .. } => "dragging",
        });
        attrs
    }

    pub fn panel_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Panel { index }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);

        let Some(panel) = self.ctx.panels.get(index) else {
            return attrs;
        };
        let Some(size) = self.ctx.sizes.get().get(index).copied() else {
            return attrs;
        };

        attrs.set(HtmlAttr::Data("ars-panel-id"), &panel.id);
        let collapsed = panel.collapsible && size <= panel.collapsed_size;
        if collapsed { attrs.set_bool(HtmlAttr::Data("ars-collapsed"), true); }
        let unit = match self.ctx.size_unit {
            SizeUnit::Percent => "%",
            SizeUnit::Pixels => "px",
        };
        match self.ctx.orientation {
            Orientation::Horizontal => attrs.set_style(CssProperty::Width, format!("{size}{unit}")),
            Orientation::Vertical => attrs.set_style(CssProperty::Height, format!("{size}{unit}")),
        };
        attrs
    }

    pub fn handle_attrs(&self, handle_index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Handle { index: handle_index }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-handle-index"), handle_index.to_string());
        if !valid_handle(handle_index, self.ctx.sizes.get().len(), self.ctx.panels.len()) {
            return attrs;
        }
        let is_focused = self.ctx.focused_handle == Some(handle_index);
        let is_dragging = matches!(self.state, State::Dragging { handle_index: hi } if *hi == handle_index);
        attrs.set(HtmlAttr::Data("ars-state"), if is_dragging { "dragging" } else if is_focused { "focus" } else { "idle" });

        // ARIA separator attributes
        attrs.set(HtmlAttr::Role, "separator");
        attrs.set(HtmlAttr::Aria(AriaAttr::Orientation), match self.ctx.orientation {
            Orientation::Horizontal => "vertical",
            Orientation::Vertical => "horizontal",
        });

        let (left, right) = (handle_index, handle_index + 1);
        let sizes = self.ctx.sizes.get();
        let total: f64 = sizes.iter().sum::<f64>().max(1.0);
        let to_pct = |v: f64| v / total * 100.0;

        let value_now = to_pct(sizes[left]);
        let value_min = to_pct(effective_min(&self.ctx.panels[left]));
        let fallback_max = total - effective_min(&self.ctx.panels[right]);
        let value_max = to_pct(
            self.ctx.panels[left]
                .max_size
                .unwrap_or(fallback_max)
                .min(fallback_max),
        );

        attrs.set(HtmlAttr::Aria(AriaAttr::ValueNow), (value_now.round() as i64).to_string());
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueMin), (value_min.round() as i64).to_string());
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueMax), (value_max.round() as i64).to_string());

        let collapsed = sizes[left] <= self.ctx.panels[left].collapsed_size && self.ctx.panels[left].collapsible;
        if collapsed {
            attrs.set(HtmlAttr::Aria(AriaAttr::ValueText), (self.ctx.messages.panel_collapsed)(&self.ctx.locale));
        } else {
            attrs.set(HtmlAttr::Aria(AriaAttr::ValueText), (self.ctx.messages.panel_size_text)(value_now, &self.ctx.locale));
        }
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.resize_handle_label)(handle_index, &self.ctx.locale));
        attrs.set(HtmlAttr::Aria(AriaAttr::Controls), self.ctx.ids.item("panel", &left));
        attrs.set(HtmlAttr::TabIndex, "0");
        attrs
    }

    /// Programmatically collapse a panel by index.
    pub fn collapse_panel(&self, panel_index: usize) {
        (self.send)(Event::CollapsePanel { panel_index });
    }

    /// Programmatically expand a collapsed panel by index.
    pub fn expand_panel(&self, panel_index: usize) {
        (self.send)(Event::ExpandPanel { panel_index });
    }

    /// Programmatically resize a specific panel to the given size.
    pub fn resize_panel(&self, panel_index: usize, size: f64) {
        let mut sizes = self.ctx.sizes.get().to_vec();
        if panel_index < sizes.len() {
            sizes[panel_index] = size;
            (self.send)(Event::SetSizes { sizes });
        }
    }

    /// Reset all panel sizes to their `default_size` values.
    pub fn reset_sizes(&self) {
        let sizes = self.ctx.panels.iter().map(|p| p.default_size).collect();
        (self.send)(Event::SetSizes { sizes });
    }

    pub fn on_handle_pointerdown(&self, handle_index: usize, pos: f64) {
        (self.send)(Event::DragStart { handle_index, pos });
    }
    pub fn on_handle_pointermove(&self, pos: f64) { (self.send)(Event::DragMove { pos }); }
    pub fn on_handle_pointerup(&self) { (self.send)(Event::DragEnd); }
    pub fn on_handle_keydown(&self, handle_index: usize, event: KeyboardEvent) {
        (self.send)(Event::KeyDown { handle_index, event });
    }
    pub fn on_handle_focus(&self, handle_index: usize) { (self.send)(Event::HandleFocus { handle_index }); }
    pub fn on_handle_blur(&self) { (self.send)(Event::HandleBlur); }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Panel { index } => self.panel_attrs(index),
            Part::Handle { index } => self.handle_attrs(index),
        }
    }
}
```

## 2. Anatomy

```text
Splitter
├── Root     <div>  data-ars-orientation data-ars-state="idle|dragging"
│   ├── Panel  (×N) <div>  inline width/height, data-ars-collapsed
│   ├── Handle (×N-1) <div>  role="separator" aria-valuenow tabindex="0"
│   └── Panel  (×N) <div>
```

| Part   | Element | Key Attributes                                                     |
| ------ | ------- | ------------------------------------------------------------------ |
| Root   | `<div>` | `data-ars-orientation`, `data-ars-state`                           |
| Panel  | `<div>` | Inline `width`/`height`, `data-ars-panel-id`, `data-ars-collapsed` |
| Handle | `<div>` | `role="separator"`, `aria-valuenow/min/max`, `tabindex="0"`        |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Element | Attribute          | Value                                            |
| ------- | ------------------ | ------------------------------------------------ |
| Handle  | `role`             | `"separator"`                                    |
| Handle  | `aria-orientation` | Perpendicular to splitter orientation            |
| Handle  | `aria-valuenow`    | Current panel size as percentage (0-100)         |
| Handle  | `aria-valuemin`    | Minimum panel size (0 if collapsible)            |
| Handle  | `aria-valuemax`    | Maximum panel size                               |
| Handle  | `aria-valuetext`   | Human-readable size (e.g., "50%") or "Collapsed" |
| Handle  | `aria-label`       | From Messages (e.g., "Resize")                   |
| Handle  | `aria-controls`    | ID of the governed panel                         |
| Handle  | `tabindex`         | `"0"` (all handles in tab order)                 |

Panels need no special ARIA role.

### 3.2 Keyboard Interaction

| Key                        | Behaviour                                              |
| -------------------------- | ------------------------------------------------------ |
| `ArrowRight` / `ArrowDown` | Move handle one step toward end                        |
| `ArrowLeft` / `ArrowUp`    | Move handle one step toward start                      |
| `Shift+Arrow`              | Move handle by `5 * keyboard_step` (coarse adjustment) |
| `Home`                     | Collapse panel to minimum                              |
| `End`                      | Expand panel to maximum                                |
| `Enter` / `Space`          | Toggle collapse/expand (collapsible panels only)       |
| `Escape`                   | Cancel drag, restore pre-drag sizes                    |

Keyboard works in both `Idle` and `Dragging` states. RTL: In horizontal orientation with `dir="rtl"`, ArrowRight/ArrowLeft deltas are inverted via `rtl_adjusted_delta()` per `03-accessibility.md` §4.1. Vertical orientation is unaffected by RTL.

## 4. Internationalization

### 4.1 Messages

```rust
#[derive(Clone, Debug)]
pub struct Messages {
    /// Label for the resize handle. Default: `"Resize"`.
    pub resize_handle_label: MessageFn<dyn Fn(usize, &Locale) -> String + Send + Sync>,
    /// Template for announcing current value. Receives size as percentage.
    pub panel_size_text: MessageFn<dyn Fn(f64, &Locale) -> String + Send + Sync>,
    /// Text announced when a panel is collapsed.
    pub panel_collapsed: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            resize_handle_label: MessageFn::new(|_index, _locale| "Resize".to_string()),
            panel_size_text: MessageFn::new(|value, _locale| format!("{value:.0}%")),
            panel_collapsed: MessageFn::static_str("Collapsed"),
        }
    }
}

impl ComponentMessages for Messages {}
```

## 5. Library Parity

> Compared against: Ark UI (`Splitter`).

### 5.1 Props

| Feature             | ars-ui                       | Ark UI                | Notes                                                |
| ------------------- | ---------------------------- | --------------------- | ---------------------------------------------------- |
| Panel definitions   | `panels: Vec<Panel>`         | `panels: PanelData[]` | Same concept; ars-ui richer (collapsible, threshold) |
| Controlled sizes    | `sizes` (Bindable)           | `size`                | Same                                                 |
| Default sizes       | `default_sizes`              | `defaultSize`         | Same                                                 |
| Orientation         | `orientation`                | `orientation`         | Same                                                 |
| Keyboard step       | `keyboard_step`              | `keyboardResizeBy`    | Same                                                 |
| Direction (RTL)     | `dir`                        | --                    | ars-ui addition                                      |
| Size unit           | `size_unit` (Percent/Pixels) | --                    | ars-ui addition                                      |
| Storage persistence | `storage_key`                | --                    | ars-ui addition                                      |
| Locale/messages     | `locale`, `messages`         | --                    | ars-ui addition                                      |

**Gaps:** None.

### 5.2 Anatomy

| Part             | ars-ui   | Ark UI                   | Notes                                        |
| ---------------- | -------- | ------------------------ | -------------------------------------------- |
| Root             | `Root`   | `Root`                   | --                                           |
| Panel            | `Panel`  | `Panel`                  | --                                           |
| Handle           | `Handle` | `ResizeTrigger`          | Different naming                             |
| Handle indicator | --       | `ResizeTriggerIndicator` | Visual child; consumer renders inside Handle |

**Gaps:** None. `ResizeTriggerIndicator` is a visual child element the consumer renders inside the Handle.

### 5.3 Events

| Callback     | ars-ui                 | Ark UI          | Notes                             |
| ------------ | ---------------------- | --------------- | --------------------------------- |
| Resize start | `Event::DragStart`     | `onResizeStart` | State machine event               |
| Resize       | `Bindable` change      | `onResize`      | Handled via Bindable notification |
| Resize end   | `Event::DragEnd`       | `onResizeEnd`   | State machine event               |
| Collapse     | `Event::CollapsePanel` | `onCollapse`    | State machine event               |
| Expand       | `Event::ExpandPanel`   | `onExpand`      | State machine event               |

**Gaps:** None.

### 5.4 Features

| Feature                 | ars-ui            | Ark UI             |
| ----------------------- | ----------------- | ------------------ |
| Pointer drag resize     | Yes               | Yes                |
| Keyboard resize         | Yes               | Yes                |
| Collapsible panels      | Yes               | Yes                |
| Collapse snap threshold | Yes               | --                 |
| Per-panel min/max       | Yes               | Yes                |
| RTL support             | Yes               | --                 |
| Pixel or percent units  | Yes               | --                 |
| Programmatic resize     | `resize_panel()`  | `resizePanel`      |
| Reset sizes             | `reset_sizes()`   | `resetSizes`       |
| Get sizes               | `ctx.sizes.get()` | `getSizes`         |
| Panel collapsed query   | State machine     | `isPanelCollapsed` |
| Storage persistence     | `storage_key`     | --                 |

**Gaps:** None.

### 5.5 Summary

- **Overall:** Full parity.
- **Divergences:** ars-ui models the resize handle as `Handle` (simpler naming); ars-ui adds `SizeUnit` for pixel-based layouts; ars-ui adds `storage_key` for persistence.
- **Recommended additions:** None.
