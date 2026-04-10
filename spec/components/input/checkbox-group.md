---
component: CheckboxGroup
category: input
tier: stateful
foundation_deps: [architecture, accessibility, interactions, forms]
shared_deps: []
related: [checkbox]
references:
  react-aria: CheckboxGroup
---

# CheckboxGroup

`CheckboxGroup` manages a group of `Checkbox` components with shared `name` attribute for form submission, group-level `value: BTreeSet<Key>` tracking which checkboxes are checked, propagation of `disabled`, `required`, `readOnly`, `invalid` from group to children, and automatic parent indeterminate computation when a parent checkbox represents the group state.

> Cross-references: Equivalent to React Aria `useCheckboxGroup` / `useCheckboxGroupItem`, Ark-UI checkbox group pattern.

## 1. State Machine

### 1.1 States

```rust
/// CheckboxGroup is context-only — it provides shared state to child Checkbox
/// components but has no state transitions of its own. `State::Idle` exists
/// solely to satisfy the state machine trait bound.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    Idle,
}
```

### 1.2 Events

```rust
/// Events for the CheckboxGroup component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Toggle a single checkbox value. If present, remove it; if absent, add it.
    Toggle(Key),
    /// Set a specific checkbox value to checked.
    Check(Key),
    /// Set a specific checkbox value to unchecked.
    Uncheck(Key),
    /// Replace the entire set of checked values.
    SetValue(BTreeSet<Key>),
    /// Check all checkboxes (using `all_values` from props).
    CheckAll,
    /// Uncheck all checkboxes.
    UncheckAll,
    /// Focus received.
    Focus { is_keyboard: bool },
    /// Focus lost.
    Blur,
}
```

### 1.3 Context

```rust
/// Context for the CheckboxGroup component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The set of currently checked checkbox values — controlled or uncontrolled.
    pub value: Bindable<BTreeSet<Key>>,
    /// Shared form field name for all checkboxes in the group.
    pub name: Option<String>,
    /// Whether the group is disabled.
    pub disabled: bool,
    /// Whether the group is required (at least one must be checked).
    pub required: bool,
    /// Whether the group is read-only.
    pub readonly: bool,
    /// Whether the group is invalid.
    pub invalid: bool,
    /// Layout direction for RTL support.
    pub dir: Direction,
    /// Layout orientation — vertical or horizontal.
    pub orientation: Orientation,
    /// Maximum number of checkboxes that can be checked simultaneously.
    pub max_checked: Option<usize>,
    /// Whether the group currently has focus.
    pub focused: bool,
    /// True when focus came from keyboard.
    pub focus_visible: bool,
    /// Whether a Description part is rendered (gates aria-describedby).
    pub has_description: bool,
    /// Whether an ErrorMessage part is rendered.
    pub has_error_message: bool,
    /// Component IDs for part identification.
    pub ids: ComponentIds,
}

impl Context {
    /// Returns the `checkbox::State` for a parent checkbox that represents
    /// the entire group. This enables the "select all" pattern.
    pub fn parent_checked_state(&self, all_values: &BTreeSet<Key>) -> checkbox::State {
        if all_values.is_empty() {
            return checkbox::State::Unchecked;
        }
        let checked = self.value.get();
        let checked_count = all_values.iter()
            .filter(|v| checked.contains(*v))
            .count();
        if checked_count == 0 {
            checkbox::State::Unchecked
        } else if checked_count == all_values.len() {
            checkbox::State::Checked
        } else {
            checkbox::State::Indeterminate
        }
    }

    /// Returns whether a specific value is currently checked.
    pub fn is_checked(&self, value: &Key) -> bool {
        self.value.get().contains(value)
    }

    /// Returns true when the maximum number of checked items has been reached.
    pub fn is_at_max(&self) -> bool {
        match self.max_checked {
            Some(max) => self.value.get().len() >= max,
            None => false,
        }
    }
}
```

### 1.4 Props

```rust
/// Props for the CheckboxGroup component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    pub id: String,
    /// Controlled checked values. When Some, component is controlled.
    pub value: Option<BTreeSet<Key>>,
    /// Default checked values for uncontrolled mode.
    pub default_value: BTreeSet<Key>,
    /// Shared form field name for all checkboxes in the group.
    pub name: Option<String>,
    /// The ID of the form element the input is associated with.
    pub form: Option<String>,
    /// Whether all checkboxes in the group are disabled.
    pub disabled: bool,
    /// Whether at least one checkbox must be checked for validity.
    pub required: bool,
    /// Whether the group is read-only.
    pub readonly: bool,
    /// Whether the group is in an invalid state.
    pub invalid: bool,
    /// Layout direction for RTL support.
    pub dir: Direction,
    /// Layout orientation.
    pub orientation: Orientation,
    /// All possible checkbox values — used for CheckAll and parent indeterminate computation.
    pub all_values: BTreeSet<Key>,
    /// Maximum number of checkboxes that can be checked simultaneously.
    /// When `Some(n)`, unchecked items become disabled once `n` items are checked.
    /// Client-side constraint only — no hidden input is emitted for this prop.
    pub max_checked: Option<usize>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: BTreeSet::new(),
            name: None,
            form: None,
            disabled: false, required: false, readonly: false, invalid: false,
            dir: Direction::Ltr,
            orientation: Orientation::Vertical,
            all_values: BTreeSet::new(),
            max_checked: None,
        }
    }
}
```

### 1.5 Guards

```rust
fn is_disabled(ctx: &Context) -> bool { ctx.disabled }
fn is_readonly(ctx: &Context) -> bool { ctx.readonly }
fn is_at_max(ctx: &Context) -> bool { ctx.is_at_max() }
```

### 1.6 Child Checkbox Integration

When a `Checkbox` is rendered inside a `CheckboxGroup`, it reads the `ChildContext` and:

1. **Derives its checked state** from `child_ctx.value.contains(&self.key)` instead of its own `checkbox::State`
2. **Inherits disabled** from `child_ctx.disabled || self.disabled`
3. **Inherits readonly** from `child_ctx.readonly`
4. **Uses the group name** from `child_ctx.name` for form submission
5. **On toggle**, sends `checkbox_group::Event::Toggle(self.key.clone())` to the group instead of toggling its own state
6. **At max**: when `child_ctx.at_max` is true and the item is unchecked, the child sets `aria-disabled="true"` and rejects toggle attempts

```rust
/// Borrowed view of group context for child Checkbox components.
pub struct ChildContext<'a> {
    pub value: &'a BTreeSet<Key>,
    pub name: Option<&'a str>,
    pub form: Option<&'a str>,
    pub disabled: bool,
    pub readonly: bool,
    pub invalid: bool,
    /// True when `max_checked` is reached. Unchecked child checkboxes should
    /// set `aria-disabled="true"` and reject toggle attempts.
    pub at_max: bool,
}
```

`CheckboxGroup` itself does not add keyboard interactions beyond what individual Checkbox components provide. Each checkbox responds to `Space` to toggle. Tab moves between checkboxes per standard tab order.

### 1.7 Full Machine Implementation

```rust
/// Machine for the CheckboxGroup component.
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

    fn init(props: &Self::Props, _env: &Env, _messages: &Self::Messages) -> (Self::State, Self::Context) {
        let ctx = Context {
            value: match &props.value {
                Some(v) => Bindable::controlled(v.clone()),
                None => Bindable::uncontrolled(props.default_value.clone()),
            },
            name: props.name.clone(),
            disabled: props.disabled,
            required: props.required,
            readonly: props.readonly,
            invalid: props.invalid,
            dir: props.dir,
            orientation: props.orientation,
            max_checked: props.max_checked,
            focused: false,
            focus_visible: false,
            has_description: false,
            has_error_message: false,
            ids: ComponentIds::from_id(&props.id),
        };
        (State::Idle, ctx)
    }

    fn transition(
        _state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if is_disabled(ctx) || is_readonly(ctx) {
            match event {
                Event::Toggle(_) | Event::Check(_) | Event::Uncheck(_)
                | Event::SetValue(_) | Event::CheckAll | Event::UncheckAll => return None,
                _ => {}
            }
        }

        match event {
            Event::Toggle(v) => {
                let v = v.clone();
                let is_adding = !ctx.value.get().contains(&v);
                if is_adding && is_at_max(ctx) { return None; }
                Some(TransitionPlan::context_only(move |ctx| {
                    let mut checked = ctx.value.get().clone();
                    if !checked.remove(&v) { checked.insert(v); }
                    ctx.value.set(checked);
                }))
            }
            Event::Check(v) => {
                if !ctx.value.get().contains(v) && is_at_max(ctx) { return None; }
                let v = v.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    let mut checked = ctx.value.get().clone();
                    checked.insert(v);
                    ctx.value.set(checked);
                }))
            }
            Event::Uncheck(v) => {
                let v = v.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    let mut checked = ctx.value.get().clone();
                    checked.remove(&v);
                    ctx.value.set(checked);
                }))
            }
            Event::SetValue(v) => {
                let v = v.clone();
                Some(TransitionPlan::context_only(move |ctx| { ctx.value.set(v); }))
            }
            Event::CheckAll => {
                let all = props.all_values.clone();
                let max = ctx.max_checked;
                Some(TransitionPlan::context_only(move |ctx| {
                    match max {
                        Some(n) => {
                            let truncated = all.into_iter().take(n).collect::<BTreeSet<_>>();
                            ctx.value.set(truncated);
                        }
                        None => ctx.value.set(all),
                    }
                }))
            }
            Event::UncheckAll => {
                Some(TransitionPlan::context_only(|ctx| { ctx.value.set(BTreeSet::new()); }))
            }
            Event::Focus { is_keyboard } => {
                let is_kb = *is_keyboard;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focused = true;
                    ctx.focus_visible = is_kb;
                }))
            }
            Event::Blur => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.focused = false;
                    ctx.focus_visible = false;
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
}
```

### 1.8 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "checkbox-group"]
pub enum Part {
    Root,
    Label,
    Description,
    ErrorMessage,
}

/// API for the CheckboxGroup component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Attributes for the root group container.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "group");
        attrs.set(HtmlAttr::Id, self.ctx.ids.id());
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), self.ctx.ids.part("label"));
        attrs.set(HtmlAttr::Aria(AriaAttr::Orientation), match self.ctx.orientation {
            Orientation::Horizontal => "horizontal",
            Orientation::Vertical => "vertical",
        });
        if self.ctx.has_description {
            let mut describedby_parts = Vec::new();
            describedby_parts.push(self.ctx.ids.part("description"));
            if self.ctx.invalid && self.ctx.has_error_message {
                describedby_parts.push(self.ctx.ids.part("error-message"));
            }
            attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), describedby_parts.join(" "));
        } else if self.ctx.invalid && self.ctx.has_error_message {
            attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), self.ctx.ids.part("error-message"));
        }
        if self.ctx.invalid { attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true"); }
        if self.ctx.disabled { attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true"); }
        attrs.set(HtmlAttr::Dir, self.ctx.dir.as_str());
        if self.ctx.focus_visible { attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true); }
        if self.ctx.is_at_max() { attrs.set_bool(HtmlAttr::Data("ars-at-max"), true); }
        attrs
    }

    /// Attributes for the group label.
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("label"));
        attrs
    }

    /// Attributes for the description/help text.
    pub fn description_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Description.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("description"));
        attrs
    }

    /// Attributes for the error message element.
    pub fn error_message_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ErrorMessage.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("error-message"));
        attrs.set(HtmlAttr::Role, "alert");
        // role="alert" implies aria-live="assertive" — do NOT set explicit aria-live
        // to avoid double-announcement on NVDA+Firefox (see 07-forms.md §9.3).
        attrs
    }

    /// Context for child Checkbox components to read.
    pub fn child_context(&self) -> ChildContext<'_> {
        ChildContext {
            value: self.ctx.value.get(),
            name: self.ctx.name.as_deref(),
            form: self.props.form.as_deref(),
            disabled: self.ctx.disabled,
            readonly: self.ctx.readonly,
            invalid: self.ctx.invalid,
            at_max: self.ctx.is_at_max(),
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
CheckboxGroup
├── Root               <div>    data-ars-scope="checkbox-group" data-ars-part="root" (role="group")
├── Label              <span>   data-ars-part="label"
├── Checkbox (×N)      _(child slot)_ — individual Checkbox components read ChildContext
├── Description        <span>   data-ars-part="description" (optional)
└── ErrorMessage       <span>   data-ars-part="error-message" (optional, role="alert")
```

| Part         | Element        | Key Attributes                                                |
| ------------ | -------------- | ------------------------------------------------------------- |
| Root         | `<div>`        | `role="group"`, `aria-labelledby`, `aria-orientation`         |
| Label        | `<span>`       | Group label                                                   |
| Description  | `<span>`       | Help text; linked via `aria-describedby` (optional)           |
| ErrorMessage | `<span>`       | `role="alert"` — do NOT add `aria-live` (implicit) (optional) |
| Checkbox     | _(child slot)_ | Individual Checkbox components read `ChildContext`            |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property           | Element      | Value                                                     |
| ------------------ | ------------ | --------------------------------------------------------- |
| `role`             | Root         | `group`                                                   |
| `aria-labelledby`  | Root         | Points to Label id                                        |
| `aria-describedby` | Root         | Points to Description + ErrorMessage ids (when present)   |
| `aria-invalid`     | Root         | Present when `invalid=true`                               |
| `aria-disabled`    | Root         | Present when `disabled=true`                              |
| `aria-orientation` | Root         | `"horizontal"` or `"vertical"`                            |
| `role`             | ErrorMessage | `alert` — implies `aria-live="assertive"`                 |
| `aria-disabled`    | Checkbox     | `"true"` on unchecked items when `max_checked` is reached |
| `data-ars-at-max`  | Root         | Present when `max_checked` limit is reached               |

> **Note on `aria-required`**: Per WAI-ARIA 1.2, `aria-required` is not a supported property on `role="group"`. The "required" constraint is communicated by: (1) appending a visually-hidden "required" indicator to the Label text, and (2) validation logic that rejects empty selections when `props.required == true`.
>
> **Note on `max_checked`**: When the maximum number of checked items is reached, unchecked checkboxes receive `aria-disabled="true"` to communicate that they cannot be selected. Already-checked items remain enabled so users can uncheck them to free a slot. The group Root receives `data-ars-at-max` for CSS styling hooks.

### 3.2 Keyboard Interaction

| Key   | Action                               |
| ----- | ------------------------------------ |
| Space | Toggle individual checkbox           |
| Tab   | Move between checkboxes in the group |

## 4. Internationalization

- **RTL**: The `dir` attribute is set on the Root element. In RTL mode, the visual order of checkbox glyph vs. label text flips. CSS must respect `[dir="rtl"]` for correct layout.
- **Orientation**: `aria-orientation` on Root communicates layout direction to screen readers.
- **Bidirectional text**: Group label and description text should use Unicode directional isolate characters (`U+2068` / `U+2069`) when `dir` is `Auto` and text may contain mixed-direction content.
- **Error messages**: Localizable via `ValidationError`. User input in error messages must be wrapped in Unicode isolates to prevent BiDi reordering.
- **Checkbox values are opaque identifiers**: The `value` keys in `BTreeSet<Key>` are machine-readable identifiers, not display labels. Display labels are consumer-provided and locale-independent.

## 5. Form Integration

- **Hidden inputs**: The group renders hidden `<input type="checkbox" name="{name}" value="{v}" checked>` for each checked value.
- **Validation states**: `aria-invalid="true"` on Root when `invalid=true`. The `ErrorMessage` part uses `role="alert"` for screen reader announcement.
- **Required validation**: At least one checkbox must be checked when `required == true`.
- **Reset behavior**: On form reset, the adapter restores `value` to `default_value`.
- **Disabled/readonly propagation**: When inside a `Field` or `Fieldset`, the adapter merges `disabled`/`readonly` from `FieldCtx` per `07-forms.md` §12.6. The group then propagates these to child checkboxes via `ChildContext`.
- **`max_checked` is client-side only**: No hidden input is emitted for `max_checked`. Server-side validation must independently enforce any maximum selection constraint.
- **Server-side errors**: Can target the group via its registered field name.

## 6. Library Parity

> Compared against: Ark UI (`Checkbox` Group), React Aria (`CheckboxGroup`).

### 6.1 Props

| Feature          | ars-ui                         | Ark UI              | React Aria                        | Notes                                 |
| ---------------- | ------------------------------ | ------------------- | --------------------------------- | ------------------------------------- |
| Controlled value | `value: Option<BTreeSet<Key>>` | `value: string[]`   | `value: string[]`                 | Full parity                           |
| Default value    | `default_value: BTreeSet<Key>` | `defaultValue`      | `defaultValue`                    | Full parity                           |
| Disabled         | `disabled: bool`               | `disabled`          | `isDisabled`                      | Full parity                           |
| Read-only        | `readonly: bool`               | `readOnly`          | `isReadOnly`                      | Full parity                           |
| Required         | `required: bool`               | --                  | `isRequired`                      | RA parity                             |
| Invalid          | `invalid: bool`                | `invalid`           | `isInvalid`                       | Full parity                           |
| Form name        | `name: Option<String>`         | `name`              | `name`                            | Full parity                           |
| Form ID          | `form: Option<String>`         | --                  | `form`                            | RA parity                             |
| Max selected     | `max_checked: Option<usize>`   | `maxSelectedValues` | --                                | Ark parity                            |
| Orientation      | `orientation: Orientation`     | --                  | `orientation`                     | RA parity                             |
| All values       | `all_values: BTreeSet<Key>`    | --                  | --                                | ars-ui specific for CheckAll          |
| Direction        | `dir: Direction`               | --                  | --                                | ars-ui specific for RTL               |
| Validation       | --                             | --                  | `validate` / `validationBehavior` | Intentional: ars-ui defers to adapter |

**Gaps:** None. React Aria's `validate`/`validationBehavior` props are handled at the adapter layer in ars-ui per forms spec.

### 6.2 Anatomy

| Part             | ars-ui             | Ark UI              | React Aria          | Notes       |
| ---------------- | ------------------ | ------------------- | ------------------- | ----------- |
| Root             | `Root`             | `Group`             | `CheckboxGroup`     | Full parity |
| Label            | `Label`            | --                  | `Label`             | Full parity |
| Description      | `Description`      | --                  | `Text[description]` | Full parity |
| ErrorMessage     | `ErrorMessage`     | --                  | `FieldError`        | Full parity |
| Child checkboxes | via `ChildContext` | via `Group` wrapper | via context         | Full parity |

**Gaps:** None.

### 6.3 Events

| Callback      | ars-ui        | Ark UI          | React Aria | Notes              |
| ------------- | ------------- | --------------- | ---------- | ------------------ |
| Value changed | `Toggle(Key)` | `onValueChange` | `onChange` | Full parity        |
| Check all     | `CheckAll`    | --              | --         | ars-ui enhancement |
| Uncheck all   | `UncheckAll`  | --              | --         | ars-ui enhancement |

**Gaps:** None.

### 6.4 Features

| Feature              | ars-ui                       | Ark UI | React Aria |
| -------------------- | ---------------------------- | ------ | ---------- |
| Group-level disabled | Yes                          | Yes    | Yes        |
| Max selection limit  | Yes                          | Yes    | --         |
| Parent indeterminate | Yes (`parent_checked_state`) | --     | --         |
| Orientation          | Yes                          | --     | Yes        |
| Form integration     | Yes                          | Yes    | Built-in   |

**Gaps:** None.

### 6.5 Summary

- **Overall:** Full parity with both reference libraries.
- **Divergences:** ars-ui uses `BTreeSet<Key>` instead of `string[]` for type safety. ars-ui includes `CheckAll`/`UncheckAll` events and `parent_checked_state()` for "select all" pattern that neither reference provides out of the box.
- **Recommended additions:** None.
