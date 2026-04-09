---
component: AlertDialog
category: overlay
tier: stateful
foundation_deps: [architecture, accessibility, interactions]
shared_deps: [z-index-stacking]
related: [dialog]
references:
  ark-ui: Dialog
  radix-ui: AlertDialog
  react-aria: Dialog
---

# AlertDialog

`AlertDialog` is `Dialog` with stricter interaction rules for destructive/critical confirmations. It reuses Dialog's state machine entirely (see [Dialog §1](./dialog.md#1-state-machine)) with the following overrides: no backdrop dismiss, no Escape dismiss, and initial focus on the cancel action.

## 1. State Machine

### 1.1 States

See [Dialog §1.1 States](./dialog.md#11-states). AlertDialog uses the same `State` enum (`Closed`, `Open`).

### 1.2 Events

See [Dialog §1.2 Events](./dialog.md#12-events). AlertDialog uses the same `Event` enum.

### 1.3 Context

See [Dialog §1.3 Context](./dialog.md#13-context). AlertDialog uses the same `Context` struct with `role` set to `Role::AlertDialog`.

### 1.4 Props

AlertDialog extends Dialog's Props with `is_destructive` and overrides several defaults:

```rust
/// The props of the alert dialog.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// All fields from Dialog Props (see Dialog §1.4).
    /// The following fields have different defaults for AlertDialog.
    pub id: String,
    pub open: Option<bool>,
    pub default_open: bool,
    pub modal: bool,
    pub close_on_backdrop: bool,
    pub close_on_escape: bool,
    pub prevent_scroll: bool,
    pub restore_focus: bool,
    pub initial_focus: Option<FocusTarget>,
    pub final_focus: Option<FocusTarget>,
    pub role: dialog::Role,
    pub title_level: u8,
    pub lazy_mount: bool,
    pub unmount_on_exit: bool,
    /// Whether the primary action is destructive (e.g., delete, remove).
    /// When true, the action button gets `data-ars-destructive` attribute for styling.
    pub is_destructive: bool,
    /// Callback invoked when the alert dialog open state changes.
    pub on_open_change: Option<Callback<bool>>,
    /// Callback invoked when Escape is pressed (only relevant if `close_on_escape` is overridden to `true`).
    /// See Dialog §1.4 for `PreventableEvent` semantics.
    pub on_escape_key_down: Option<Callback<dialog::PreventableEvent>>,
    /// Callback invoked when interaction occurs outside the alert dialog content.
    /// See Dialog §1.4 for `PreventableEvent` semantics.
    pub on_interact_outside: Option<Callback<dialog::PreventableEvent>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            open: None,
            default_open: false,
            role: dialog::Role::AlertDialog,
            close_on_backdrop: false,   // Cannot dismiss by clicking outside
            close_on_escape: false,     // Cannot dismiss with Escape
            modal: true,
            prevent_scroll: true,
            restore_focus: true,
            initial_focus: None,        // Adapter should default to CancelTrigger
            final_focus: None,
            title_level: 2,
            lazy_mount: false,
            unmount_on_exit: false,
            is_destructive: false,
            on_open_change: None,
            on_escape_key_down: None,
            on_interact_outside: None,
        }
    }
}
```

### 1.5 Full Machine Implementation

AlertDialog delegates entirely to Dialog's Machine. See [Dialog §1.9 Full Machine Implementation](./dialog.md#19-full-machine-implementation). The only difference is the Props defaults shown in §1.4 above.

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "alert-dialog"]
pub enum Part {
    Root,
    Trigger,
    Backdrop,
    Positioner,
    Content,
    Title,
    Description,
    CancelTrigger,
    ActionTrigger,
    CloseTrigger,
}

/// The API for the `AlertDialog` component.
/// Wraps Dialog's Api with additional parts for cancel/action buttons.
pub struct Api<'a> {
    /// The inner Dialog API.
    inner: dialog::Api<'a>,
    /// The props of the alert dialog.
    props: &'a Props,
    /// The current locale for message resolution.
    locale: &'a Locale,
    /// Resolved messages for accessibility labels.
    messages: Messages,
}

impl<'a> Api<'a> {
    /// Whether the alert dialog is open.
    pub fn is_open(&self) -> bool { self.inner.is_open() }

    /// The attributes for the root element.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), if self.is_open() { "open" } else { "closed" });
        attrs
    }

    /// The attributes for the trigger element.
    pub fn trigger_attrs(&self) -> AttrMap {
        // Delegates to Dialog trigger_attrs but with alert-dialog scope
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::HasPopup), "dialog");
        attrs.set(HtmlAttr::Aria(AriaAttr::Expanded), if self.is_open() { "true" } else { "false" });
        attrs
    }

    /// The attributes for the backdrop element.
    pub fn backdrop_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Backdrop.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), if self.is_open() { "open" } else { "closed" });
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs.set(HtmlAttr::Inert, "");
        attrs
    }

    /// The attributes for the positioner element.
    pub fn positioner_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Positioner.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// The attributes for the content element.
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = self.inner.content_attrs();
        // Override scope to alert-dialog
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// The attributes for the title element.
    pub fn title_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Title.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// The attributes for the description element.
    pub fn description_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Description.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// The attributes for the cancel trigger (the "safe" action — Cancel/No).
    /// This button **receives initial focus**.
    pub fn cancel_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::CancelTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.messages.cancel_label)(self.locale));
        attrs
    }

    /// The attributes for the action trigger (the destructive action — Delete/Confirm).
    pub fn action_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ActionTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.messages.confirm_label)(self.locale));
        if self.props.is_destructive {
            attrs.set_bool(HtmlAttr::Data("ars-destructive"), true);
        }
        attrs
    }

    /// The attributes for the close trigger element.
    pub fn close_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::CloseTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Trigger => self.trigger_attrs(),
            Part::Backdrop => self.backdrop_attrs(),
            Part::Positioner => self.positioner_attrs(),
            Part::Content => self.content_attrs(),
            Part::Title => self.title_attrs(),
            Part::Description => self.description_attrs(),
            Part::CancelTrigger => self.cancel_trigger_attrs(),
            Part::ActionTrigger => self.action_trigger_attrs(),
            Part::CloseTrigger => self.close_trigger_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
AlertDialog
├── Root             (required)
├── Trigger          (required — button that opens the alert dialog)
├── Backdrop         (required — blocks interaction with background)
├── Positioner       (required — centers the content)
├── Content          (required — role="alertdialog")
├── Title            (required — alert heading)
├── Description      (required — alert explanation)
├── CancelTrigger    (required — the "safe" action, receives initial focus)
├── ActionTrigger    (required — the destructive/confirming action)
└── CloseTrigger     (optional — explicit close button)
```

| Part          | Element    | Key Attributes                                          |
| ------------- | ---------- | ------------------------------------------------------- |
| Root          | `<div>`    | `data-ars-scope="alert-dialog"`, `data-ars-state`       |
| Trigger       | `<button>` | `aria-haspopup="dialog"`, `aria-expanded`               |
| Backdrop      | `<div>`    | `aria-hidden="true"`, `inert`                           |
| Positioner    | `<div>`    | `data-ars-scope="alert-dialog"`                         |
| Content       | `<div>`    | `role="alertdialog"`, `aria-modal="true"`               |
| Title         | `<h2>`     | `aria-labelledby` target                                |
| Description   | `<p>`      | `aria-describedby` target                               |
| CancelTrigger | `<button>` | `aria-label` from Messages — **receives initial focus** |
| ActionTrigger | `<button>` | `aria-label` from Messages, `data-ars-destructive`      |
| CloseTrigger  | `<button>` | `data-ars-scope="alert-dialog"`                         |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part    | Property           | Value               |
| ------- | ------------------ | ------------------- |
| Content | `role`             | `"alertdialog"`     |
| Content | `aria-modal`       | `"true"`            |
| Content | `aria-labelledby`  | Title part ID       |
| Content | `aria-describedby` | Description part ID |

- `role="alertdialog"` announces the dialog as an alert, distinguishing it from a generic dialog.
- No backdrop dismiss: clicking outside does NOT close the AlertDialog.
- No Escape dismiss: pressing Escape does NOT close the AlertDialog.
- The user MUST explicitly choose an action (Cancel or Confirm/Delete).

### 3.2 Focus Management

- **Initial focus goes to CancelTrigger** (the safe action), not the first focusable element or the destructive action. This prevents accidental confirmation of destructive actions.
- Focus trap works identically to Dialog (Tab/Shift+Tab cycle within content).
- On close, focus returns to the trigger element that opened the AlertDialog.

## 4. Internationalization

### 4.1 Messages

```rust
#[derive(Clone, Debug)]
pub struct Messages {
    /// Confirm action label (default: "Confirm")
    pub confirm_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Cancel action label (default: "Cancel")
    pub cancel_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            confirm_label: MessageFn::static_str("Confirm"),
            cancel_label: MessageFn::static_str("Cancel"),
        }
    }
}

impl ComponentMessages for Messages {}
```

## 5. Library Parity

> Compared against: Ark UI (`Dialog` with `role="alertdialog"`), Radix UI (`AlertDialog`), React Aria (`Dialog` with `role="alertdialog"`).

### 5.1 Props

| Feature                      | ars-ui                              | Ark UI                   | Radix UI          | React Aria                  | Notes                                                                  |
| ---------------------------- | ----------------------------------- | ------------------------ | ----------------- | --------------------------- | ---------------------------------------------------------------------- |
| Controlled open              | `open`                              | `open`                   | `open`            | `isOpen`                    | All libraries                                                          |
| Default open                 | `default_open`                      | `defaultOpen`            | `defaultOpen`     | `defaultOpen`               | All libraries                                                          |
| Modal (always)               | `modal` (default true)              | `modal`                  | (implicit true)   | (implicit)                  | All libraries                                                          |
| Close on Escape              | `close_on_escape` (default false)   | `closeOnEscape`          | (onEscapeKeyDown) | `isKeyboardDismissDisabled` | ars-ui defaults to false for safety                                    |
| Close on outside             | `close_on_backdrop` (default false) | `closeOnInteractOutside` | --                | `isDismissable`             | ars-ui defaults to false; Radix has no outside dismiss for AlertDialog |
| Role                         | `role` (AlertDialog)                | `role="alertdialog"`     | (implicit)        | `role="alertdialog"`        | All libraries                                                          |
| Is destructive               | `is_destructive`                    | --                       | --                | --                          | ars-ui addition for styling                                            |
| Open change callback         | `on_open_change`                    | `onOpenChange`           | `onOpenChange`    | `onOpenChange`              | All libraries                                                          |
| Escape callback              | `on_escape_key_down`                | `onEscapeKeyDown`        | `onEscapeKeyDown` | --                          | Preventable                                                            |
| Outside interaction callback | `on_interact_outside`               | `onInteractOutside`      | --                | --                          | Preventable                                                            |

**Gaps:** None.

### 5.2 Anatomy

| Part          | ars-ui        | Ark UI       | Radix UI    | React Aria      | Notes                      |
| ------------- | ------------- | ------------ | ----------- | --------------- | -------------------------- |
| Root          | Root          | Root         | Root        | --              | Container                  |
| Trigger       | Trigger       | Trigger      | Trigger     | (DialogTrigger) | Open button                |
| Backdrop      | Backdrop      | Backdrop     | Overlay     | ModalOverlay    | Background overlay         |
| Positioner    | Positioner    | Positioner   | --          | --              | Centering wrapper          |
| Content       | Content       | Content      | Content     | Dialog          | Main content               |
| Title         | Title         | Title        | Title       | (Heading slot)  | Alert heading              |
| Description   | Description   | Description  | Description | --              | Alert explanation          |
| CancelTrigger | CancelTrigger | --           | Cancel      | --              | Safe/cancel action         |
| ActionTrigger | ActionTrigger | --           | Action      | --              | Destructive/confirm action |
| CloseTrigger  | CloseTrigger  | CloseTrigger | --          | --              | Explicit close             |

**Gaps:** None. Radix has dedicated `Cancel` and `Action` parts which map to ars-ui's `CancelTrigger` and `ActionTrigger`.

### 5.3 Events

| Callback         | ars-ui               | Ark UI            | Radix UI           | React Aria     | Notes          |
| ---------------- | -------------------- | ----------------- | ------------------ | -------------- | -------------- |
| Open change      | `on_open_change`     | `onOpenChange`    | `onOpenChange`     | `onOpenChange` | All libraries  |
| Escape key       | `on_escape_key_down` | `onEscapeKeyDown` | `onEscapeKeyDown`  | --             | Preventable    |
| Open auto focus  | (initial_focus)      | --                | `onOpenAutoFocus`  | --             | Radix callback |
| Close auto focus | (final_focus)        | --                | `onCloseAutoFocus` | --             | Radix callback |

**Gaps:** None.

### 5.4 Features

| Feature                       | ars-ui | Ark UI         | Radix UI | React Aria |
| ----------------------------- | ------ | -------------- | -------- | ---------- |
| role="alertdialog"            | Yes    | Yes            | Yes      | Yes        |
| No Escape dismiss (default)   | Yes    | (configurable) | Yes      | Yes        |
| No outside dismiss (default)  | Yes    | (configurable) | Yes      | Yes        |
| Cancel button (initial focus) | Yes    | --             | Yes      | --         |
| Action button                 | Yes    | --             | Yes      | --         |
| Destructive styling flag      | Yes    | --             | --       | --         |
| Focus trap                    | Yes    | Yes            | Yes      | Yes        |

**Gaps:** None.

### 5.5 Summary

- **Overall:** Full parity.
- **Divergences:** (1) Ark UI uses `Dialog` with `role="alertdialog"` prop; ars-ui has a dedicated `AlertDialog` component with safety-oriented defaults (no Escape, no outside dismiss). (2) ars-ui adds `is_destructive` prop for styling the action button, which no reference library provides. (3) `CancelTrigger` receives initial focus by default to prevent accidental destructive actions.
- **Recommended additions:** None.
