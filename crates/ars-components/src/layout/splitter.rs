//! Splitter component machine.
//!
//! `Splitter` owns framework-agnostic panel sizing, min/max enforcement,
//! collapse/expand behavior, keyboard resizing, drag intent, and ARIA/data
//! attributes. Framework adapters own live DOM handles, pointer capture,
//! measurement, and resize observer wiring.

use alloc::{format, string::String, vec, vec::Vec};
use core::fmt::{self, Debug};

use ars_core::{
    AriaAttr, AttrMap, Bindable, ComponentIds, ComponentMessages, ComponentPart, ConnectApi,
    CssProperty, Direction, Env, HtmlAttr, KeyboardKey, Locale, MessageFn, NoEffect, Orientation,
    TransitionPlan,
};

/// Message signature for splitter resize handle labels.
pub type ResizeHandleLabelFn = dyn Fn(usize, &Locale) -> String + Send + Sync;

/// Message signature for splitter panel size value text.
pub type PanelSizeTextFn = dyn Fn(f64, &Locale) -> String + Send + Sync;

/// The states of the splitter machine.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// No drag is in progress.
    #[default]
    Idle,

    /// User is dragging the handle between panels `handle_index` and
    /// `handle_index + 1`.
    Dragging {
        /// Zero-based handle index.
        handle_index: usize,
    },
}

/// Keyboard event mirror used by the splitter machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyboardEvent {
    /// Normalized keyboard key.
    pub key: KeyboardKey,

    /// Whether Shift was held.
    pub shift: bool,

    /// Whether Alt was held.
    pub alt: bool,

    /// Whether Control was held.
    pub ctrl: bool,

    /// Whether Meta was held.
    pub meta: bool,
}

/// The events of the splitter machine.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Pointer pressed on a handle at the split-axis coordinate.
    DragStart {
        /// Zero-based handle index.
        handle_index: usize,

        /// Client coordinate along the split axis.
        pos: f64,
    },

    /// Pointer moved while dragging.
    DragMove {
        /// Client coordinate along the split axis.
        pos: f64,
    },

    /// Pointer released or cancelled.
    DragEnd,

    /// Key pressed on a resize handle.
    KeyDown {
        /// Zero-based handle index.
        handle_index: usize,

        /// Normalized keyboard event data.
        event: KeyboardEvent,
    },

    /// Handle received focus.
    HandleFocus {
        /// Zero-based handle index.
        handle_index: usize,
    },

    /// Handle lost focus.
    HandleBlur,

    /// Programmatically collapse a panel.
    CollapsePanel {
        /// Zero-based panel index.
        panel_index: usize,
    },

    /// Programmatically expand a collapsed panel.
    ExpandPanel {
        /// Zero-based panel index.
        panel_index: usize,
    },

    /// Programmatically set all panel sizes.
    SetSizes {
        /// New sizes in the configured unit.
        sizes: Vec<f64>,
    },

    /// Synchronize context-backed props after parent prop changes.
    SyncProps {
        /// Latest props snapshot.
        props: Props,
    },
}

/// Definition for a single panel within the splitter.
#[derive(Clone, Debug, PartialEq)]
pub struct Panel {
    /// Stable semantic identifier for this panel.
    pub id: String,

    /// Minimum size in the configured unit.
    pub min_size: f64,

    /// Optional hard maximum size in the configured unit.
    pub max_size: Option<f64>,

    /// Initial size when no external value is provided.
    pub default_size: f64,

    /// Whether this panel can be collapsed.
    pub collapsible: bool,

    /// Size when collapsed.
    pub collapsed_size: f64,

    /// Fraction of `min_size` at which the panel snaps to collapsed size.
    pub collapse_threshold: f64,
}

impl Default for Panel {
    fn default() -> Self {
        Self {
            id: String::new(),
            min_size: 0.0,
            max_size: None,
            default_size: 100.0,
            collapsible: false,
            collapsed_size: 0.0,
            collapse_threshold: 0.5,
        }
    }
}

/// Unit for panel sizes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SizeUnit {
    /// Sizes are percentages.
    #[default]
    Percent,

    /// Sizes are CSS pixels.
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

    /// Text direction used for horizontal RTL keyboard behavior.
    pub dir: Direction,

    /// Unit for all panel sizes.
    pub size_unit: SizeUnit,

    /// Sizes captured at drag start.
    pub drag_start_sizes: Vec<f64>,

    /// Last expanded sizes remembered for collapsible panels.
    pub collapsed_restore_sizes: Vec<Option<f64>>,

    /// Pointer coordinate captured at drag start.
    pub drag_start_pos: f64,

    /// Keyboard resize step in the configured unit.
    pub keyboard_step: f64,

    /// Index of the focused handle.
    pub focused_handle: Option<usize>,

    /// Resolved locale for message formatting.
    pub locale: Locale,

    /// Resolved translatable messages.
    pub messages: Messages,

    /// Component IDs.
    pub ids: ComponentIds,

    /// CSS zoom or transform scale factor supplied by adapters.
    pub drag_scale_factor: f64,
}

impl Context {
    /// Creates splitter context from props and adapter-resolved environment.
    #[must_use]
    pub fn from_props(props: &Props, env: &Env, messages: &Messages) -> Self {
        let sizes = initial_sizes(props);
        let normalized_sizes = clamp_all(&sizes, &props.panels, &sizes);

        Self {
            sizes: props.sizes.as_ref().map_or_else(
                || Bindable::uncontrolled(normalized_sizes.clone()),
                |sizes| {
                    Bindable::controlled(clamp_all(sizes.get(), &props.panels, &normalized_sizes))
                },
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
            locale: env.locale.clone(),
            messages: messages.clone(),
            ids: ComponentIds::from_id(&props.id),
            drag_scale_factor: 1.0,
        }
    }
}

/// Props for the `Splitter` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
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

    /// Controlled panel sizes.
    pub sizes: Option<Bindable<Vec<f64>>>,

    /// Initial sizes for uncontrolled mode.
    pub default_sizes: Option<Vec<f64>>,

    /// Total pixel length when using pixel units and no default sizes.
    pub initial_total_px: Option<f64>,

    /// Keyboard resize step size.
    pub keyboard_step: Option<f64>,

    /// Key for adapter-owned persistence.
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

impl Props {
    /// Returns fresh splitter props with documented defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the component instance ID.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets panel definitions.
    #[must_use]
    pub fn panels(mut self, panels: Vec<Panel>) -> Self {
        self.panels = panels;
        self
    }

    /// Sets split orientation.
    #[must_use]
    pub const fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Sets text direction.
    #[must_use]
    pub const fn dir(mut self, dir: Direction) -> Self {
        self.dir = Some(dir);
        self
    }

    /// Sets size unit.
    #[must_use]
    pub const fn size_unit(mut self, size_unit: SizeUnit) -> Self {
        self.size_unit = size_unit;
        self
    }

    /// Sets controlled sizes.
    #[must_use]
    pub fn sizes(mut self, sizes: Bindable<Vec<f64>>) -> Self {
        self.sizes = Some(sizes);
        self
    }

    /// Sets uncontrolled default sizes.
    #[must_use]
    pub fn default_sizes(mut self, default_sizes: Vec<f64>) -> Self {
        self.default_sizes = Some(default_sizes);
        self
    }

    /// Sets total initial pixel length.
    #[must_use]
    pub const fn initial_total_px(mut self, initial_total_px: f64) -> Self {
        self.initial_total_px = Some(initial_total_px);
        self
    }

    /// Sets keyboard resize step.
    #[must_use]
    pub const fn keyboard_step(mut self, keyboard_step: f64) -> Self {
        self.keyboard_step = Some(keyboard_step);
        self
    }

    /// Sets adapter-owned persistence key.
    #[must_use]
    pub fn storage_key(mut self, storage_key: impl Into<String>) -> Self {
        self.storage_key = Some(storage_key.into());
        self
    }
}

/// Translatable strings used by `Splitter`.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Label for resize handles.
    pub resize_handle_label: MessageFn<ResizeHandleLabelFn>,

    /// Human-readable current panel size text.
    pub panel_size_text: MessageFn<PanelSizeTextFn>,

    /// Text announced when a panel is collapsed.
    pub panel_collapsed: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            resize_handle_label: MessageFn::new(|_index, _locale: &Locale| String::from("Resize")),
            panel_size_text: MessageFn::new(|value, _locale: &Locale| format!("{value:.0}%")),
            panel_collapsed: MessageFn::static_str("Collapsed"),
        }
    }
}

impl ComponentMessages for Messages {}

/// Compute new panel sizes after dragging a handle by `delta` units.
#[must_use]
pub fn compute_resize(
    sizes: &[f64],
    handle_index: usize,
    delta: f64,
    panels: &[Panel],
) -> Vec<f64> {
    if !delta.is_finite() || !valid_handle(handle_index, sizes.len(), panels.len()) {
        return sizes.to_vec();
    }

    let mut new_sizes = sizes.to_vec();

    let (left, right) = (handle_index, handle_index + 1);

    let left_growth = panels[left]
        .max_size
        .map_or(f64::INFINITY, |max| max - new_sizes[left]);

    let right_growth = panels[right]
        .max_size
        .map_or(f64::INFINITY, |max| max - new_sizes[right]);

    let right_shrink = new_sizes[right] - effective_min(&panels[right]);
    let left_shrink = new_sizes[left] - effective_min(&panels[left]);

    let (left_delta, right_delta) = if delta > 0.0 {
        let applied = delta.min(left_growth).min(right_shrink).max(0.0);

        (applied, -applied)
    } else {
        let applied = (-delta).min(left_shrink).min(right_growth).max(0.0);

        (-applied, applied)
    };

    new_sizes[left] += left_delta;
    new_sizes[right] += right_delta;

    snap_if_collapsible(&mut new_sizes, left, panels, delta < 0.0);
    snap_if_collapsible(&mut new_sizes, right, panels, delta > 0.0);

    new_sizes
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

fn keyboard_step_for(props: &Props) -> f64 {
    let default = match props.size_unit {
        SizeUnit::Percent => 10.0,
        SizeUnit::Pixels => 20.0,
    };

    finite_positive(props.keyboard_step.unwrap_or(default), default)
}

fn finite_positive(value: f64, fallback: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}

const fn valid_handle(handle_index: usize, sizes_len: usize, panels_len: usize) -> bool {
    if let Some(next) = handle_index.checked_add(1) {
        next < sizes_len && next < panels_len
    } else {
        false
    }
}

const fn effective_min(panel: &Panel) -> f64 {
    if panel.collapsible {
        panel.collapsed_size
    } else {
        panel.min_size
    }
}

fn snap_if_collapsible(
    sizes: &mut [f64],
    index: usize,
    panels: &[Panel],
    moving_toward_collapse: bool,
) {
    let panel = &panels[index];
    if !panel.collapsible {
        return;
    }

    let threshold = panel.min_size * panel.collapse_threshold
        + panel.collapsed_size * (1.0 - panel.collapse_threshold);

    if moving_toward_collapse && sizes[index] < threshold {
        sizes[index] = panel.collapsed_size;
    } else if sizes[index] < panel.min_size && sizes[index] > panel.collapsed_size {
        sizes[index] = panel.min_size;
    }
}

fn clamp_all(sizes: &[f64], panels: &[Panel], current: &[f64]) -> Vec<f64> {
    let normalized = normalize_sizes(sizes, panels, current);

    normalized
        .iter()
        .zip(panels.iter())
        .map(|(&size, panel)| {
            size.max(effective_min(panel))
                .min(panel.max_size.unwrap_or(f64::INFINITY))
        })
        .collect()
}

fn normalize_sizes(sizes: &[f64], panels: &[Panel], current: &[f64]) -> Vec<f64> {
    panels
        .iter()
        .enumerate()
        .map(|(index, panel)| {
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
        })
        .collect()
}

fn collapse_panel(sizes: &mut [f64], index: usize, panels: &[Panel]) {
    if index >= sizes.len() || index >= panels.len() || !panels[index].collapsible {
        return;
    }

    let collapsed_size = panels[index].collapsed_size;

    let freed = (sizes[index] - collapsed_size).max(0.0);

    let recipient = if index + 1 < sizes.len() {
        Some(index + 1)
    } else if index > 0 {
        Some(index - 1)
    } else {
        None
    };

    let transferable = recipient.map_or(freed, |recipient| {
        panels
            .get(recipient)
            .and_then(|panel| panel.max_size)
            .map_or(freed, |max_size| {
                (max_size - sizes[recipient]).max(0.0).min(freed)
            })
    });

    sizes[index] = collapsed_size;

    if let Some(recipient) = recipient {
        sizes[recipient] += transferable;
    }
}

fn expand_panel(sizes: &mut [f64], index: usize, panels: &[Panel], restore_size: Option<f64>) {
    if index >= sizes.len() || index >= panels.len() {
        return;
    }

    let panel = &panels[index];

    if !panel.collapsible {
        return;
    }

    if sizes[index] > panel.collapsed_size {
        return;
    }

    let target_size = restore_size
        .filter(|size| size.is_finite())
        .unwrap_or(panel.default_size);

    let need = (target_size - sizes[index]).max(0.0);

    let donor = if index + 1 < sizes.len() {
        Some(index + 1)
    } else if index > 0 {
        Some(index - 1)
    } else {
        None
    };

    if let Some(donor) = donor {
        let available = (sizes[donor] - effective_min(&panels[donor])).max(0.0);

        let actual = need.min(available);

        sizes[index] += actual;
        sizes[donor] -= actual;
    }
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
    if sizes.iter().all(|size| size.is_finite()) && sizes.len() == ctx.panels.len() {
        ctx.sizes.set(sizes.clone());

        if ctx.sizes.is_controlled() {
            ctx.sizes.sync_controlled(Some(sizes));
        }
    }
}

fn handle_keyboard(ctx: &mut Context, handle_index: usize, event: &KeyboardEvent) {
    if !valid_handle(handle_index, ctx.sizes.get().len(), ctx.panels.len()) {
        return;
    }

    let step = if event.shift {
        ctx.keyboard_step * 5.0
    } else {
        ctx.keyboard_step
    };

    let is_rtl = ctx.dir == Direction::Rtl;

    match (&event.key, ctx.orientation) {
        (KeyboardKey::ArrowRight, Orientation::Horizontal)
        | (KeyboardKey::ArrowDown, Orientation::Vertical) => {
            let delta = rtl_adjusted_delta(step, ctx.orientation, is_rtl);
            let next = compute_resize(ctx.sizes.get(), handle_index, delta, &ctx.panels);

            commit_sizes(ctx, next);
        }

        (KeyboardKey::ArrowLeft, Orientation::Horizontal)
        | (KeyboardKey::ArrowUp, Orientation::Vertical) => {
            let delta = rtl_adjusted_delta(-step, ctx.orientation, is_rtl);
            let next = compute_resize(ctx.sizes.get(), handle_index, delta, &ctx.panels);

            commit_sizes(ctx, next);
        }

        (KeyboardKey::Home, _) => {
            let sizes = ctx.sizes.get().clone();
            let delta = effective_min(&ctx.panels[handle_index]) - sizes[handle_index];

            let next = compute_resize(&sizes, handle_index, delta, &ctx.panels);

            commit_sizes(ctx, next);
        }

        (KeyboardKey::End, _) => {
            let sizes = ctx.sizes.get().clone();
            let total = sizes.iter().sum::<f64>();
            let max = ctx.panels[handle_index].max_size.unwrap_or(total);
            let delta = max - sizes[handle_index];

            let next = compute_resize(&sizes, handle_index, delta, &ctx.panels);

            commit_sizes(ctx, next);
        }

        (KeyboardKey::Enter | KeyboardKey::Space, _) => {
            let mut sizes = ctx.sizes.get().clone();
            let panel = &ctx.panels[handle_index];

            if panel.collapsible {
                if sizes[handle_index] <= panel.collapsed_size {
                    let restore_size = ctx
                        .collapsed_restore_sizes
                        .get(handle_index)
                        .copied()
                        .flatten();

                    expand_panel(&mut sizes, handle_index, &ctx.panels, restore_size);

                    if let Some(restore_size) = ctx.collapsed_restore_sizes.get_mut(handle_index) {
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
    if is_rtl && orientation == Orientation::Horizontal {
        -delta
    } else {
        delta
    }
}

/// The `Splitter` state machine.
#[derive(Debug)]
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Effect = NoEffect;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (State, Context) {
        (State::Idle, Context::from_props(props, env, messages))
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            (State::Idle, Event::DragStart { handle_index, pos })
                if pos.is_finite()
                    && valid_handle(*handle_index, ctx.sizes.get().len(), ctx.panels.len()) =>
            {
                let handle_index = *handle_index;
                let pos = *pos;
                Some(TransitionPlan::to(State::Dragging { handle_index }).apply(
                    move |ctx: &mut Context| {
                        ctx.drag_start_sizes = ctx.sizes.get().clone();
                        ctx.drag_start_pos = pos;
                    },
                ))
            }

            (
                State::Idle,
                Event::KeyDown {
                    handle_index,
                    event,
                },
            ) => {
                let handle_index = *handle_index;
                let event = event.clone();

                if valid_handle(handle_index, ctx.sizes.get().len(), ctx.panels.len()) {
                    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                        handle_keyboard(ctx, handle_index, &event);
                    }))
                } else {
                    Some(TransitionPlan::new())
                }
            }

            (State::Idle, Event::HandleFocus { handle_index })
                if valid_handle(*handle_index, ctx.sizes.get().len(), ctx.panels.len()) =>
            {
                let handle_index = *handle_index;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.focused_handle = Some(handle_index);
                }))
            }

            (State::Idle, Event::HandleBlur) => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.focused_handle = None;
                }))
            }

            (State::Idle, Event::CollapsePanel { panel_index })
                if *panel_index < ctx.sizes.get().len() && *panel_index < ctx.panels.len() =>
            {
                let panel_index = *panel_index;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let mut sizes = ctx.sizes.get().clone();

                    remember_collapse_size(ctx, panel_index);
                    collapse_panel(&mut sizes, panel_index, &ctx.panels);

                    commit_sizes(ctx, sizes);
                }))
            }

            (State::Idle, Event::ExpandPanel { panel_index })
                if *panel_index < ctx.sizes.get().len() && *panel_index < ctx.panels.len() =>
            {
                let panel_index = *panel_index;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let mut sizes = ctx.sizes.get().clone();
                    let restore_size = ctx
                        .collapsed_restore_sizes
                        .get(panel_index)
                        .copied()
                        .flatten();

                    expand_panel(&mut sizes, panel_index, &ctx.panels, restore_size);

                    if let Some(restore_size) = ctx.collapsed_restore_sizes.get_mut(panel_index) {
                        *restore_size = None;
                    }

                    commit_sizes(ctx, sizes);
                }))
            }

            (State::Idle, Event::CollapsePanel { .. } | Event::ExpandPanel { .. }) => {
                Some(TransitionPlan::new())
            }

            (State::Idle, Event::SetSizes { sizes }) => {
                let sizes = clamp_all(sizes, &ctx.panels, ctx.sizes.get());
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    commit_sizes(ctx, sizes);
                }))
            }

            (State::Dragging { handle_index }, Event::DragMove { pos }) if pos.is_finite() => {
                let handle_index = *handle_index;
                let pos = *pos;
                let start_pos = ctx.drag_start_pos;
                let start_sizes = ctx.drag_start_sizes.clone();
                let panels = ctx.panels.clone();
                let scale = finite_positive(ctx.drag_scale_factor, 1.0);
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let delta = (pos - start_pos) / scale;
                    let next = compute_resize(&start_sizes, handle_index, delta, &panels);

                    commit_sizes(ctx, next);
                }))
            }

            (State::Dragging { .. }, Event::DragEnd) => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                    ctx.drag_start_sizes.clear();
                    ctx.drag_start_pos = 0.0;
                }))
            }

            (State::Dragging { handle_index }, Event::KeyDown { event, .. }) => {
                let handle_index = *handle_index;
                match event.key {
                    KeyboardKey::Escape => {
                        let pre_drag = ctx.drag_start_sizes.clone();
                        Some(
                            TransitionPlan::to(State::Idle).apply(move |ctx: &mut Context| {
                                commit_sizes(ctx, pre_drag);

                                ctx.drag_start_sizes.clear();
                                ctx.drag_start_pos = 0.0;
                            }),
                        )
                    }

                    KeyboardKey::ArrowLeft
                    | KeyboardKey::ArrowRight
                    | KeyboardKey::ArrowUp
                    | KeyboardKey::ArrowDown
                    | KeyboardKey::Home
                    | KeyboardKey::End
                    | KeyboardKey::Enter
                    | KeyboardKey::Space => {
                        let event = event.clone();
                        Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                            handle_keyboard(ctx, handle_index, &event);
                        }))
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

                Some(plan.apply(move |ctx: &mut Context| {
                    let current = ctx.sizes.get().clone();

                    ctx.panels = props.panels.clone();
                    ctx.orientation = props.orientation;
                    ctx.dir = props.dir.unwrap_or(Direction::Ltr);
                    ctx.size_unit = props.size_unit;
                    ctx.keyboard_step = keyboard_step_for(&props);
                    ctx.collapsed_restore_sizes.resize(ctx.panels.len(), None);

                    let normalized = clamp_all(ctx.sizes.get(), &ctx.panels, &current);

                    ctx.sizes.sync_controlled(
                        props
                            .sizes
                            .clone()
                            .map(|sizes| clamp_all(sizes.get(), &ctx.panels, &normalized)),
                    );

                    if !ctx.sizes.is_controlled() {
                        ctx.sizes.set(normalized);
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
        Api {
            state,
            ctx,
            props,
            send,
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        assert_eq!(
            old.id, new.id,
            "Splitter id cannot change after initialization"
        );

        if old == new {
            Vec::new()
        } else {
            vec![Event::SyncProps { props: new.clone() }]
        }
    }
}

/// Structural parts exposed by the `Splitter` connect API.
#[derive(ComponentPart)]
#[scope = "splitter"]
pub enum Part {
    /// The root splitter container.
    Root,

    /// A splitter panel by index.
    Panel {
        /// Zero-based panel index.
        index: usize,
    },

    /// A resize handle by index.
    Handle {
        /// Zero-based handle index.
        index: usize,
    },
}

/// Connected API for the `Splitter` component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Api")
            .field("state", self.state)
            .field("ctx", self.ctx)
            .field("props", self.props)
            .finish_non_exhaustive()
    }
}

impl Api<'_> {
    /// Returns root splitter attributes.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(
                HtmlAttr::Data("ars-orientation"),
                match self.ctx.orientation {
                    Orientation::Horizontal => "horizontal",
                    Orientation::Vertical => "vertical",
                },
            )
            .set(
                HtmlAttr::Data("ars-state"),
                match self.state {
                    State::Idle => "idle",
                    State::Dragging { .. } => "dragging",
                },
            );

        attrs
    }

    /// Returns panel attributes for `index`.
    #[must_use]
    pub fn panel_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = (Part::Panel { index }).data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        let Some(panel) = self.ctx.panels.get(index) else {
            return attrs;
        };

        let Some(size) = self.ctx.sizes.get().get(index).copied() else {
            return attrs;
        };

        let unit = match self.ctx.size_unit {
            SizeUnit::Percent => "%",
            SizeUnit::Pixels => "px",
        };

        let collapsed = panel.collapsible && size <= panel.collapsed_size;

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.item("panel", &index))
            .set(HtmlAttr::Data("ars-panel-id"), panel.id.as_str());

        if collapsed {
            attrs.set_bool(HtmlAttr::Data("ars-collapsed"), true);
        }

        attrs.set_style(
            match self.ctx.orientation {
                Orientation::Horizontal => CssProperty::Width,
                Orientation::Vertical => CssProperty::Height,
            },
            format!("{size}{unit}"),
        );

        attrs
    }

    /// Returns resize handle attributes for `handle_index`.
    #[must_use]
    pub fn handle_attrs(&self, handle_index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = (Part::Handle {
            index: handle_index,
        })
        .data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-handle-index"), handle_index.to_string());

        if !valid_handle(
            handle_index,
            self.ctx.sizes.get().len(),
            self.ctx.panels.len(),
        ) {
            return attrs;
        }

        let is_focused = self.ctx.focused_handle == Some(handle_index);
        let is_dragging = matches!(self.state, State::Dragging { handle_index: active } if *active == handle_index);

        attrs
            .set(
                HtmlAttr::Data("ars-state"),
                if is_dragging {
                    "dragging"
                } else if is_focused {
                    "focus"
                } else {
                    "idle"
                },
            )
            .set(HtmlAttr::Role, "separator")
            .set(
                HtmlAttr::Aria(AriaAttr::Orientation),
                match self.ctx.orientation {
                    Orientation::Horizontal => "vertical",
                    Orientation::Vertical => "horizontal",
                },
            );

        let left = handle_index;
        let right = handle_index + 1;

        let sizes = self.ctx.sizes.get();

        let total = sizes.iter().copied().sum::<f64>().max(1.0);

        let to_percent = |value: f64| {
            if self.ctx.size_unit == SizeUnit::Percent {
                value
            } else {
                value / total * 100.0
            }
        };

        let value_now = to_percent(sizes[left]);

        let value_min = to_percent(effective_min(&self.ctx.panels[left]));

        let fallback_max = match self.ctx.size_unit {
            SizeUnit::Percent => 100.0 - effective_min(&self.ctx.panels[right]),
            SizeUnit::Pixels => total - effective_min(&self.ctx.panels[right]),
        };

        let value_max = to_percent(
            self.ctx.panels[left]
                .max_size
                .unwrap_or(fallback_max)
                .min(fallback_max),
        );

        let collapsed = self.ctx.panels[left].collapsible
            && sizes[left] <= self.ctx.panels[left].collapsed_size;

        attrs
            .set(
                HtmlAttr::Aria(AriaAttr::ValueNow),
                (value_now.round() as i64).to_string(),
            )
            .set(
                HtmlAttr::Aria(AriaAttr::ValueMin),
                (value_min.round() as i64).to_string(),
            )
            .set(
                HtmlAttr::Aria(AriaAttr::ValueMax),
                (value_max.round() as i64).to_string(),
            )
            .set(
                HtmlAttr::Aria(AriaAttr::ValueText),
                if collapsed {
                    (self.ctx.messages.panel_collapsed)(&self.ctx.locale)
                } else {
                    (self.ctx.messages.panel_size_text)(value_now, &self.ctx.locale)
                },
            )
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.resize_handle_label)(handle_index, &self.ctx.locale),
            )
            .set(
                HtmlAttr::Aria(AriaAttr::Controls),
                self.ctx.ids.item("panel", &left),
            )
            .set(HtmlAttr::TabIndex, "0");

        attrs
    }

    /// Programmatically collapses a panel by index.
    pub fn collapse_panel(&self, panel_index: usize) {
        (self.send)(Event::CollapsePanel { panel_index });
    }

    /// Programmatically expands a panel by index.
    pub fn expand_panel(&self, panel_index: usize) {
        (self.send)(Event::ExpandPanel { panel_index });
    }

    /// Programmatically resizes a panel by index.
    pub fn resize_panel(&self, panel_index: usize, size: f64) {
        let mut sizes = self.ctx.sizes.get().clone();

        if panel_index < sizes.len() {
            sizes[panel_index] = size;

            (self.send)(Event::SetSizes { sizes });
        }
    }

    /// Resets all panels to their default sizes.
    pub fn reset_sizes(&self) {
        let sizes = self
            .ctx
            .panels
            .iter()
            .map(|panel| panel.default_size)
            .collect();

        (self.send)(Event::SetSizes { sizes });
    }

    /// Dispatches handle pointer-down intent.
    pub fn on_handle_pointerdown(&self, handle_index: usize, pos: f64) {
        (self.send)(Event::DragStart { handle_index, pos });
    }

    /// Dispatches handle pointer-move intent.
    pub fn on_handle_pointermove(&self, pos: f64) {
        (self.send)(Event::DragMove { pos });
    }

    /// Dispatches handle pointer-up intent.
    pub fn on_handle_pointerup(&self) {
        (self.send)(Event::DragEnd);
    }

    /// Dispatches handle keydown intent.
    pub fn on_handle_keydown(&self, handle_index: usize, event: KeyboardEvent) {
        (self.send)(Event::KeyDown {
            handle_index,
            event,
        });
    }

    /// Dispatches handle focus intent.
    pub fn on_handle_focus(&self, handle_index: usize) {
        (self.send)(Event::HandleFocus { handle_index });
    }

    /// Dispatches handle blur intent.
    pub fn on_handle_blur(&self) {
        (self.send)(Event::HandleBlur);
    }

    /// Returns the immutable props used to construct this API.
    #[must_use]
    pub const fn props(&self) -> &Props {
        self.props
    }
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

#[cfg(test)]
mod tests {
    use alloc::{
        format,
        string::{String, ToString},
        vec,
        vec::Vec,
    };
    use core::cell::RefCell;

    use ars_core::{
        AriaAttr, AttrMap, Bindable, ConnectApi, CssProperty, Direction, Env, HtmlAttr,
        KeyboardKey, Orientation, Service,
    };
    use insta::assert_snapshot;

    use super::*;

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    fn key(key: KeyboardKey) -> KeyboardEvent {
        KeyboardEvent {
            key,
            shift: false,
            alt: false,
            ctrl: false,
            meta: false,
        }
    }

    fn shift_key(key: KeyboardKey) -> KeyboardEvent {
        KeyboardEvent {
            shift: true,
            ..self::key(key)
        }
    }

    fn panel(id: &str, default_size: f64) -> Panel {
        Panel {
            id: id.to_string(),
            min_size: 10.0,
            max_size: Some(90.0),
            default_size,
            collapsible: false,
            collapsed_size: 0.0,
            collapse_threshold: 0.5,
        }
    }

    fn collapsible_panel(id: &str, default_size: f64) -> Panel {
        Panel {
            collapsible: true,
            collapsed_size: 0.0,
            collapse_threshold: 0.5,
            ..panel(id, default_size)
        }
    }

    fn props() -> Props {
        Props::new().id("split").panels(vec![
            panel("left", 40.0),
            panel("middle", 30.0),
            panel("right", 30.0),
        ])
    }

    fn two_panel_props() -> Props {
        Props::new()
            .id("split")
            .panels(vec![panel("left", 50.0), panel("right", 50.0)])
    }

    fn service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    #[test]
    fn props_new_returns_default_values() {
        assert_eq!(Props::new(), Props::default());
    }

    #[test]
    fn panel_default_matches_spec() {
        assert_eq!(
            Panel::default(),
            Panel {
                id: String::new(),
                min_size: 0.0,
                max_size: None,
                default_size: 100.0,
                collapsible: false,
                collapsed_size: 0.0,
                collapse_threshold: 0.5,
            }
        );
    }

    #[test]
    fn props_builder_chain_applies_each_setter() {
        let sizes = vec![25.0, 75.0];
        let props = Props::new()
            .id("splitter")
            .panels(vec![panel("a", 25.0), panel("b", 75.0)])
            .orientation(Orientation::Vertical)
            .dir(Direction::Rtl)
            .size_unit(SizeUnit::Pixels)
            .sizes(Bindable::controlled(sizes.clone()))
            .default_sizes(vec![20.0, 80.0])
            .initial_total_px(800.0)
            .keyboard_step(32.0)
            .storage_key("layout.main");

        assert_eq!(props.id, "splitter");
        assert_eq!(props.panels.len(), 2);
        assert_eq!(props.orientation, Orientation::Vertical);
        assert_eq!(props.dir, Some(Direction::Rtl));
        assert_eq!(props.size_unit, SizeUnit::Pixels);
        assert_eq!(props.sizes.as_ref().map(Bindable::get), Some(&sizes));
        assert_eq!(props.default_sizes, Some(vec![20.0, 80.0]));
        assert_eq!(props.initial_total_px, Some(800.0));
        assert_eq!(props.keyboard_step, Some(32.0));
        assert_eq!(props.storage_key, Some("layout.main".into()));
    }

    #[test]
    fn initializes_percent_sizes_evenly_without_defaults() {
        let service = service(props());

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().sizes.get().len(), 3);
        assert!((service.context().sizes.get()[0] - (100.0 / 3.0)).abs() < 0.0001);
        assert_eq!(service.context().orientation, Orientation::Horizontal);
        assert_eq!(service.context().dir, Direction::Ltr);
        assert_eq!(service.context().size_unit, SizeUnit::Percent);
        assert_eq!(service.context().keyboard_step, 10.0);
        assert_eq!(service.context().ids.id(), "split");
    }

    #[test]
    fn initializes_pixel_sizes_from_initial_total_px() {
        let mut left = panel("left", 300.0);
        let mut middle = panel("middle", 300.0);
        let mut right = panel("right", 300.0);

        left.max_size = None;
        middle.max_size = None;
        right.max_size = None;

        let service = service(
            Props::new()
                .id("split")
                .panels(vec![left, middle, right])
                .size_unit(SizeUnit::Pixels)
                .initial_total_px(900.0),
        );

        assert_eq!(service.context().sizes.get(), &vec![300.0, 300.0, 300.0]);
        assert_eq!(service.context().keyboard_step, 20.0);
    }

    #[test]
    fn default_sizes_override_even_distribution() {
        let service = service(props().default_sizes(vec![20.0, 30.0, 50.0]));

        assert_eq!(service.context().sizes.get(), &vec![20.0, 30.0, 50.0]);
    }

    #[test]
    fn uncontrolled_initial_sizes_are_clamped_to_panel_constraints() {
        let service = service(two_panel_props().default_sizes(vec![5.0, 150.0]));

        assert_eq!(service.context().sizes.get(), &vec![10.0, 90.0]);
    }

    #[test]
    fn initializes_controlled_sizes_with_normalized_constraints() {
        let service = service(two_panel_props().sizes(Bindable::controlled(vec![f64::NAN, 150.0])));

        assert_eq!(service.context().sizes.get(), &vec![50.0, 90.0]);
        assert!(service.context().sizes.is_controlled());
    }

    #[test]
    fn drag_resize_handle_resizes_adjacent_panels() {
        let mut service = service(two_panel_props().default_sizes(vec![50.0, 50.0]));

        let start = service.send(Event::DragStart {
            handle_index: 0,
            pos: 100.0,
        });

        assert!(start.state_changed);
        assert_eq!(service.state(), &State::Dragging { handle_index: 0 });

        let resize = service.send(Event::DragMove { pos: 120.0 });

        assert!(!resize.state_changed);
        assert_eq!(service.context().sizes.get(), &vec![70.0, 30.0]);

        let end = service.send(Event::DragEnd);

        assert!(end.state_changed);
        assert_eq!(service.state(), &State::Idle);
        assert!(service.context().drag_start_sizes.is_empty());
    }

    #[test]
    fn drag_resize_respects_min_and_max_constraints() {
        let mut service = service(two_panel_props().default_sizes(vec![50.0, 50.0]));

        drop(service.send(Event::DragStart {
            handle_index: 0,
            pos: 0.0,
        }));

        drop(service.send(Event::DragMove { pos: 1000.0 }));

        assert_eq!(service.context().sizes.get(), &vec![90.0, 10.0]);
    }

    #[test]
    fn drag_resize_applies_scale_factor() {
        let mut service = service(two_panel_props().default_sizes(vec![50.0, 50.0]));

        service.context_mut().drag_scale_factor = 2.0;

        drop(service.send(Event::DragStart {
            handle_index: 0,
            pos: 0.0,
        }));
        drop(service.send(Event::DragMove { pos: 20.0 }));

        assert_eq!(service.context().sizes.get(), &vec![60.0, 40.0]);
    }

    #[test]
    fn controlled_sizes_commit_to_observable_bindable_value() {
        let mut service = service(two_panel_props().sizes(Bindable::controlled(vec![50.0, 50.0])));

        drop(service.send(Event::KeyDown {
            handle_index: 0,
            event: key(KeyboardKey::ArrowRight),
        }));

        assert_eq!(service.context().sizes.get(), &vec![60.0, 40.0]);
        assert!(service.context().sizes.is_controlled());
    }

    #[test]
    fn collapsible_panel_snaps_to_collapsed_size() {
        let mut unconstrained_right = panel("right", 80.0);

        unconstrained_right.max_size = None;

        let sizes = compute_resize(
            &[20.0, 80.0],
            0,
            -16.0,
            &[collapsible_panel("left", 20.0), unconstrained_right],
        );

        assert_eq!(sizes, vec![0.0, 96.0]);
    }

    #[test]
    fn right_collapsible_panel_snaps_to_collapsed_size() {
        let mut unconstrained_left = panel("left", 80.0);

        unconstrained_left.max_size = None;

        let sizes = compute_resize(
            &[80.0, 20.0],
            0,
            16.0,
            &[unconstrained_left, collapsible_panel("right", 20.0)],
        );

        assert_eq!(sizes, vec![96.0, 0.0]);
    }

    #[test]
    fn collapsible_snap_uses_weighted_threshold_and_strict_boundary() {
        let mut collapsible = collapsible_panel("left", 20.0);

        collapsible.min_size = 10.0;
        collapsible.collapsed_size = 2.0;
        collapsible.collapse_threshold = 0.25;

        let mut right = panel("right", 80.0);

        right.max_size = None;

        let below_threshold = compute_resize(
            &[20.0, 80.0],
            0,
            -17.0,
            &[collapsible.clone(), right.clone()],
        );
        assert_eq!(below_threshold, vec![2.0, 97.0]);

        let at_threshold = compute_resize(&[20.0, 80.0], 0, -16.0, &[collapsible, right]);

        assert_eq!(at_threshold, vec![10.0, 96.0]);
    }

    #[test]
    fn compute_resize_rejects_non_finite_delta() {
        let panels = [panel("left", 50.0), panel("right", 50.0)];

        assert_eq!(
            compute_resize(&[50.0, 50.0], 0, f64::NAN, &panels),
            vec![50.0, 50.0]
        );
        assert_eq!(
            compute_resize(&[50.0, 50.0], 0, f64::INFINITY, &panels),
            vec![50.0, 50.0]
        );
    }

    #[test]
    fn compute_resize_respects_left_max_before_neighbor_minimum() {
        let left = Panel {
            max_size: Some(60.0),
            ..panel("left", 50.0)
        };

        let right = Panel {
            min_size: 0.0,
            max_size: None,
            ..panel("right", 50.0)
        };

        let sizes = compute_resize(&[50.0, 50.0], 0, 30.0, &[left, right]);

        assert_eq!(sizes, vec![60.0, 40.0]);
    }

    #[test]
    fn compute_resize_respects_right_minimum_before_left_growth() {
        let left = Panel {
            max_size: None,
            ..panel("left", 50.0)
        };

        let right = Panel {
            min_size: 40.0,
            max_size: None,
            ..panel("right", 50.0)
        };

        let sizes = compute_resize(&[50.0, 50.0], 0, 30.0, &[left, right]);

        assert_eq!(sizes, vec![60.0, 40.0]);
    }

    #[test]
    fn compute_resize_respects_left_minimum_before_right_growth() {
        let left = Panel {
            min_size: 40.0,
            max_size: None,
            ..panel("left", 50.0)
        };

        let right = Panel {
            max_size: None,
            ..panel("right", 50.0)
        };

        let sizes = compute_resize(&[50.0, 50.0], 0, -30.0, &[left, right]);

        assert_eq!(sizes, vec![40.0, 60.0]);
    }

    #[test]
    fn compute_resize_respects_right_maximum_before_left_shrink() {
        let left = Panel {
            min_size: 0.0,
            max_size: None,
            ..panel("left", 50.0)
        };

        let right = Panel {
            max_size: Some(60.0),
            ..panel("right", 50.0)
        };

        let sizes = compute_resize(&[50.0, 50.0], 0, -30.0, &[left, right]);

        assert_eq!(sizes, vec![40.0, 60.0]);
    }

    #[test]
    fn compute_resize_zero_delta_does_not_snap_collapsible_panel() {
        let mut left = collapsible_panel("left", 0.25);

        left.collapse_threshold = 0.5;

        let right = Panel {
            max_size: None,
            ..panel("right", 99.75)
        };

        let sizes = compute_resize(&[0.25, 99.75], 0, 0.0, &[left, right]);

        assert_eq!(sizes, vec![10.0, 99.75]);
    }

    #[test]
    fn compute_resize_zero_delta_does_not_snap_right_collapsible_panel() {
        let left = Panel {
            max_size: None,
            ..panel("left", 99.75)
        };

        let mut right = collapsible_panel("right", 0.25);

        right.collapse_threshold = 0.5;

        let sizes = compute_resize(&[99.75, 0.25], 0, 0.0, &[left, right]);

        assert_eq!(sizes, vec![99.75, 10.0]);
    }

    #[test]
    fn zero_delta_keeps_above_min_collapsible_size() {
        let left = Panel {
            max_size: None,
            ..panel("left", 88.0)
        };

        let sizes = compute_resize(
            &[12.0, 88.0],
            0,
            0.0,
            &[collapsible_panel("left", 12.0), left],
        );

        assert_eq!(sizes, vec![12.0, 88.0]);
    }

    #[test]
    fn compute_resize_ignores_invalid_handle_boundaries() {
        assert_eq!(
            compute_resize(
                &[50.0, 50.0],
                1,
                10.0,
                &[panel("left", 50.0), panel("right", 50.0)]
            ),
            vec![50.0, 50.0]
        );
        assert_eq!(
            compute_resize(&[50.0, 50.0], 0, 10.0, &[panel("left", 50.0)]),
            vec![50.0, 50.0]
        );
        assert_eq!(
            compute_resize(
                &[50.0],
                0,
                10.0,
                &[panel("left", 50.0), panel("right", 50.0)]
            ),
            vec![50.0]
        );
    }

    #[test]
    fn collapse_and_expand_panel_move_size_to_neighbor() {
        let mut right = panel("right", 60.0);

        right.max_size = None;

        let mut service = service(
            Props::new()
                .id("split")
                .panels(vec![collapsible_panel("left", 40.0), right])
                .default_sizes(vec![40.0, 60.0]),
        );

        drop(service.send(Event::CollapsePanel { panel_index: 0 }));

        assert_eq!(service.context().sizes.get(), &vec![0.0, 100.0]);

        drop(service.send(Event::ExpandPanel { panel_index: 0 }));

        assert_eq!(service.context().sizes.get(), &vec![40.0, 60.0]);
    }

    #[test]
    fn collapse_panel_respects_neighbor_max_size() {
        let mut right = panel("right", 60.0);

        right.max_size = Some(65.0);

        let mut service = service(
            Props::new()
                .id("split")
                .panels(vec![collapsible_panel("left", 40.0), right])
                .default_sizes(vec![40.0, 60.0]),
        );

        drop(service.send(Event::CollapsePanel { panel_index: 0 }));

        assert_eq!(service.context().sizes.get(), &vec![0.0, 65.0]);
    }

    #[test]
    fn collapse_panel_ignores_invalid_or_non_collapsible_panels() {
        let mut invalid = service(
            Props::new()
                .id("split")
                .panels(vec![collapsible_panel("left", 40.0), panel("right", 60.0)])
                .default_sizes(vec![40.0, 60.0]),
        );

        drop(invalid.send(Event::CollapsePanel { panel_index: 2 }));

        assert_eq!(invalid.context().sizes.get(), &vec![40.0, 60.0]);

        let mut non_collapsible = service(two_panel_props().default_sizes(vec![50.0, 50.0]));

        drop(non_collapsible.send(Event::CollapsePanel { panel_index: 0 }));

        assert_eq!(non_collapsible.context().sizes.get(), &vec![50.0, 50.0]);
    }

    #[test]
    fn collapse_panel_ignores_index_outside_size_slice() {
        let mut sizes = vec![40.0];

        collapse_panel(
            &mut sizes,
            1,
            &[panel("left", 40.0), collapsible_panel("right", 60.0)],
        );

        assert_eq!(sizes, vec![40.0]);
    }

    #[test]
    fn collapse_panel_transfers_only_freed_size() {
        let mut right = panel("right", 60.0);

        right.max_size = None;

        let mut service = service(
            Props::new()
                .id("split")
                .panels(vec![
                    Panel {
                        collapsed_size: 5.0,
                        ..collapsible_panel("left", 40.0)
                    },
                    right,
                ])
                .default_sizes(vec![40.0, 60.0]),
        );

        drop(service.send(Event::CollapsePanel { panel_index: 0 }));

        assert_eq!(service.context().sizes.get(), &vec![5.0, 95.0]);
    }

    #[test]
    fn collapse_last_panel_transfers_freed_size_to_previous_neighbor() {
        let mut left = panel("left", 60.0);

        left.max_size = None;

        let mut service = service(
            Props::new()
                .id("split")
                .panels(vec![left, collapsible_panel("right", 40.0)])
                .default_sizes(vec![60.0, 40.0]),
        );

        drop(service.send(Event::CollapsePanel { panel_index: 1 }));

        assert_eq!(service.context().sizes.get(), &vec![100.0, 0.0]);
    }

    #[test]
    fn collapse_single_panel_has_no_donor_neighbor() {
        let mut sizes = vec![40.0];

        collapse_panel(&mut sizes, 0, &[collapsible_panel("only", 40.0)]);

        assert_eq!(sizes, vec![0.0]);
    }

    #[test]
    fn expand_panel_ignores_invalid_or_non_collapsed_panels() {
        let mut invalid = vec![0.0];

        expand_panel(
            &mut invalid,
            1,
            &[
                collapsible_panel("left", 40.0),
                collapsible_panel("right", 60.0),
            ],
            None,
        );

        assert_eq!(invalid, vec![0.0]);

        let mut expanded = vec![20.0, 80.0];

        expand_panel(
            &mut expanded,
            0,
            &[collapsible_panel("left", 40.0), panel("right", 60.0)],
            None,
        );

        assert_eq!(expanded, vec![20.0, 80.0]);
    }

    #[test]
    fn expand_panel_uses_nonzero_collapsed_size() {
        let mut sizes = vec![5.0, 95.0];

        expand_panel(
            &mut sizes,
            0,
            &[
                Panel {
                    collapsed_size: 5.0,
                    ..collapsible_panel("left", 40.0)
                },
                panel("right", 60.0),
            ],
            None,
        );

        assert_eq!(sizes, vec![40.0, 60.0]);
    }

    #[test]
    fn expand_last_panel_uses_previous_neighbor() {
        let mut sizes = vec![100.0, 0.0];

        expand_panel(
            &mut sizes,
            1,
            &[panel("left", 60.0), collapsible_panel("right", 40.0)],
            None,
        );

        assert_eq!(sizes, vec![60.0, 40.0]);
    }

    #[test]
    fn expand_single_panel_has_no_donor_neighbor() {
        let mut sizes = vec![0.0];

        expand_panel(&mut sizes, 0, &[collapsible_panel("only", 40.0)], None);

        assert_eq!(sizes, vec![0.0]);
    }

    #[test]
    fn expand_panel_limited_by_donor_available_size() {
        let mut donor = panel("right", 60.0);

        donor.min_size = 70.0;

        let mut sizes = vec![0.0, 90.0];

        expand_panel(
            &mut sizes,
            0,
            &[collapsible_panel("left", 40.0), donor],
            None,
        );

        assert_eq!(sizes, vec![20.0, 70.0]);
    }

    #[test]
    fn expand_panel_ignores_non_collapsible_panel() {
        let mut non_collapsible = panel("left", 0.0);

        non_collapsible.min_size = 0.0;
        non_collapsible.collapsed_size = 0.0;

        let mut sizes = vec![0.0, 100.0];

        expand_panel(
            &mut sizes,
            0,
            &[non_collapsible, panel("right", 100.0)],
            None,
        );

        assert_eq!(sizes, vec![0.0, 100.0]);
    }

    #[test]
    fn commit_sizes_rejects_non_finite_or_wrong_length_sizes() {
        let props = two_panel_props().default_sizes(vec![50.0, 50.0]);

        let mut ctx = Context::from_props(&props, &Env::default(), &Messages::default());

        commit_sizes(&mut ctx, vec![f64::NAN, 50.0]);

        assert_eq!(ctx.sizes.get(), &vec![50.0, 50.0]);

        commit_sizes(&mut ctx, vec![25.0, 25.0, 50.0]);

        assert_eq!(ctx.sizes.get(), &vec![50.0, 50.0]);
    }

    #[test]
    fn arrow_keys_resize_by_keyboard_step() {
        let mut service = service(two_panel_props().default_sizes(vec![50.0, 50.0]));

        drop(service.send(Event::KeyDown {
            handle_index: 0,
            event: key(KeyboardKey::ArrowRight),
        }));

        assert_eq!(service.context().sizes.get(), &vec![60.0, 40.0]);
    }

    #[test]
    fn reverse_arrow_keys_resize_by_keyboard_step() {
        let mut horizontal = service(two_panel_props().default_sizes(vec![50.0, 50.0]));

        drop(horizontal.send(Event::KeyDown {
            handle_index: 0,
            event: key(KeyboardKey::ArrowLeft),
        }));

        assert_eq!(horizontal.context().sizes.get(), &vec![40.0, 60.0]);

        let mut vertical = service(
            two_panel_props()
                .default_sizes(vec![50.0, 50.0])
                .orientation(Orientation::Vertical),
        );

        drop(vertical.send(Event::KeyDown {
            handle_index: 0,
            event: key(KeyboardKey::ArrowUp),
        }));

        assert_eq!(vertical.context().sizes.get(), &vec![40.0, 60.0]);
    }

    #[test]
    fn shift_arrow_uses_coarse_keyboard_step() {
        let mut service = service(
            two_panel_props()
                .default_sizes(vec![50.0, 50.0])
                .keyboard_step(4.0),
        );

        drop(service.send(Event::KeyDown {
            handle_index: 0,
            event: shift_key(KeyboardKey::ArrowRight),
        }));

        assert_eq!(service.context().sizes.get(), &vec![70.0, 30.0]);
    }

    #[test]
    fn non_positive_or_non_finite_keyboard_step_uses_default() {
        let mut zero = service(
            two_panel_props()
                .default_sizes(vec![50.0, 50.0])
                .keyboard_step(0.0),
        );

        drop(zero.send(Event::KeyDown {
            handle_index: 0,
            event: key(KeyboardKey::ArrowRight),
        }));

        assert_eq!(zero.context().sizes.get(), &vec![60.0, 40.0]);

        let mut infinite = service(
            two_panel_props()
                .default_sizes(vec![50.0, 50.0])
                .keyboard_step(f64::INFINITY),
        );

        drop(infinite.send(Event::KeyDown {
            handle_index: 0,
            event: key(KeyboardKey::ArrowRight),
        }));

        assert_eq!(infinite.context().sizes.get(), &vec![60.0, 40.0]);
    }

    #[test]
    fn home_and_end_resize_to_min_and_max() {
        let mut service = service(two_panel_props().default_sizes(vec![50.0, 50.0]));

        drop(service.send(Event::KeyDown {
            handle_index: 0,
            event: key(KeyboardKey::Home),
        }));

        assert_eq!(service.context().sizes.get(), &vec![10.0, 90.0]);

        drop(service.send(Event::KeyDown {
            handle_index: 0,
            event: key(KeyboardKey::End),
        }));

        assert_eq!(service.context().sizes.get(), &vec![90.0, 10.0]);
    }

    #[test]
    fn handle_focus_and_blur_update_focused_handle() {
        let mut service = service(two_panel_props().default_sizes(vec![50.0, 50.0]));

        assert!(
            service
                .send(Event::HandleFocus { handle_index: 0 })
                .context_changed
        );
        assert_eq!(service.context().focused_handle, Some(0));

        assert!(service.send(Event::HandleBlur).context_changed);
        assert_eq!(service.context().focused_handle, None);
    }

    #[test]
    fn invalid_handle_focus_is_ignored() {
        let mut service = service(two_panel_props().default_sizes(vec![50.0, 50.0]));

        let result = service.send(Event::HandleFocus { handle_index: 7 });

        assert!(!result.context_changed);
        assert_eq!(service.context().focused_handle, None);
    }

    #[test]
    fn enter_and_space_toggle_collapsible_panel() {
        let mut right = panel("right", 60.0);

        right.max_size = None;

        let mut service = service(
            Props::new()
                .id("split")
                .panels(vec![collapsible_panel("left", 40.0), right])
                .default_sizes(vec![40.0, 60.0]),
        );

        drop(service.send(Event::KeyDown {
            handle_index: 0,
            event: key(KeyboardKey::Enter),
        }));

        assert_eq!(service.context().sizes.get(), &vec![0.0, 100.0]);

        drop(service.send(Event::KeyDown {
            handle_index: 0,
            event: key(KeyboardKey::Space),
        }));

        assert_eq!(service.context().sizes.get(), &vec![40.0, 60.0]);
    }

    #[test]
    fn collapse_expand_restores_pre_collapse_size() {
        let mut right = panel("right", 30.0);

        right.max_size = None;

        let mut service = service(
            Props::new()
                .id("split")
                .panels(vec![collapsible_panel("left", 40.0), right])
                .default_sizes(vec![70.0, 30.0]),
        );

        drop(service.send(Event::KeyDown {
            handle_index: 0,
            event: key(KeyboardKey::Enter),
        }));

        assert_eq!(service.context().sizes.get(), &vec![0.0, 100.0]);

        drop(service.send(Event::KeyDown {
            handle_index: 0,
            event: key(KeyboardKey::Space),
        }));

        assert_eq!(service.context().sizes.get(), &vec![70.0, 30.0]);
    }

    #[test]
    fn escape_during_drag_restores_pre_drag_sizes() {
        let mut service = service(two_panel_props().default_sizes(vec![50.0, 50.0]));

        drop(service.send(Event::DragStart {
            handle_index: 0,
            pos: 0.0,
        }));
        drop(service.send(Event::DragMove { pos: 25.0 }));

        assert_eq!(service.context().sizes.get(), &vec![75.0, 25.0]);

        drop(service.send(Event::KeyDown {
            handle_index: 0,
            event: key(KeyboardKey::Escape),
        }));

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().sizes.get(), &vec![50.0, 50.0]);
    }

    #[test]
    fn non_finite_drag_move_is_ignored_while_dragging() {
        let mut service = service(two_panel_props().default_sizes(vec![50.0, 50.0]));

        drop(service.send(Event::DragStart {
            handle_index: 0,
            pos: 0.0,
        }));

        let result = service.send(Event::DragMove { pos: f64::NAN });

        assert!(!result.context_changed);
        assert_eq!(service.context().sizes.get(), &vec![50.0, 50.0]);
    }

    #[test]
    fn arrow_keys_resize_while_dragging() {
        let mut service = service(two_panel_props().default_sizes(vec![50.0, 50.0]));

        drop(service.send(Event::DragStart {
            handle_index: 0,
            pos: 0.0,
        }));

        let result = service.send(Event::KeyDown {
            handle_index: 0,
            event: key(KeyboardKey::ArrowRight),
        });

        assert!(result.context_changed);
        assert_eq!(service.state(), &State::Dragging { handle_index: 0 });
        assert_eq!(service.context().sizes.get(), &vec![60.0, 40.0]);
    }

    #[test]
    fn full_keymap_resizes_and_toggles_while_dragging() {
        let mut resize = service(two_panel_props().default_sizes(vec![50.0, 50.0]));

        drop(resize.send(Event::DragStart {
            handle_index: 0,
            pos: 0.0,
        }));
        drop(resize.send(Event::KeyDown {
            handle_index: 0,
            event: key(KeyboardKey::End),
        }));

        assert_eq!(resize.state(), &State::Dragging { handle_index: 0 });
        assert_eq!(resize.context().sizes.get(), &vec![90.0, 10.0]);

        drop(resize.send(Event::KeyDown {
            handle_index: 0,
            event: key(KeyboardKey::Home),
        }));

        assert_eq!(resize.context().sizes.get(), &vec![10.0, 90.0]);

        let mut right = panel("right", 60.0);

        right.max_size = None;

        let mut toggle = service(
            Props::new()
                .id("split")
                .panels(vec![collapsible_panel("left", 40.0), right])
                .default_sizes(vec![40.0, 60.0]),
        );

        drop(toggle.send(Event::DragStart {
            handle_index: 0,
            pos: 0.0,
        }));
        drop(toggle.send(Event::KeyDown {
            handle_index: 0,
            event: key(KeyboardKey::Enter),
        }));

        assert_eq!(toggle.state(), &State::Dragging { handle_index: 0 });
        assert_eq!(toggle.context().sizes.get(), &vec![0.0, 100.0]);

        drop(toggle.send(Event::KeyDown {
            handle_index: 0,
            event: key(KeyboardKey::Space),
        }));

        assert_eq!(toggle.context().sizes.get(), &vec![40.0, 60.0]);
    }

    #[test]
    fn horizontal_rtl_inverts_arrow_delta() {
        let mut service = service(
            two_panel_props()
                .default_sizes(vec![50.0, 50.0])
                .dir(Direction::Rtl),
        );

        drop(service.send(Event::KeyDown {
            handle_index: 0,
            event: key(KeyboardKey::ArrowRight),
        }));

        assert_eq!(service.context().sizes.get(), &vec![40.0, 60.0]);
    }

    #[test]
    fn vertical_orientation_does_not_invert_rtl_delta() {
        let mut service = service(
            two_panel_props()
                .default_sizes(vec![50.0, 50.0])
                .orientation(Orientation::Vertical)
                .dir(Direction::Rtl),
        );

        drop(service.send(Event::KeyDown {
            handle_index: 0,
            event: key(KeyboardKey::ArrowDown),
        }));

        assert_eq!(service.context().sizes.get(), &vec![60.0, 40.0]);
    }

    #[test]
    fn set_sizes_clamps_each_panel() {
        let mut service = service(two_panel_props());

        drop(service.send(Event::SetSizes {
            sizes: vec![500.0, -20.0],
        }));

        assert_eq!(service.context().sizes.get(), &vec![90.0, 10.0]);
    }

    #[test]
    fn sync_props_updates_context_backed_fields() {
        let mut service = service(two_panel_props().default_sizes(vec![50.0, 50.0]));

        drop(service.send(Event::HandleFocus { handle_index: 0 }));

        let next_props = Props::new()
            .id("split")
            .panels(vec![panel("top", 25.0), panel("bottom", 75.0)])
            .orientation(Orientation::Vertical)
            .dir(Direction::Rtl)
            .size_unit(SizeUnit::Pixels)
            .default_sizes(vec![25.0, 75.0])
            .keyboard_step(8.0);

        let result = service.send(Event::SyncProps {
            props: next_props.clone(),
        });

        assert!(result.context_changed);
        assert_eq!(service.context().panels, next_props.panels);
        assert_eq!(service.context().orientation, Orientation::Vertical);
        assert_eq!(service.context().dir, Direction::Rtl);
        assert_eq!(service.context().size_unit, SizeUnit::Pixels);
        assert_eq!(service.context().keyboard_step, 8.0);
        assert_eq!(service.context().focused_handle, Some(0));
    }

    #[test]
    fn sync_props_updates_uncontrolled_sizes_when_panel_count_grows() {
        let mut service = service(two_panel_props().default_sizes(vec![50.0, 50.0]));

        let next_props = Props::new().id("split").panels(vec![
            panel("left", 30.0),
            panel("middle", 30.0),
            panel("right", 40.0),
        ]);

        drop(service.send(Event::SyncProps { props: next_props }));

        assert_eq!(service.context().sizes.get(), &vec![50.0, 50.0, 40.0]);
    }

    #[test]
    fn sync_props_clamps_uncontrolled_sizes_to_new_panel_constraints() {
        let mut service = service(two_panel_props().default_sizes(vec![80.0, 20.0]));

        let mut left = panel("left", 80.0);

        left.min_size = 90.0;

        let next_props = Props::new()
            .id("split")
            .panels(vec![left, panel("right", 20.0)]);

        drop(service.send(Event::SyncProps { props: next_props }));

        assert_eq!(service.context().sizes.get(), &vec![90.0, 20.0]);
    }

    #[test]
    fn sync_props_exits_dragging_when_active_handle_becomes_invalid() {
        let mut service = service(props().default_sizes(vec![40.0, 30.0, 30.0]));

        drop(service.send(Event::DragStart {
            handle_index: 1,
            pos: 0.0,
        }));

        assert_eq!(service.state(), &State::Dragging { handle_index: 1 });

        drop(service.send(Event::SyncProps {
            props: two_panel_props().default_sizes(vec![50.0, 50.0]),
        }));

        assert_eq!(service.state(), &State::Idle);
        assert!(service.context().drag_start_sizes.is_empty());
        assert_eq!(service.context().drag_start_pos, 0.0);
    }

    #[test]
    fn sync_props_exits_dragging_when_active_handle_remains_valid() {
        let mut service = service(two_panel_props().default_sizes(vec![50.0, 50.0]));

        drop(service.send(Event::DragStart {
            handle_index: 0,
            pos: 25.0,
        }));

        drop(service.send(Event::SyncProps {
            props: two_panel_props().keyboard_step(5.0),
        }));

        assert_eq!(service.state(), &State::Idle);
        assert!(service.context().drag_start_sizes.is_empty());
        assert_eq!(service.context().drag_start_pos, 0.0);
    }

    #[test]
    fn on_props_changed_emits_sync_props_for_context_changes() {
        let old = two_panel_props();
        let new = two_panel_props().orientation(Orientation::Vertical);

        assert_eq!(
            <Machine as ars_core::Machine>::on_props_changed(&old, &new),
            vec![Event::SyncProps { props: new }]
        );
        assert!(<Machine as ars_core::Machine>::on_props_changed(&old, &old).is_empty());
    }

    #[test]
    fn invalid_handle_and_panel_indexes_are_ignored() {
        let mut service = service(two_panel_props().default_sizes(vec![50.0, 50.0]));

        assert!(
            !service
                .send(Event::DragStart {
                    handle_index: 7,
                    pos: 0.0
                })
                .state_changed
        );
        assert!(
            !service
                .send(Event::KeyDown {
                    handle_index: 7,
                    event: key(KeyboardKey::ArrowRight)
                })
                .context_changed
        );
        assert!(
            !service
                .send(Event::KeyDown {
                    handle_index: usize::MAX,
                    event: key(KeyboardKey::ArrowRight)
                })
                .context_changed
        );
        assert!(
            !service
                .send(Event::CollapsePanel { panel_index: 7 })
                .context_changed
        );
        assert!(
            !service
                .send(Event::ExpandPanel { panel_index: 7 })
                .context_changed
        );
        assert_eq!(service.context().sizes.get(), &vec![50.0, 50.0]);
    }

    #[test]
    fn collapse_and_expand_reject_mismatched_context_bounds() {
        let mut fewer_panels = service(
            Props::new()
                .id("split")
                .panels(vec![collapsible_panel("left", 40.0)])
                .default_sizes(vec![40.0, 60.0]),
        );

        assert!(
            !fewer_panels
                .send(Event::CollapsePanel { panel_index: 1 })
                .context_changed
        );
        assert!(
            !fewer_panels
                .send(Event::ExpandPanel { panel_index: 1 })
                .context_changed
        );

        let props = Props::new()
            .id("split")
            .panels(vec![
                collapsible_panel("left", 40.0),
                panel("middle", 30.0),
                panel("right", 30.0),
            ])
            .default_sizes(vec![40.0, 30.0, 30.0]);

        let mut ctx = Context::from_props(&props, &Env::default(), &Messages::default());

        ctx.sizes = Bindable::uncontrolled(vec![40.0, 60.0]);

        assert_eq!(
            <Machine as ars_core::Machine>::transition(
                &State::Idle,
                &Event::CollapsePanel { panel_index: 2 },
                &ctx,
                &props
            )
            .expect("invalid collapse should be an explicit no-op")
            .debug_summary(),
            "none"
        );
        assert_eq!(
            <Machine as ars_core::Machine>::transition(
                &State::Idle,
                &Event::ExpandPanel { panel_index: 2 },
                &ctx,
                &props
            )
            .expect("invalid expand should be an explicit no-op")
            .debug_summary(),
            "none"
        );

        let props = Props::new()
            .id("split")
            .panels(vec![collapsible_panel("left", 40.0), panel("right", 60.0)])
            .default_sizes(vec![40.0, 60.0]);

        let mut ctx = Context::from_props(&props, &Env::default(), &Messages::default());

        ctx.sizes = Bindable::uncontrolled(vec![40.0, 30.0, 30.0]);

        assert_eq!(
            <Machine as ars_core::Machine>::transition(
                &State::Idle,
                &Event::CollapsePanel { panel_index: 2 },
                &ctx,
                &props
            )
            .expect("invalid collapse should be an explicit no-op")
            .debug_summary(),
            "none"
        );
        assert_eq!(
            <Machine as ars_core::Machine>::transition(
                &State::Idle,
                &Event::ExpandPanel { panel_index: 2 },
                &ctx,
                &props
            )
            .expect("invalid expand should be an explicit no-op")
            .debug_summary(),
            "none"
        );
    }

    #[test]
    fn collapse_and_expand_accept_full_context_bounds() {
        let mut service = service(
            Props::new()
                .id("split")
                .panels(vec![
                    collapsible_panel("left", 40.0),
                    panel("middle", 30.0),
                    panel("right", 30.0),
                ])
                .default_sizes(vec![40.0, 30.0, 30.0]),
        );

        let result = service.send(Event::CollapsePanel { panel_index: 2 });

        assert!(result.context_changed);
    }

    #[test]
    fn root_panel_and_handle_attrs_emit_contract() {
        let service = service(two_panel_props().default_sizes(vec![50.0, 50.0]));

        let api = service.connect(&|_| {});

        let root = api.root_attrs();

        assert_eq!(root.get(&HtmlAttr::Data("ars-scope")), Some("splitter"));
        assert_eq!(root.get(&HtmlAttr::Data("ars-part")), Some("root"));
        assert_eq!(
            root.get(&HtmlAttr::Data("ars-orientation")),
            Some("horizontal")
        );
        assert_eq!(root.get(&HtmlAttr::Data("ars-state")), Some("idle"));

        let panel = api.panel_attrs(0);

        assert_eq!(panel.get(&HtmlAttr::Id), Some("split-panel-0"));
        assert_eq!(panel.get(&HtmlAttr::Data("ars-panel-id")), Some("left"));
        assert!(panel.styles().contains(&(CssProperty::Width, "50%".into())));

        let handle = api.handle_attrs(0);

        assert_eq!(handle.get(&HtmlAttr::Role), Some("separator"));
        assert_eq!(
            handle.get(&HtmlAttr::Aria(AriaAttr::Orientation)),
            Some("vertical")
        );
        assert_eq!(handle.get(&HtmlAttr::Aria(AriaAttr::ValueNow)), Some("50"));
        assert_eq!(handle.get(&HtmlAttr::Aria(AriaAttr::ValueMin)), Some("10"));
        assert_eq!(handle.get(&HtmlAttr::Aria(AriaAttr::ValueMax)), Some("90"));
        assert_eq!(
            handle.get(&HtmlAttr::Aria(AriaAttr::ValueText)),
            Some("50%")
        );
        assert_eq!(handle.get(&HtmlAttr::Aria(AriaAttr::Label)), Some("Resize"));
        assert_eq!(
            handle.get(&HtmlAttr::Aria(AriaAttr::Controls)),
            Some("split-panel-0")
        );
        assert_eq!(handle.get(&HtmlAttr::TabIndex), Some("0"));
    }

    #[test]
    fn collapsible_panel_above_collapsed_size_does_not_emit_collapsed_attr() {
        let service = service(
            Props::new()
                .id("split")
                .panels(vec![collapsible_panel("left", 40.0), panel("right", 60.0)])
                .default_sizes(vec![40.0, 60.0]),
        );

        let attrs = service.connect(&|_| {}).panel_attrs(0);

        assert_eq!(attrs.get(&HtmlAttr::Data("ars-collapsed")), None);
    }

    #[test]
    fn second_handle_aria_max_uses_right_neighbor_minimum() {
        let mut middle = panel("middle", 30.0);

        middle.max_size = None;

        let mut right = panel("right", 40.0);

        right.min_size = 20.0;

        let service = service(
            Props::new()
                .id("split")
                .panels(vec![panel("left", 30.0), middle, right])
                .default_sizes(vec![30.0, 30.0, 40.0]),
        );

        let attrs = service.connect(&|_| {}).handle_attrs(1);

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueMax)), Some("80"));
    }

    #[test]
    fn handle_aria_max_uses_collapsible_neighbor_collapsed_size() {
        let mut left = panel("left", 100.0);

        left.max_size = None;

        let mut right = collapsible_panel("right", 0.0);

        right.min_size = 20.0;
        right.collapsed_size = 0.0;

        let service = service(
            Props::new()
                .id("split")
                .panels(vec![left, right])
                .default_sizes(vec![100.0, 0.0]),
        );

        let attrs = service.connect(&|_| {}).handle_attrs(0);

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueNow)), Some("100"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueMax)), Some("100"));
    }

    #[test]
    fn pixel_handle_attrs_convert_values_to_percent() {
        let mut left = panel("left", 200.0);

        left.max_size = None;

        let mut right = panel("right", 300.0);

        right.min_size = 100.0;
        right.max_size = None;

        let service = service(
            Props::new()
                .id("split")
                .panels(vec![left, right])
                .size_unit(SizeUnit::Pixels)
                .default_sizes(vec![200.0, 300.0]),
        );

        let attrs = service.connect(&|_| {}).handle_attrs(0);

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueNow)), Some("40"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueMin)), Some("2"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueMax)), Some("80"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueText)), Some("40%"));
    }

    #[test]
    fn collapsed_handle_attrs_use_collapsed_value_text() {
        let service = service(
            Props::new()
                .id("split")
                .panels(vec![collapsible_panel("left", 40.0), panel("right", 60.0)])
                .default_sizes(vec![0.0, 100.0]),
        );

        let attrs = service.connect(&|_| {}).handle_attrs(0);

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueMin)), Some("0"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::ValueText)),
            Some("Collapsed")
        );
    }

    #[test]
    fn collapsible_handle_aria_min_uses_nonzero_collapsed_size() {
        let mut left = collapsible_panel("left", 5.0);

        left.min_size = 20.0;
        left.collapsed_size = 5.0;

        let service = service(
            Props::new()
                .id("split")
                .panels(vec![left, panel("right", 95.0)])
                .default_sizes(vec![5.0, 95.0]),
        );

        let attrs = service.connect(&|_| {}).handle_attrs(0);

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueMin)), Some("5"));
    }

    #[test]
    fn non_collapsible_zero_size_handle_uses_numeric_value_text() {
        let mut left = panel("left", 0.0);

        left.min_size = 0.0;

        let service = service(
            Props::new()
                .id("split")
                .panels(vec![left, panel("right", 100.0)])
                .default_sizes(vec![0.0, 100.0]),
        );

        let attrs = service.connect(&|_| {}).handle_attrs(0);

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ValueText)), Some("0%"));
    }

    #[test]
    fn api_methods_dispatch_events() {
        let sent = RefCell::new(Vec::new());

        let service = service(two_panel_props().default_sizes(vec![50.0, 50.0]));

        let send = |event| sent.borrow_mut().push(event);

        let api = service.connect(&send);

        api.collapse_panel(0);
        api.expand_panel(0);
        api.resize_panel(1, 42.0);
        api.reset_sizes();
        api.on_handle_pointerdown(0, 10.0);
        api.on_handle_pointermove(12.0);
        api.on_handle_pointerup();
        api.on_handle_keydown(0, key(KeyboardKey::ArrowRight));
        api.on_handle_focus(0);
        api.on_handle_blur();

        assert_eq!(sent.borrow().len(), 10);
    }

    #[test]
    fn resize_panel_ignores_index_at_size_len() {
        let sent = RefCell::new(Vec::new());

        let service = service(two_panel_props().default_sizes(vec![50.0, 50.0]));

        let send = |event| sent.borrow_mut().push(event);

        let api = service.connect(&send);

        api.resize_panel(2, 42.0);

        assert!(sent.borrow().is_empty());
    }

    #[test]
    fn part_attrs_delegates_to_specific_attr_methods() {
        let service = service(two_panel_props().default_sizes(vec![50.0, 50.0]));

        let api = service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(api.part_attrs(Part::Panel { index: 0 }), api.panel_attrs(0));
        assert_eq!(
            api.part_attrs(Part::Handle { index: 0 }),
            api.handle_attrs(0)
        );
    }

    #[test]
    fn splitter_root_idle_snapshot() {
        let service = service(two_panel_props().default_sizes(vec![50.0, 50.0]));

        assert_snapshot!(
            "splitter_root_idle",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn splitter_root_dragging_snapshot() {
        let mut service = service(two_panel_props().default_sizes(vec![50.0, 50.0]));

        drop(service.send(Event::DragStart {
            handle_index: 0,
            pos: 0.0,
        }));

        assert_snapshot!(
            "splitter_root_dragging",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn splitter_panel_percent_snapshot() {
        let service = service(two_panel_props().default_sizes(vec![50.0, 50.0]));

        assert_snapshot!(
            "splitter_panel_percent",
            snapshot_attrs(&service.connect(&|_| {}).panel_attrs(0))
        );
    }

    #[test]
    fn splitter_panel_collapsed_snapshot() {
        let mut right = panel("right", 60.0);

        right.max_size = None;

        let mut service = service(
            Props::new()
                .id("split")
                .panels(vec![collapsible_panel("left", 40.0), right])
                .default_sizes(vec![40.0, 60.0]),
        );

        drop(service.send(Event::CollapsePanel { panel_index: 0 }));

        assert_snapshot!(
            "splitter_panel_collapsed",
            snapshot_attrs(&service.connect(&|_| {}).panel_attrs(0))
        );
    }

    #[test]
    fn splitter_handle_idle_snapshot() {
        let service = service(two_panel_props().default_sizes(vec![50.0, 50.0]));

        assert_snapshot!(
            "splitter_handle_idle",
            snapshot_attrs(&service.connect(&|_| {}).handle_attrs(0))
        );
    }

    #[test]
    fn splitter_handle_focused_snapshot() {
        let mut service = service(two_panel_props().default_sizes(vec![50.0, 50.0]));

        drop(service.send(Event::HandleFocus { handle_index: 0 }));

        assert_snapshot!(
            "splitter_handle_focused",
            snapshot_attrs(&service.connect(&|_| {}).handle_attrs(0))
        );
    }

    #[test]
    fn splitter_handle_dragging_snapshot() {
        let mut service = service(two_panel_props().default_sizes(vec![50.0, 50.0]));

        drop(service.send(Event::DragStart {
            handle_index: 0,
            pos: 0.0,
        }));

        assert_snapshot!(
            "splitter_handle_dragging",
            snapshot_attrs(&service.connect(&|_| {}).handle_attrs(0))
        );
    }

    #[test]
    fn splitter_vertical_handle_snapshot() {
        let service = service(
            two_panel_props()
                .orientation(Orientation::Vertical)
                .default_sizes(vec![50.0, 50.0]),
        );

        assert_snapshot!(
            "splitter_vertical_handle",
            snapshot_attrs(&service.connect(&|_| {}).handle_attrs(0))
        );
    }
}
