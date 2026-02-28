---
component: Steps
category: navigation
tier: stateful
foundation_deps: [architecture, accessibility, interactions]
shared_deps: []
related: []
references:
  ark-ui: Steps
---

# Steps

A multi-step wizard progress indicator. Displays a sequence of steps with per-step status
and provides navigation between them.

## 1. State Machine

### 1.1 States

| State  | Description                                                 |
| ------ | ----------------------------------------------------------- |
| `Idle` | The only machine state; current step is tracked in context. |

### 1.2 Events

| Event                        | Payload              | Description                            |
| ---------------------------- | -------------------- | -------------------------------------- |
| `GoToStep(u32)`              | step index (0-based) | Navigate to a specific step directly.  |
| `NextStep`                   | —                    | Advance to step + 1.                   |
| `PrevStep`                   | —                    | Go back to step - 1.                   |
| `CompleteStep(u32)`          | step index           | Mark a step as `Complete`.             |
| `SetStatus { step, status }` | `u32`, `Status`      | Explicitly set the status of any step. |

### 1.3 Context

```rust
use ars_core::Bindable;
use ars_i18n::Orientation;

/// Context for the `Steps` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// 0-based index of the currently active step — controlled or uncontrolled.
    pub step: Bindable<u32>,
    /// Total number of steps.
    pub count: NonZero<u32>,
    /// Per-step status (indexed 0..count).
    pub statuses: Vec<Status>,
    /// If true, users can only move forward one step at a time; skipping is disallowed.
    pub linear: bool,
    /// Visual stacking axis.
    pub orientation: Orientation,
    /// Resolved locale for i18n.
    pub locale: Locale,
    /// Unique IDs for each part.
    pub ids: ComponentIds,
    /// Resolved messages for accessibility labels.
    pub messages: Messages,
}

/// Status of a step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Status {
    /// Step has not been visited.
    Incomplete,
    /// Step is the currently active one.
    Current,
    /// Step has been successfully completed.
    Complete,
    /// Step has an error that needs resolution.
    Error,
}
```

### 1.4 Props

```rust
use ars_i18n::Orientation;

/// Props for the `Steps` component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Unique component identifier.
    pub id: String,
    /// Controlled current step (0-based).
    pub step: Option<u32>,
    /// Default step to use if `step` is `None`.
    pub default_step: u32,
    /// Total step count.
    pub count: NonZero<u32>,
    /// Initial per-step statuses (defaults to all `Incomplete` except step 0 = `Current`).
    pub statuses: Option<Vec<Status>>,
    /// Restrict navigation to sequential advancement only.
    pub linear: bool,
    /// Per-step validation callback. When `Some`, called before advancing past
    /// a step. Return `true` to allow advancement, `false` to block it.
    /// The callback receives the 0-based step index being validated.
    pub is_step_valid: Option<Callback<dyn Fn(u32) -> bool + Send + Sync>>,
    /// Per-step skip predicate. When `Some`, allows the user to skip ahead past
    /// a step without completing it. Return `true` if the step is skippable.
    pub is_step_skippable: Option<Callback<dyn Fn(u32) -> bool + Send + Sync>>,
    /// Callback fired when all steps are completed (the user advances past the last step).
    pub on_complete: Option<Callback<()>>,
    /// Visual stacking axis.
    pub orientation: Orientation,
    /// Locale for i18n message resolution.
    pub locale: Option<Locale>,
    /// Translatable messages.
    pub messages: Option<Messages>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            step: None,
            default_step: 0,
            count: NonZero::new(1).expect("non-zero"),
            statuses: None,
            linear: false,
            is_step_valid: None,
            is_step_skippable: None,
            on_complete: None,
            orientation: Orientation::Horizontal,
            locale: None,
            messages: None,
        }
    }
}
```

### 1.5 Full Machine Implementation

```rust
use ars_core::{TransitionPlan, PendingEffect, Bindable, AttrMap};
use ars_i18n::Orientation;

/// State of the `Steps` component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// The only machine state; current step is tracked in context.
    Idle,
}

/// Events for the `Steps` component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// Navigate to a specific step directly.
    GoToStep(u32),
    /// Advance to step + 1.
    NextStep,
    /// Go back to step - 1.
    PrevStep,
    /// Mark a step as `Complete`.
    CompleteStep(u32),
    /// Explicitly set the status of any step.
    SetStatus {
        /// 0-based step index.
        step: u32,
        /// New status.
        status: Status,
    },
}

/// Machine for the `Steps` component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State   = State;
    type Event   = Event;
    type Context = Context;
    type Props   = Props;
    type Api<'a> = Api<'a>;

    fn init(props: &Props) -> (State, Context) {
        let step = match props.step {
            Some(s) => Bindable::controlled(s),
            None    => Bindable::uncontrolled(props.default_step),
        };
        let count = props.count;
        let statuses = props.statuses.clone().unwrap_or_else(|| {
            (0..count.get()).map(|i| {
                if i == *step.get() { Status::Current } else { Status::Incomplete }
            }).collect()
        });
        let ids = ComponentIds::from_id(&props.id);
        let locale = resolve_locale(props.locale.as_ref());
        let messages = resolve_messages::<Messages>(props.messages.as_ref(), &locale);
        (State::Idle, Context {
            step,
            count,
            statuses,
            linear: props.linear,
            orientation: props.orientation.clone(),
            locale,
            ids,
            messages,
        })
    }

    fn transition(
        _state: &State,
        event: &Event,
        ctx: &Context,
        _props: &Props,
    ) -> Option<TransitionPlan<Self>> {
        match event {
            Event::GoToStep(target) => {
                let target = *target;
                let current = *ctx.step.get();
                let count   = ctx.count.get();
                // In linear mode, only allow advancing one step at a time or going backward,
                // UNLESS the intermediate steps are all skippable.
                if ctx.linear && target > current + 1 {
                    if let Some(ref is_skippable) = props.is_step_skippable {
                        // Check all intermediate steps are skippable.
                        let all_skippable = (current + 1..target).all(|s| is_skippable(s));
                        if !all_skippable { return None; }
                    } else {
                        return None;
                    }
                }
                if target >= count { return None; }
                if target == current { return None; }
                Some(TransitionPlan::context_only(move |ctx| {
                    // Mark the previous current step as Complete (if going forward).
                    if target > current {
                        if let Some(s) = ctx.statuses.get_mut(current as usize) {
                            *s = Status::Complete;
                        }
                    }
                    // Update status of the new current step.
                    if let Some(s) = ctx.statuses.get_mut(target as usize) {
                        *s = Status::Current;
                    }
                    ctx.step.set(target);
                }))
            }
            Event::NextStep => {
                let current = *ctx.step.get();
                let next = current + 1;

                // Validate current step before advancing (if callback configured).
                if let Some(ref is_valid) = props.is_step_valid {
                    if !is_valid(current) {
                        return None; // Block advancement if step is invalid.
                    }
                }

                // Last step → fire on_complete instead of advancing.
                if next >= ctx.count.get() {
                    return Some(TransitionPlan::context_only(move |ctx| {
                        if let Some(s) = ctx.statuses.get_mut(current as usize) {
                            *s = Status::Complete;
                        }
                    }).with_effect(PendingEffect::new("on-complete", |_ctx, props, _send| {
                        if let Some(ref cb) = props.on_complete {
                            cb.call(());
                        }
                        no_cleanup()
                    })));
                }

                Some(TransitionPlan::context_only(move |ctx| {
                    if let Some(s) = ctx.statuses.get_mut(current as usize) {
                        *s = Status::Complete;
                    }
                    if let Some(s) = ctx.statuses.get_mut(next as usize) {
                        *s = Status::Current;
                    }
                    ctx.step.set(next);
                }))
            }
            Event::PrevStep => {
                let current = *ctx.step.get();
                if current == 0 { return None; }
                let prev = current - 1;
                Some(TransitionPlan::context_only(move |ctx| {
                    if let Some(s) = ctx.statuses.get_mut(current as usize) {
                        // Going backward leaves the step as Incomplete (not wiping Complete).
                        *s = Status::Incomplete;
                    }
                    if let Some(s) = ctx.statuses.get_mut(prev as usize) {
                        *s = Status::Current;
                    }
                    ctx.step.set(prev);
                }))
            }
            Event::CompleteStep(idx) => {
                let idx = *idx as usize;
                Some(TransitionPlan::context_only(move |ctx| {
                    if let Some(s) = ctx.statuses.get_mut(idx) {
                        *s = Status::Complete;
                    }
                }))
            }
            Event::SetStatus { step, status } => {
                let idx    = *step as usize;
                let status = status.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    if let Some(s) = ctx.statuses.get_mut(idx) {
                        *s = status;
                    }
                }))
            }
        }
    }

    fn connect<'a>(
        state: &'a State,
        ctx: &'a Context,
        props: &'a Props,
        send: &'a dyn Fn(Event),
    ) -> Self::Api<'a> {
        Api { state, ctx, props, send }
    }
}
```

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "steps"]
pub enum Part {
    Root,
    List,
    Item { index: u32 },
    Indicator { index: u32 },
    Title { index: u32 },
    Description { index: u32 },
    Separator { after_index: u32 },
    Content { index: u32 },
    PrevTrigger,
    NextTrigger,
}

/// API for the `Steps` component.
pub struct Api<'a> {
    /// Current machine state.
    state: &'a State,
    /// Current context.
    ctx:   &'a Context,
    /// Current props.
    props: &'a Props,
    /// Event dispatcher.
    send:  &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Get the current step index.
    pub fn current_step(&self) -> u32 { *self.ctx.step.get() }

    /// Get the total number of steps.
    pub fn step_count(&self)   -> u32 { self.ctx.count.get() }

    /// Check if the current step is the first step.
    pub fn is_first_step(&self) -> bool { self.current_step() == 0 }

    /// Check if the current step is the last step.
    pub fn is_last_step(&self)  -> bool { self.current_step() == self.ctx.count.get().saturating_sub(1) }

    /// Get the status of a specific step.
    pub fn step_status(&self, idx: u32) -> Option<&Status> { self.ctx.statuses.get(idx as usize) }

    /// Convert a status to an attribute value.
    fn state_attr(status: &Status) -> &'static str {
        match status {
            Status::Incomplete => "incomplete",
            Status::Current    => "current",
            Status::Complete   => "complete",
            Status::Error      => "error",
        }
    }

    /// Attrs for the root wrapper.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-orientation"), match self.ctx.orientation {
            Orientation::Horizontal => "horizontal",
            Orientation::Vertical   => "vertical",
        });
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.root_label)(&self.ctx.locale));
        attrs
    }

    /// Attrs for the list container that holds all step items.
    pub fn list_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::List.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "list");
        attrs
    }

    /// Attrs for an individual step item.
    ///
    /// `index` — 0-based step index.
    pub fn item_attrs(&self, index: u32) -> AttrMap {
        let mut attrs = AttrMap::new();
        let status = self.ctx.statuses.get(index as usize)
            .unwrap_or(&Status::Incomplete);
        let is_current = *self.ctx.step.get() == index;
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Item { index }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-index"), index.to_string());
        attrs.set(HtmlAttr::Data("ars-state"), Self::state_attr(status));
        attrs.set(HtmlAttr::Role, "listitem");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label),
            (self.ctx.messages.step_label)(index as usize + 1, self.ctx.count.get() as usize, &self.ctx.locale));
        if is_current {
            attrs.set(HtmlAttr::Aria(AriaAttr::Current), "step");
        }
        attrs
    }

    /// Attrs for the step indicator (number badge or icon).
    pub fn indicator_attrs(&self, index: u32) -> AttrMap {
        let mut attrs = AttrMap::new();
        let status = self.ctx.statuses.get(index as usize)
            .unwrap_or(&Status::Incomplete);
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Indicator { index }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), Self::state_attr(status));
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }

    /// Attrs for the step title text.
    pub fn title_attrs(&self, index: u32) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Title { index }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-index"), index.to_string());
        attrs
    }

    /// Attrs for the step description text.
    pub fn description_attrs(&self, index: u32) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Description { index }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-index"), index.to_string());
        attrs
    }

    /// Attrs for the separator line between steps.
    pub fn separator_attrs(&self, after_index: u32) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Separator { after_index }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-index"), after_index.to_string());
        // Mark completed separators so the adapter can style the progress trail.
        let completed = after_index < *self.ctx.step.get();
        attrs.set_bool(HtmlAttr::Data("ars-completed"), completed);
        attrs.set(HtmlAttr::Role, "separator");
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }

    /// Attrs for the content panel associated with a specific step.
    ///
    /// Only the current step's content is visible.
    pub fn content_attrs(&self, index: u32) -> AttrMap {
        let mut attrs = AttrMap::new();
        let is_current = *self.ctx.step.get() == index;
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content { index: Default::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-index"), index.to_string());
        if !is_current {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }
        attrs
    }

    /// Attrs for the "previous step" trigger button.
    pub fn prev_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let disabled = self.is_first_step();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::PrevTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        if disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }
        attrs
    }

    /// Handle click event for the "previous step" trigger button.
    pub fn on_prev_trigger_click(&self) {
        if !self.is_first_step() { (self.send)(Event::PrevStep); }
    }

    /// Attrs for the "next step" trigger button.
    pub fn next_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let disabled = self.is_last_step();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::NextTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        if disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }
        attrs
    }

    /// Handle click event for the "next step" trigger button.
    pub fn on_next_trigger_click(&self) {
        if !self.is_last_step() { (self.send)(Event::NextStep); }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::List => self.list_attrs(),
            Part::Item { index } => self.item_attrs(index),
            Part::Indicator { index } => self.indicator_attrs(index),
            Part::Title { index } => self.title_attrs(index),
            Part::Description { index } => self.description_attrs(index),
            Part::Separator { after_index } => self.separator_attrs(after_index),
            Part::Content { index } => self.content_attrs(index),
            Part::PrevTrigger => self.prev_trigger_attrs(),
            Part::NextTrigger => self.next_trigger_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
Steps
├── Root
├── List (steps row or column)
│   ├── Item (×N)              data-ars-state=current/complete/incomplete/error
│   │   ├── Indicator          aria-hidden="true"
│   │   ├── Title
│   │   └── Description
│   └── Separator (×N-1)       aria-hidden="true"
├── Content (×N)               hidden when not current step
├── PrevTrigger
└── NextTrigger
```

| Part          | Element           | Key Attributes                                                                                                                                                                       |
| ------------- | ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `Root`        | `<div>`           | `data-ars-scope="steps"`, `data-ars-part="root"`, `data-ars-orientation`, `aria-label="Steps"`                                                                                       |
| `List`        | `<ol>` or `<div>` | `data-ars-scope="steps"`, `data-ars-part="list"`, `role="list"`                                                                                                                      |
| `Item`        | `<li>` or `<div>` | `data-ars-scope="steps"`, `data-ars-part="item"`, `data-ars-index`, `data-ars-state="current\|complete\|incomplete\|error"`, `role="listitem"`, `aria-current="step"` (current only) |
| `Indicator`   | `<span>`          | `data-ars-scope="steps"`, `data-ars-part="indicator"`, `data-ars-state`, `aria-hidden="true"`                                                                                        |
| `Title`       | `<span>`          | `data-ars-scope="steps"`, `data-ars-part="title"`, `data-ars-index`                                                                                                                  |
| `Description` | `<span>`          | `data-ars-scope="steps"`, `data-ars-part="description"`, `data-ars-index`                                                                                                            |
| `Separator`   | `<div>`           | `data-ars-scope="steps"`, `data-ars-part="separator"`, `role="separator"`, `aria-hidden="true"`                                                                                      |
| `Content`     | `<div>`           | `data-ars-scope="steps"`, `data-ars-part="content"`, `data-ars-index`, `hidden` when not current                                                                                     |
| `PrevTrigger` | `<button>`        | `data-ars-scope="steps"`, `data-ars-part="prev-trigger"`, `disabled`, `data-ars-disabled`                                                                                            |
| `NextTrigger` | `<button>`        | `data-ars-scope="steps"`, `data-ars-part="next-trigger"`, `disabled`, `data-ars-disabled`                                                                                            |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part        | Role             | Properties                                                                                              |
| ----------- | ---------------- | ------------------------------------------------------------------------------------------------------- |
| `Root`      | (none / `<div>`) | `aria-label="Steps"` (localized)                                                                        |
| `List`      | `list`           | —                                                                                                       |
| `Item`      | `listitem`       | `aria-current="step"` for the current item                                                              |
| `Indicator` | (none)           | `aria-hidden="true"` (visual only; step status is conveyed via `aria-current` and visually hidden text) |

When the stepper is interactive and users can click any step, the indicators should use
`role="tab"` and the content panels `role="tabpanel"` instead of the list pattern, providing
the same keyboard navigation as the `Tabs` component.

### 3.2 Keyboard Interaction

When interactive (tabs mode):

| Key                        | Behavior                                                                   |
| -------------------------- | -------------------------------------------------------------------------- |
| `ArrowRight` / `ArrowDown` | Move focus to next step indicator.                                         |
| `ArrowLeft` / `ArrowUp`    | Move focus to previous step indicator.                                     |
| `Enter` / `Space`          | Navigate to the focused step (if `linear=false` or target <= current + 1). |
| `Home`                     | Focus the first step indicator.                                            |
| `End`                      | Focus the last step indicator.                                             |

`PrevTrigger` and `NextTrigger` buttons use standard keyboard activation (Enter/Space).

## 4. Internationalization

```rust
#[derive(Clone, Debug)]
pub struct Messages {
    /// Root label (default: "Steps")
    pub root_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Step label template (default: "Step {current} of {total}")
    pub step_label: MessageFn<dyn Fn(usize, usize, &Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            root_label: MessageFn::static_str("Steps"),
            step_label: MessageFn::new(|current, total, _locale| format!("Step {} of {}", current, total)),
        }
    }
}

impl ComponentMessages for Messages {}
```

- **Status text**: `Status` variants map to translation keys: `steps.status.incomplete`,
  `steps.status.current`, `steps.status.complete`, `steps.status.error`. These are rendered
  as visually hidden text inside `Indicator` for screen readers.
- **"Step N of M"**: The `aria-label` on the root or item uses `Messages.step_label`.
- **Orientation**: Vertical orientation is common for mobile; the machine is orientation-agnostic.
- **RTL**: Horizontal separators and visual order reverse in RTL. The machine emits no
  direction-specific markup; CSS handles the visual flip.

## 5. Library Parity

> Compared against: Ark UI (`Steps`).

### 5.1 Props

| Feature         | ars-ui              | Ark UI            | Notes      |
| --------------- | ------------------- | ----------------- | ---------- |
| Step count      | `count`             | `count`           | Full match |
| Controlled step | `step`              | `step`            | Full match |
| Default step    | `default_step`      | `defaultStep`     | Full match |
| Linear mode     | `linear`            | `linear`          | Full match |
| Orientation     | `orientation`       | `orientation`     | Full match |
| Step validation | `is_step_valid`     | `isStepValid`     | Full match |
| Step skippable  | `is_step_skippable` | `isStepSkippable` | Full match |
| On complete     | `on_complete`       | --                | See events |

**Gaps:** None. ars-ui covers all Ark UI props.

### 5.2 Anatomy

| Part              | ars-ui        | Ark UI             | Notes                                  |
| ----------------- | ------------- | ------------------ | -------------------------------------- |
| Root              | `Root`        | `Root`             | Full match                             |
| List              | `List`        | `List`             | Full match                             |
| Item              | `Item`        | `Item`             | Full match                             |
| Indicator         | `Indicator`   | `Indicator`        | Full match                             |
| Title             | `Title`       | --                 | ars-ui addition; Ark uses Trigger text |
| Description       | `Description` | --                 | ars-ui addition                        |
| Separator         | `Separator`   | `Separator`        | Full match                             |
| Content           | `Content`     | `Content`          | Full match                             |
| Completed content | --            | `CompletedContent` | See below                              |
| Prev trigger      | `PrevTrigger` | `PrevTrigger`      | Full match                             |
| Next trigger      | `NextTrigger` | `NextTrigger`      | Full match                             |
| Trigger           | --            | `Trigger`          | See below                              |

**Gaps:**

- **`CompletedContent`**: Ark UI has a `CompletedContent` part displayed after all steps are done. ars-ui fires an `on_complete` callback instead, letting the consumer render completion UI. Not a functional gap -- the consumer controls completion rendering.
- **`Trigger`**: Ark UI has a clickable `Trigger` part on each step item for direct navigation. ars-ui's `Item` part can be made clickable by the consumer sending `GoToStep(index)`. Adding a dedicated `Trigger` part would improve ergonomics.

### 5.3 Events

| Callback      | ars-ui              | Ark UI           | Notes                        |
| ------------- | ------------------- | ---------------- | ---------------------------- |
| Step change   | `Bindable` onChange | `onStepChange`   | ars-ui uses Bindable pattern |
| Step complete | `on_complete`       | `onStepComplete` | Full match                   |
| Step invalid  | --                  | `onStepInvalid`  | See below                    |

**Gaps:**

- **`onStepInvalid`**: Ark UI fires `onStepInvalid` when advancement is blocked by validation. ars-ui's `is_step_valid` callback returns `false` to block, but does not fire a separate event. The consumer can detect this by checking `is_step_valid` themselves before calling `NextStep`. Low priority.

### 5.4 Features

| Feature                  | ars-ui                                  | Ark UI                    |
| ------------------------ | --------------------------------------- | ------------------------- | --------- |
| Sequential (linear) mode | Yes                                     | Yes                       |
| Step validation          | Yes                                     | Yes                       |
| Step skippability        | Yes                                     | Yes                       |
| Completion callback      | Yes                                     | Yes                       |
| Per-step status tracking | Yes (Incomplete/Current/Complete/Error) | Yes (via data attributes) |
| Orientation              | Yes                                     | Yes                       |
| Completion percentage    | --                                      | `--percent` CSS var       | See below |
| Completed content        | Via consumer rendering                  | `CompletedContent` part   |
| Reset                    | Via `GoToStep(0)` + status reset        | `resetStep` in context    |

**Gaps:**

- **`--percent` CSS variable**: Ark UI exposes a `--percent` CSS variable on the root for progress visualization. This is a derived value (`current_step / count * 100`). Worth adding as a convenience.

### 5.5 Summary

- **Overall:** Full parity.
- **Divergences:** ars-ui uses `Title`/`Description` parts instead of Ark's `Trigger` for step items. ars-ui fires `on_complete` callback instead of rendering a `CompletedContent` part. ars-ui uses explicit `Status` enum tracking instead of CSS-variable-based progress.
- **Recommended additions:** None for v1. A `--ars-percent` CSS custom property on the Root element would be a minor ergonomic addition for progress bar styling.
