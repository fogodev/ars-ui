---
component: Tour
category: overlay
tier: complex
foundation_deps: [architecture, accessibility, interactions]
shared_deps: [z-index-stacking]
related: []
references:
  ark-ui: Tour
---

# Tour

A `Tour` provides a guided step-by-step walkthrough that highlights elements on the page
and displays explanatory content anchored to each target. Tours are commonly used for
onboarding, feature discovery, and interactive tutorials.

## 1. State Machine

### 1.1 States

```rust
/// The states of the `Tour` component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// Tour has not started or has been reset.
    Inactive,
    /// Tour is active and showing a specific step.
    Active {
        /// The index of the current step.
        step_index: usize,
    },
    /// Tour has been completed (all steps visited).
    Completed,
}
```

### 1.2 Events

```rust
/// The events of the `Tour` component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Start the tour from step 0 (or resume from last position).
    Start,
    /// Advance to the next step. Completes the tour if on the last step.
    NextStep,
    /// Go back to the previous step.
    PrevStep,
    /// Jump to a specific step by index.
    GoToStep(usize),
    /// User skipped the tour (closes without completing).
    Skip,
    /// Tour completed (all steps visited).
    Complete,
    /// User dismissed the tour (Escape key or overlay click).
    Dismiss,
    /// Dynamically add a step at the given index. Shifts subsequent steps.
    AddStep { index: usize, step: Step },
    /// Remove the step at the given index. Shifts subsequent steps.
    RemoveStep(usize),
    /// Replace the step at the given index.
    UpdateStep { index: usize, step: Step },
    /// Callback when step changes (adapter dispatches after transition).
    StepChange(usize),
    /// Focus received on tour content.
    Focus {
        /// Whether the focus came from a keyboard event.
        is_keyboard: bool,
    },
    /// Focus lost from tour content.
    Blur,
}
```

### 1.3 Context

```rust
/// The presentation type of a tour step.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum StepType {
    /// Anchored popover-style positioning near the target element. Default.
    #[default]
    Tooltip,
    /// Centered modal dialog (no target anchoring).
    Dialog,
    /// Fixed-position floating panel.
    Floating,
    /// Invisible step that waits for a condition before auto-advancing.
    /// No UI is rendered; progression happens when `effect` completes.
    Wait,
}

/// The definition of a step in the tour.
#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    /// CSS selector or element ID for the target element to highlight.
    /// Ignored for `StepType::Dialog` and `StepType::Floating`.
    pub target: Option<String>,
    /// Step title text.
    pub title: String,
    /// Step description/content text.
    pub content: String,
    /// Presentation type of this step. Default: `StepType::Tooltip`.
    pub step_type: StepType,
    /// Preferred placement of the step content relative to the target.
    /// Only used for `StepType::Tooltip`.
    pub placement: Placement,
    /// Spotlight border-radius for the highlight cutout (pixels). Default: 4.0.
    pub spotlight_radius: f64,
    /// Spotlight padding around the target element (pixels). Default: 8.0.
    pub spotlight_offset: f64,
}

impl Default for Step {
    fn default() -> Self {
        Self {
            target: None,
            title: String::new(),
            content: String::new(),
            step_type: StepType::Tooltip,
            placement: Placement::Bottom,
            spotlight_radius: 4.0,
            spotlight_offset: 8.0,
        }
    }
}

/// The context of the `Tour` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The resolved locale for message formatting.
    pub locale: Locale,
    /// Step definitions.
    pub steps: Vec<Step>,
    /// Current step index (0-based).
    pub current_step: usize,
    /// Total number of steps.
    pub total_steps: usize,
    /// ID of the current target element (resolved from `steps[current_step].target`).
    pub target_element_id: Option<String>,
    /// Whether the tour content is focused.
    pub focused: bool,
    /// Whether the focus is visible.
    pub focus_visible: bool,
    /// Whether the tour is open (active).
    pub open: bool,
    /// Component instance IDs.
    pub ids: ComponentIds,
    /// Resolved messages for accessibility labels.
    pub messages: Messages,
}
```

### 1.4 Props

```rust
/// The props of the `Tour` component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Step definitions for the tour.
    pub steps: Vec<Step>,
    /// Controlled open state. When `Some`, consumer controls tour visibility.
    pub open: Option<bool>,
    /// Whether the tour opens by default (uncontrolled). Default: false.
    pub default_open: bool,
    /// Whether to automatically start the tour on mount.
    pub auto_start: bool,
    /// Whether clicking the overlay dismisses the tour.
    pub close_on_overlay_click: bool,
    /// Whether Escape key dismisses the tour. Default: true.
    pub close_on_escape: bool,
    /// Whether keyboard navigation (arrow keys) between steps is enabled.
    pub keyboard_navigation: bool,
    /// Callback invoked when the tour open state changes.
    pub on_open_change: Option<Callback<bool>>,
    /// Callback invoked when the current step changes. Receives the new step index.
    pub on_step_change: Option<Callback<usize>>,
    /// When true, tour content is not mounted until started. Default: false.
    pub lazy_mount: bool,
    /// When true, tour content is removed from the DOM after completing. Default: false.
    pub unmount_on_exit: bool,
    /// Localizable messages (see §4 Internationalization).
    pub messages: Option<Messages>,
    /// Optional locale override. When `None`, resolved from the nearest `ArsProvider` context.
    pub locale: Option<Locale>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            steps: Vec::new(),
            open: None,
            default_open: false,
            auto_start: false,
            close_on_overlay_click: true,
            close_on_escape: true,
            keyboard_navigation: true,
            on_open_change: None,
            on_step_change: None,
            lazy_mount: false,
            unmount_on_exit: false,
            messages: None,
            locale: None,
        }
    }
}
```

### 1.5 Full Machine Implementation

```rust
use ars_core::{TransitionPlan, PendingEffect, ComponentIds, AttrMap, Bindable};

/// The machine of the `Tour` component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State   = State;
    type Event   = Event;
    type Context = Context;
    type Props   = Props;
    type Api<'a> = Api<'a>;

    fn init(props: &Props) -> (State, Context) {
        let ids = ComponentIds::from_id(&props.id);
        let total = props.steps.len();
        let initial_state = if props.auto_start && total > 0 {
            State::Active { step_index: 0 }
        } else {
            State::Inactive
        };
        let open = matches!(initial_state, State::Active { .. });
        let locale = resolve_locale(props.locale.as_ref());
        let messages = resolve_messages::<Messages>(props.messages.as_ref(), &locale);
        (initial_state, Context {
            locale,
            steps: props.steps.clone(),
            current_step: 0,
            total_steps: total,
            target_element_id: props.steps.first().and_then(|s| s.target.clone()),
            focused: false,
            focus_visible: false,
            open,
            ids,
            messages,
        })
    }

    fn transition(
        state: &State,
        event: &Event,
        ctx: &Context,
        props: &Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            // ── Start ───────────────────────────────────────────────────
            (State::Inactive, Event::Start) if ctx.total_steps > 0 => {
                Some(TransitionPlan::to(State::Active { step_index: 0 })
                    .apply(|ctx| {
                        ctx.current_step = 0;
                        ctx.target_element_id = ctx.steps.first().and_then(|s| s.target.clone());
                        ctx.open = true;
                    })
                    .with_named_effect("focus-step", |ctx, _props, _send| {
                        let platform = use_platform_effects();
                        let content_id = ctx.ids.part("step-content");
                        platform.focus_element_by_id(&content_id);
                        no_cleanup()
                    }))
            }

            // ── Next step ───────────────────────────────────────────────
            (State::Active { step_index }, Event::NextStep) => {
                let next = step_index + 1;
                if next >= ctx.total_steps {
                    Some(TransitionPlan::to(State::Completed).apply(|ctx| {
                        ctx.open = false;
                    }))
                } else {
                    Some(TransitionPlan::to(State::Active { step_index: next })
                        .apply(move |ctx| {
                            ctx.current_step = next;
                            ctx.target_element_id = ctx.steps.get(next).and_then(|s| s.target.clone());
                        })
                        .with_named_effect("focus-step", |ctx, _props, _send| {
                            let platform = use_platform_effects();
                            let content_id = ctx.ids.part("step-content");
                            platform.focus_element_by_id(&content_id);
                            no_cleanup()
                        }))
                }
            }

            // ── Previous step ───────────────────────────────────────────
            (State::Active { step_index }, Event::PrevStep) if *step_index > 0 => {
                let prev = step_index - 1;
                Some(TransitionPlan::to(State::Active { step_index: prev })
                    .apply(move |ctx| {
                        ctx.current_step = prev;
                        ctx.target_element_id = ctx.steps.get(prev).and_then(|s| s.target.clone());
                    })
                    .with_named_effect("focus-step", |ctx, _props, _send| {
                        let platform = use_platform_effects();
                        let content_id = ctx.ids.part("step-content");
                        platform.focus_element_by_id(&content_id);
                        no_cleanup()
                    }))
            }

            // ── Go to step ──────────────────────────────────────────────
            (State::Active { .. }, Event::GoToStep(index)) if *index < ctx.total_steps => {
                let idx = *index;
                Some(TransitionPlan::to(State::Active { step_index: idx })
                    .apply(move |ctx| {
                        ctx.current_step = idx;
                        ctx.target_element_id = ctx.steps.get(idx).and_then(|s| s.target.clone());
                    })
                    .with_named_effect("focus-step", |ctx, _props, _send| {
                        let platform = use_platform_effects();
                        let content_id = ctx.ids.part("step-content");
                        platform.focus_element_by_id(&content_id);
                        no_cleanup()
                    }))
            }

            // ── Skip / Dismiss ──────────────────────────────────────────
            (State::Active { .. }, Event::Skip)
            | (State::Active { .. }, Event::Dismiss) => {
                Some(TransitionPlan::to(State::Inactive).apply(|ctx| {
                    ctx.open = false;
                }))
            }

            // ── Complete ────────────────────────────────────────────────
            (State::Active { .. }, Event::Complete) => {
                Some(TransitionPlan::to(State::Completed).apply(|ctx| {
                    ctx.open = false;
                }))
            }

            // ── Restart from completed ──────────────────────────────────
            (State::Completed, Event::Start) if ctx.total_steps > 0 => {
                Some(TransitionPlan::to(State::Active { step_index: 0 })
                    .apply(|ctx| {
                        ctx.current_step = 0;
                        ctx.target_element_id = ctx.steps.first().and_then(|s| s.target.clone());
                        ctx.open = true;
                    }))
            }

            // ── Focus ───────────────────────────────────────────────────
            (_, Event::Focus { is_keyboard }) => {
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focused = true;
                    ctx.focus_visible = *is_keyboard;
                }))
            }
            (_, Event::Blur) => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.focused = false;
                    ctx.focus_visible = false;
                }))
            }

            // ── Dynamic step management ─────────────────────────────────
            (State::Active { .. }, Event::AddStep { index, step }) => {
                let index = *index;
                let step = step.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    let clamped = index.min(ctx.steps.len());
                    ctx.steps.insert(clamped, step);
                    ctx.total_steps = ctx.steps.len();
                    // If inserted before current, shift current forward
                    if clamped <= ctx.current_step {
                        ctx.current_step += 1;
                    }
                }))
            }
            (State::Active { step_index }, Event::RemoveStep(index))
                if *index < ctx.total_steps => {
                let index = *index;
                let step_index = *step_index;
                if ctx.total_steps <= 1 {
                    // Last step removed — dismiss tour
                    Some(TransitionPlan::to(State::Inactive).apply(|ctx| {
                        ctx.steps.clear();
                        ctx.total_steps = 0;
                        ctx.open = false;
                    }))
                } else {
                    let new_step = if index < step_index {
                        step_index - 1
                    } else if index == step_index {
                        step_index.min(ctx.total_steps - 2)
                    } else {
                        step_index
                    };
                    Some(TransitionPlan::to(State::Active { step_index: new_step })
                        .apply(move |ctx| {
                            ctx.steps.remove(index);
                            ctx.total_steps = ctx.steps.len();
                            ctx.current_step = new_step;
                            ctx.target_element_id = ctx.steps.get(new_step)
                                .and_then(|s| s.target.clone());
                        }))
                }
            }
            (State::Active { .. }, Event::UpdateStep { index, step })
                if *index < ctx.total_steps => {
                let index = *index;
                let step = step.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.steps[index] = step;
                    if index == ctx.current_step {
                        ctx.target_element_id = ctx.steps.get(index)
                            .and_then(|s| s.target.clone());
                    }
                }))
            }

            _ => None,
        }
    }

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

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "tour"]
pub enum Part {
    Root,
    Overlay,
    Highlight,
    StepContent,
    StepTitle,
    StepDescription,
    NextTrigger,
    PrevTrigger,
    SkipTrigger,
    CloseTrigger,
    Progress,
    StepIndicator { index: usize },
}

/// The API of the `Tour` component.
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
        attrs.set(HtmlAttr::Id, self.ctx.ids.id());
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.label)(&self.ctx.locale));
        let state_str = match self.state {
            State::Inactive => "inactive",
            State::Active { .. } => "active",
            State::Completed => "completed",
        };
        attrs.set(HtmlAttr::Data("ars-state"), state_str);
        attrs
    }

    pub fn overlay_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Overlay.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }

    pub fn highlight_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Highlight.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }

    pub fn step_content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::StepContent.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("step-content"));
        attrs.set(HtmlAttr::Role, "dialog");
        attrs.set(HtmlAttr::Aria(AriaAttr::Modal), "false");
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), self.ctx.ids.part("step-title"));
        attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), self.ctx.ids.part("step-description"));
        if let State::Active { step_index } = self.state {
            attrs.set(HtmlAttr::Data("ars-step"), step_index.to_string());
        }
        attrs
    }

    pub fn step_title_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::StepTitle.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("step-title"));
        attrs
    }

    pub fn step_description_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::StepDescription.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("step-description"));
        attrs
    }

    pub fn next_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::NextTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        let is_last = self.ctx.current_step >= self.ctx.total_steps.saturating_sub(1);
        if is_last {
            attrs.set_bool(HtmlAttr::Data("ars-last-step"), true);
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.done_label)(&self.ctx.locale));
        } else {
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.next_label)(&self.ctx.locale));
        }
        attrs
    }

    pub fn prev_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::PrevTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.prev_label)(&self.ctx.locale));
        if self.ctx.current_step == 0 {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }
        attrs
    }

    pub fn skip_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::SkipTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.skip_label)(&self.ctx.locale));
        attrs
    }

    pub fn close_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::CloseTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.close_label)(&self.ctx.locale));
        attrs
    }

    pub fn progress_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Progress.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Live), "polite");
        attrs
    }

    pub fn step_indicator_attrs(&self, step_index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::StepIndicator { index: step_index }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-step"), step_index.to_string());
        if step_index == self.ctx.current_step {
            attrs.set_bool(HtmlAttr::Data("ars-current"), true);
        }
        attrs
    }

    // ── Convenience getters ─────────────────────────────────────────────

    pub fn current_step(&self) -> usize { self.ctx.current_step }
    pub fn total_steps(&self) -> usize { self.ctx.total_steps }
    pub fn is_first_step(&self) -> bool { self.ctx.current_step == 0 }
    pub fn is_last_step(&self) -> bool { self.ctx.current_step >= self.ctx.total_steps.saturating_sub(1) }
    pub fn is_open(&self) -> bool { self.ctx.open }
    pub fn current_step_def(&self) -> Option<&Step> { self.ctx.steps.get(self.ctx.current_step) }

    /// Progress as a percentage (0.0 to 100.0).
    pub fn progress_percent(&self) -> f64 {
        if self.ctx.total_steps == 0 { return 0.0; }
        ((self.ctx.current_step + 1) as f64 / self.ctx.total_steps as f64) * 100.0
    }

    /// Progress as a localized text string (e.g., "Step 2 of 5").
    pub fn progress_text(&self) -> String {
        (self.ctx.messages.progress_text)(self.ctx.current_step + 1, self.ctx.total_steps)
    }

    /// Whether there is a next step.
    pub fn has_next_step(&self) -> bool {
        self.ctx.current_step + 1 < self.ctx.total_steps
    }

    /// Whether there is a previous step.
    pub fn has_prev_step(&self) -> bool {
        self.ctx.current_step > 0
    }

    /// The current step definition.
    pub fn current_step_info(&self) -> Option<&Step> {
        self.ctx.steps.get(self.ctx.current_step)
    }

    pub fn on_keydown(&self, data: &KeyboardEventData) {
        match data.key {
            KeyboardKey::Escape if self.props.close_on_escape => (self.send)(Event::Dismiss),
            KeyboardKey::ArrowRight if self.props.keyboard_navigation => (self.send)(Event::NextStep),
            KeyboardKey::ArrowLeft if self.props.keyboard_navigation => (self.send)(Event::PrevStep),
            _ => {}
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Overlay => self.overlay_attrs(),
            Part::Highlight => self.highlight_attrs(),
            Part::StepContent => self.step_content_attrs(),
            Part::StepTitle => self.step_title_attrs(),
            Part::StepDescription => self.step_description_attrs(),
            Part::NextTrigger => self.next_trigger_attrs(),
            Part::PrevTrigger => self.prev_trigger_attrs(),
            Part::SkipTrigger => self.skip_trigger_attrs(),
            Part::CloseTrigger => self.close_trigger_attrs(),
            Part::Progress => self.progress_attrs(),
            Part::StepIndicator { index } => self.step_indicator_attrs(index),
        }
    }
}
```

## 2. Anatomy

```text
Tour
├── Root
│   ├── Overlay                     backdrop covering the page
│   ├── Highlight                   spotlight around target element
│   └── StepContent                 role="dialog"
│       ├── CloseTrigger            dismiss button
│       ├── StepTitle               heading for the step
│       ├── StepDescription         explanatory text
│       ├── Progress                "Step N of M"
│       │   └── StepIndicator (×N)  dot for each step
│       ├── PrevTrigger             previous button
│       ├── NextTrigger             next button (or finish on last step)
│       └── SkipTrigger             skip entire tour
```

| Part            | Element    | Key Attributes                                                               |
| --------------- | ---------- | ---------------------------------------------------------------------------- |
| Root            | `<div>`    | `data-ars-scope="tour"`, `data-ars-state`, `aria-label`                      |
| Overlay         | `<div>`    | `aria-hidden="true"`                                                         |
| Highlight       | `<div>`    | `aria-hidden="true"`                                                         |
| StepContent     | `<div>`    | `role="dialog"`, `aria-modal="false"`, `aria-labelledby`, `aria-describedby` |
| StepTitle       | `<h3>`     | `id` for `aria-labelledby`                                                   |
| StepDescription | `<p>`      | `id` for `aria-describedby`                                                  |
| Progress        | `<div>`    | `aria-live="polite"`                                                         |
| StepIndicator   | `<span>`   | `data-ars-step`, `data-ars-current`                                          |
| PrevTrigger     | `<button>` | `aria-label`, `aria-disabled` when on first step                             |
| NextTrigger     | `<button>` | `aria-label` (Next / Done), `data-ars-last-step` when on last step           |
| SkipTrigger     | `<button>` | `type="button"`, `aria-label`                                                |
| CloseTrigger    | `<button>` | `type="button"`, `aria-label`                                                |

**12 parts total.**

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part        | Property           | Value                               |
| ----------- | ------------------ | ----------------------------------- |
| StepContent | `role`             | `"dialog"`                          |
| StepContent | `aria-modal`       | `"false"` (non-modal)               |
| StepContent | `aria-labelledby`  | StepTitle ID                        |
| StepContent | `aria-describedby` | StepDescription ID                  |
| Progress    | `aria-live`        | `"polite"` (announces step changes) |

### 3.2 Keyboard Interaction

| Key        | Action                                                 |
| ---------- | ------------------------------------------------------ |
| Escape     | Dismiss the tour                                       |
| ArrowRight | Next step (when `keyboard_navigation` is enabled)      |
| ArrowLeft  | Previous step (when `keyboard_navigation` is enabled)  |
| Tab        | Cycle through interactive elements within step content |

### 3.3 Focus Management

- **Focus trap per step**: When a step is active, focus is trapped within the step content
  using FocusScope. When transitioning between steps, focus moves to the new step content.
- **Step announcements**: The progress element uses `aria-live="polite"` to announce step
  changes to screen readers (e.g., "Step 2 of 5").
- **Non-modal**: Tour uses `aria-modal="false"` because the underlying page content should
  remain partially accessible. The overlay provides visual dimming but doesn't block AT
  from reading background content.

## 4. Internationalization

### 4.1 Messages

```rust
/// Localizable messages for the Tour component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Label for the tour overlay. Default: `"Tour"`.
    pub label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Format string for progress text. Receives (current, total).
    /// Default: `"Step {current} of {total}"`.
    pub progress_text: Box<dyn Fn(usize, usize) -> String + Send + Sync>,
    /// Label for the Next button. Default: `"Next"`.
    pub next_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Label for the Previous button. Default: `"Previous"`.
    pub prev_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Label for the Skip button. Default: `"Skip tour"`.
    pub skip_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Label for the Close button. Default: `"Close"`.
    pub close_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Label for the Done/Finish button (last step). Default: `"Done"`.
    pub done_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            label: MessageFn::new(|_| "Tour".to_string()),
            progress_text: Box::new(|current, total| format!("Step {} of {}", current, total)),
            next_label: MessageFn::new(|_| "Next".to_string()),
            prev_label: MessageFn::new(|_| "Previous".to_string()),
            skip_label: MessageFn::new(|_| "Skip tour".to_string()),
            close_label: MessageFn::new(|_| "Close".to_string()),
            done_label: MessageFn::new(|_| "Done".to_string()),
        }
    }
}

impl ComponentMessages for Messages {}
```

- RTL: Arrow key direction for step navigation is NOT reversed (ArrowRight always means
  "next") because step progression is conceptual, not spatial.

## 5. Positioning

`Tour` step content is positioned relative to the target element using the positioning engine
from `05-interactions.md`. Each `Step` specifies a preferred `Placement`. The
positioning engine automatically handles:

- Flipping when the preferred placement would overflow the viewport.
- Sliding along the edge to keep the content fully visible.
- Offset from the target element (default: 8px gap).

The highlight element is positioned as an overlay matching the target element's bounding
rect, with optional padding to create a spotlight effect.

## 6. Library Parity

> Compared against: Ark UI (`Tour`).

Radix UI and React Aria do not have a Tour component.

### 6.1 Props

| Feature                 | ars-ui                   | Ark UI                 | Notes                                                |
| ----------------------- | ------------------------ | ---------------------- | ---------------------------------------------------- |
| Steps                   | `steps`                  | (via useTour)          | Ark UI configures steps in a hook; ars-ui uses props |
| Controlled open         | `open`                   | --                     | ars-ui addition; Ark UI uses useTour imperative API  |
| Default open            | `default_open`           | --                     | ars-ui addition                                      |
| Auto start              | `auto_start`             | --                     | ars-ui addition                                      |
| Close on overlay click  | `close_on_overlay_click` | --                     | ars-ui addition                                      |
| Close on Escape         | `close_on_escape`        | --                     | ars-ui addition                                      |
| Keyboard navigation     | `keyboard_navigation`    | --                     | ars-ui addition                                      |
| Lazy mount              | `lazy_mount`             | `lazyMount`            | Same                                                 |
| Unmount on exit         | `unmount_on_exit`        | `unmountOnExit`        | Same                                                 |
| Present                 | --                       | `present`              | Ark UI controlled visibility                         |
| Immediate               | --                       | `immediate`            | Ark UI sync mode                                     |
| Skip animation on mount | --                       | `skipAnimationOnMount` | Ark UI only                                          |
| Open change callback    | `on_open_change`         | --                     | ars-ui addition                                      |
| Step change callback    | `on_step_change`         | --                     | ars-ui addition                                      |

**Gaps:** None. Ark UI's Tour API is imperative (`useTour` hook); ars-ui uses a declarative state machine with callbacks.

### 6.2 Anatomy

| Part            | ars-ui          | Ark UI         | Notes                          |
| --------------- | --------------- | -------------- | ------------------------------ |
| Root            | Root            | Root           | Container                      |
| Overlay         | Overlay         | Backdrop       | Page dimming                   |
| Highlight       | Highlight       | Spotlight      | Target highlight               |
| StepContent     | StepContent     | Content        | Step dialog                    |
| StepTitle       | StepTitle       | Title          | Step heading                   |
| StepDescription | StepDescription | Description    | Step text                      |
| NextTrigger     | NextTrigger     | ActionTrigger  | Next/Done button               |
| PrevTrigger     | PrevTrigger     | ActionTrigger  | Previous button                |
| SkipTrigger     | SkipTrigger     | --             | ars-ui addition                |
| CloseTrigger    | CloseTrigger    | CloseTrigger   | Dismiss button                 |
| Progress        | Progress        | ProgressText   | Step progress                  |
| StepIndicator   | StepIndicator   | --             | ars-ui addition (dot per step) |
| Arrow           | --              | Arrow/ArrowTip | Ark UI arrow parts             |
| Positioner      | --              | Positioner     | Ark UI positioning wrapper     |
| Actions         | --              | Actions        | Ark UI button container        |
| Control         | --              | Control        | Ark UI control wrapper         |

**Gaps:** None. Ark UI's `Arrow`/`ArrowTip` and `Positioner` are handled by the positioning engine. Ark UI's `Actions` and `Control` are structural wrappers; ars-ui uses explicit trigger parts.

### 6.3 Events

| Callback      | ars-ui           | Ark UI           | Notes               |
| ------------- | ---------------- | ---------------- | ------------------- |
| Open change   | `on_open_change` | --               | ars-ui addition     |
| Step change   | `on_step_change` | --               | ars-ui addition     |
| Exit complete | (Presence)       | `onExitComplete` | Handled by Presence |

**Gaps:** None.

### 6.4 Features

| Feature                                   | ars-ui         | Ark UI           |
| ----------------------------------------- | -------------- | ---------------- |
| Step navigation (next/prev)               | Yes            | Yes              |
| Go to step (jump)                         | Yes            | Yes              |
| Skip tour                                 | Yes            | --               |
| Spotlight/highlight                       | Yes            | Yes              |
| Step types (tooltip/dialog/floating/wait) | Yes            | (tooltip/dialog) |
| Dynamic step add/remove/update            | Yes            | --               |
| Keyboard navigation (arrows)              | Yes            | --               |
| Progress text/percent                     | Yes            | Yes              |
| Step indicators                           | Yes            | --               |
| Focus management per step                 | Yes            | --               |
| aria-live progress announcements          | Yes            | --               |
| Overlay click dismiss                     | Yes            | --               |
| Auto start                                | Yes            | --               |
| Animation support                         | Yes (Presence) | Yes              |

**Gaps:** None.

### 6.5 Summary

- **Overall:** Full parity with Ark UI; significantly exceeds reference with additional features.
- **Divergences:** (1) Ark UI uses an imperative `useTour` hook API; ars-ui uses a declarative state machine with `Props`-based configuration. (2) ars-ui adds `StepType::Wait` (invisible condition-based auto-advance), `StepType::Floating` (fixed-position panel), and dynamic step management (add/remove/update at runtime). (3) ars-ui adds skip trigger, step indicators, keyboard arrow navigation, and `aria-live` progress announcements.
- **Recommended additions:** None.
