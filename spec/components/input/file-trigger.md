---
component: FileTrigger
category: input
tier: stateless
foundation_deps: [architecture, accessibility, interactions, forms]
shared_deps: []
related: [file-upload]
references:
  react-aria: FileTrigger
---

# FileTrigger

A `FileTrigger` is a thin, **stateless** wrapper around a hidden `<input type="file">` that opens the native file picker when a trigger element is pressed. Unlike FileUpload (full drag-drop + upload lifecycle) and DropZone (drag-drop target), FileTrigger is the simplest file-selection primitive: click, pick, callback.

`FileTrigger` can be composed with `DropZone` and `FileUpload`. For example, `FileUpload`'s trigger part internally behaves like a `FileTrigger`.

| Component       | Selection Method           | Upload | State Machine |
| --------------- | -------------------------- | ------ | ------------- |
| **FileTrigger** | Click → native picker      | No     | Stateless     |
| **DropZone**    | Drag-and-drop              | No     | 3 states      |
| **FileUpload**  | Click + drag-drop + upload | Yes    | 3 states      |

## 1. API

### 1.1 Props

```rust
/// Camera capture direction for mobile devices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureMode {
    /// Front-facing (selfie) camera.
    User,
    /// Rear-facing (environment) camera.
    Environment,
}

/// Props for the FileTrigger component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Accepted MIME types or file extensions (e.g., ["image/*", ".pdf"]).
    /// Empty = accept all files.
    pub accept: Vec<String>,
    /// Allow selecting multiple files.
    pub multiple: bool,
    /// Allow selecting directories (uses `webkitdirectory` attribute).
    pub directory: bool,
    /// Camera capture mode for mobile devices (sets `capture` attribute).
    /// None = no capture preference (default file picker).
    pub capture: Option<CaptureMode>,
    /// Whether the trigger is disabled.
    pub disabled: bool,
    /// Form field name for the hidden input.
    pub name: Option<String>,
    /// Optional locale override. When `None`, resolved from the nearest
    /// `ArsProvider` context.
    pub locale: Option<Locale>,
    /// Translatable messages.
    pub messages: Option<Messages>,
    // `on_select` callback is framework-specific; provided by adapter layer.
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            accept: Vec::new(),
            multiple: false,
            directory: false,
            capture: None,
            disabled: false,
            name: None,
            locale: None,
            messages: None,
        }
    }
}

/// Messages for the FileTrigger component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Accessible label for the hidden file input.
    /// Default (en): "Choose file" / "Choose files" depending on `multiple`.
    pub input_label: MessageFn<dyn Fn(bool, &Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            input_label: MessageFn::new(|multiple, _locale| {
                if multiple { "Choose files".to_string() } else { "Choose file".to_string() }
            }),
        }
    }
}

impl ComponentMessages for Messages {}
```

### 1.2 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "file-trigger"]
pub enum Part {
    Root,
    Trigger,
    Input,
}

/// API for the FileTrigger component (stateless — created directly from Props).
pub struct Api<'a> {
    props: &'a Props,
    locale: Locale,
    messages: Messages,
}

impl<'a> Api<'a> {
    pub fn new(props: &'a Props) -> Self {
        let locale = resolve_locale(props.locale.as_ref());
        let messages = resolve_messages::<Messages>(props.messages.as_ref(), &locale);
        Self { props, locale, messages }
    }

    /// Attributes for the root wrapper.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if self.props.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }
        attrs
    }

    /// Attributes for the pressable trigger element (e.g. a Button).
    /// Adapter wires: on:click → open_file_picker().
    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if self.props.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }
        attrs
    }

    /// Attributes for the hidden `<input type="file">`.
    /// Adapter wires: on:change → on_select callback with selected files.
    pub fn input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Input.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "file");
        if !self.props.accept.is_empty() {
            attrs.set(HtmlAttr::Accept, self.props.accept.join(","));
        }
        if self.props.multiple {
            attrs.set_bool(HtmlAttr::Multiple, true);
        }
        if self.props.directory {
            attrs.set_bool(HtmlAttr::WebkitDirectory, true);
        }
        if let Some(ref capture) = self.props.capture {
            attrs.set(HtmlAttr::Capture, match capture {
                CaptureMode::User => "user",
                CaptureMode::Environment => "environment",
            });
        }
        if let Some(ref name) = self.props.name {
            attrs.set(HtmlAttr::Name, name);
        }
        attrs.set(HtmlAttr::Aria(AriaAttr::Label),
            (self.messages.input_label)(self.props.multiple, &self.locale));
        attrs.set(HtmlAttr::TabIndex, "-1");
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }

    /// Programmatically opens the native file picker.
    /// Adapter implements via DOM ref: input_ref.click().
    pub fn open_file_picker(&self) {
        // Imperative — adapter calls input_ref.click() on the DOM element.
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Trigger => self.trigger_attrs(),
            Part::Input => self.input_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
FileTrigger
├── Root      <div>              data-ars-scope="file-trigger" data-ars-part="root"
├── Trigger   (consumer element) data-ars-part="trigger" (e.g. <Button>)
└── Input     <input>            data-ars-part="input" (type="file", visually hidden)
```

| Part    | Element                 | Key Attributes                               |
| ------- | ----------------------- | -------------------------------------------- |
| Root    | `<div>`                 | `data-ars-scope="file-trigger"`              |
| Trigger | slot (consumer element) | `aria-disabled` when disabled                |
| Input   | `<input type="file">`   | `aria-label`, `aria-hidden`, `tabindex="-1"` |

The Trigger is a slot — the consumer renders their own element (e.g., `<Button>`) and spreads `trigger_attrs()` onto it.

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property        | Element | Value                                 |
| --------------- | ------- | ------------------------------------- |
| `type`          | Input   | `"file"` — native file input          |
| `aria-label`    | Input   | From `messages.input_label`           |
| `aria-hidden`   | Input   | `"true"` — hidden from assistive tech |
| `tabindex`      | Input   | `"-1"` — not in tab order             |
| `aria-disabled` | Trigger | `"true"` when disabled                |

- The trigger element itself should be a focusable, pressable element (Button recommended).
- The hidden input is not in the tab order; all interaction goes through the trigger.
- Screen readers announce the trigger's own label; the hidden input label is a fallback for assistive tech that focuses inputs directly.

### 3.2 Keyboard Interaction

| Key           | Action                   |
| ------------- | ------------------------ |
| Enter / Space | Opens native file picker |

## 4. Internationalization

| Key                                  | Default (en-US)                    | Notes                    |
| ------------------------------------ | ---------------------------------- | ------------------------ |
| `file_trigger.input_label(multiple)` | `"Choose file"` / `"Choose files"` | Plural-aware via closure |

- RTL: No special handling needed — the trigger element and hidden input are not affected by text direction.

## 5. Form Integration

- **Hidden input**: The `Input` part is a native `<input type="file">` that participates in form submission when `name` is set. The browser handles file data serialization in `FormData`.
- **Disabled**: When `disabled=true`, the trigger shows `aria-disabled="true"` and the adapter prevents opening the file picker.
- **Reset behavior**: On form reset, the browser clears the file input value. The adapter does not need to handle this explicitly.

## 6. Library Parity

> Compared against: React Aria (`FileTrigger`).
>
> Note: Ark UI's `FileUpload` is a full drag-drop+upload component, not a minimal trigger. Radix UI has no file selection component.

### 6.1 Props

| Feature      | ars-ui                         | React Aria          | Notes              |
| ------------ | ------------------------------ | ------------------- | ------------------ |
| Accept types | `accept: Vec<String>`          | `acceptedFileTypes` | Full parity        |
| Multiple     | `multiple: bool`               | `allowsMultiple`    | Full parity        |
| Directory    | `directory: bool`              | `acceptDirectory`   | Full parity        |
| Capture      | `capture: Option<CaptureMode>` | `defaultCamera`     | Full parity        |
| Disabled     | `disabled: bool`               | -- (via child)      | ars-ui enhancement |
| Form name    | `name: Option<String>`         | --                  | ars-ui enhancement |

**Gaps:** None.

### 6.2 Anatomy

| Part    | ars-ui                | React Aria               | Notes       |
| ------- | --------------------- | ------------------------ | ----------- |
| Root    | `Root`                | `FileTrigger`            | Full parity |
| Trigger | `Trigger` (slot)      | `children` (Button/Link) | Full parity |
| Input   | `Input` (hidden file) | (internal)               | Full parity |

**Gaps:** None.

### 6.3 Events

| Callback       | ars-ui                | React Aria | Notes       |
| -------------- | --------------------- | ---------- | ----------- |
| Files selected | `on_select` (adapter) | `onSelect` | Full parity |

**Gaps:** None.

### 6.4 Features

| Feature             | ars-ui         | React Aria |
| ------------------- | -------------- | ---------- |
| Click to open       | Yes            | Yes        |
| Multiple files      | Yes            | Yes        |
| Directory selection | Yes            | Yes        |
| Camera capture      | Yes            | Yes        |
| MIME type filtering | Yes            | Yes        |
| Form integration    | Yes (via name) | --         |

**Gaps:** None.

### 6.5 Summary

- **Overall:** Full parity with React Aria FileTrigger.
- **Divergences:** ars-ui adds `disabled` and `name` props not present in React Aria's FileTrigger. ars-ui provides i18n messages for the hidden input's `aria-label`.
- **Recommended additions:** None.
