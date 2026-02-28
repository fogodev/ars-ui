---
component: TagGroup
category: data-display
tier: stateful
foundation_deps: [architecture, accessibility, collections, interactions]
shared_deps: []
related: []
references:
  react-aria: TagGroup
---

# TagGroup

A display-only group of removable tags with keyboard navigation and optional selection.
TagGroup is NOT a text input for creating tags (that would be TagsInput); it manages an
existing collection of tag items that the user can navigate, select, and remove. Maps to
React Aria's `TagGroup`.

## 1. State Machine

### 1.1 States

| State     | Description                                           |
| --------- | ----------------------------------------------------- |
| `Idle`    | No tag is focused.                                    |
| `Focused` | A tag within the group has keyboard or pointer focus. |

### 1.2 Events

| Event           | Payload                                | Description                                             |
| --------------- | -------------------------------------- | ------------------------------------------------------- |
| `Focus`         | `item: Option<Key>, is_keyboard: bool` | Focus entered the tag group or moved to a specific tag. |
| `Blur`          | —                                      | Focus left the tag group entirely.                      |
| `RemoveTag`     | `Key`                                  | Remove the tag identified by key.                       |
| `FocusNext`     | —                                      | Move focus to the next tag (ArrowRight / ArrowDown).    |
| `FocusPrevious` | —                                      | Move focus to the previous tag (ArrowLeft / ArrowUp).   |
| `FocusFirst`    | —                                      | Move focus to the first tag (Home).                     |
| `FocusLast`     | —                                      | Move focus to the last tag (End).                       |

### 1.3 Context

```rust
/// Context for the TagGroup component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The list of tag items currently displayed.
    pub items: StaticCollection<Tag>,
    /// Key of the currently focused tag, if any.
    pub focused_key: Option<Key>,
    /// True when focus was keyboard-initiated (drives visible focus ring).
    pub focus_visible: bool,
    /// When true, all tags are non-interactive.
    pub disabled: bool,
    /// Selection mode for tags.
    pub selection_mode: selection::Mode,
    /// Currently selected tag keys.
    pub selected_keys: Bindable<BTreeSet<Key>>,
    /// Unique component instance identifier.
    pub id: ComponentId,
    /// The current locale for message resolution.
    pub locale: Locale,
    /// Resolved messages for accessibility labels.
    pub messages: Messages,
}

/// Definition for a tag item.
#[derive(Clone, Debug, PartialEq)]
pub struct Tag {
    /// Unique identifier for this tag.
    pub key: Key,
    /// Display label text.
    pub label: String,
    /// Whether this individual tag is disabled.
    pub disabled: bool,
}
```

### 1.4 Props

```rust
/// Props for the TagGroup component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Tag items to display.
    pub items: StaticCollection<Tag>,
    /// Controlled selected keys.
    pub selected_keys: Option<BTreeSet<Key>>,
    /// Default selected keys for uncontrolled mode.
    pub default_selected_keys: BTreeSet<Key>,
    /// Selection mode.
    pub selection_mode: selection::Mode,
    /// Prevents deselecting the last remaining selected tag. When `true` and the user
    /// attempts to deselect the only selected tag, the action is a no-op.
    pub disallow_empty_selection: bool,
    /// Disable the entire tag group.
    pub disabled: bool,
    /// Accessible label for the tag group.
    pub label: Option<String>,
    /// Optional locale override. When `None`, resolved from the nearest `ArsProvider` context.
    pub locale: Option<Locale>,
    /// Translatable messages for accessibility labels (see §4.1 Messages).
    pub messages: Option<Messages>,
    // on_remove callback is registered in the adapter layer, not in Props.
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            items: Vec::new(),
            selected_keys: None,
            default_selected_keys: BTreeSet::new(),
            selection_mode: selection::Mode::None,
            disallow_empty_selection: false,
            disabled: false,
            label: None,
            locale: None,
            messages: None,
        }
    }
}
```

### 1.5 Full Machine Implementation

```rust
use ars_core::{TransitionPlan, ComponentIds, AttrMap, Bindable};

/// States for the TagGroup.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// No tag is focused.
    Idle,
    /// A tag within the group has keyboard or pointer focus.
    Focused,
}

/// Events for the TagGroup.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Focus entered the tag group or moved to a specific tag.
    Focus { item: Option<Key>, is_keyboard: bool },
    /// Focus left the tag group entirely.
    Blur,
    /// Remove the tag identified by key.
    RemoveTag(Key),
    /// Move focus to the next tag (ArrowRight / ArrowDown).
    FocusNext,
    /// Move focus to the previous tag (ArrowLeft / ArrowUp).
    FocusPrevious,
    /// Move focus to the first tag (Home).
    FocusFirst,
    /// Move focus to the last tag (End).
    FocusLast,
}

/// Machine for the TagGroup.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State   = State;
    type Event   = Event;
    type Context = Context;
    type Props   = Props;
    type Api<'a> = Api<'a>;

    fn init(props: &Props) -> (State, Context) {
        let locale = resolve_locale(props.locale.as_ref());
        let messages = resolve_messages::<Messages>(props.messages.as_ref(), &locale);
        (State::Idle, Context {
            items: props.items.clone(),
            focused_key: None,
            focus_visible: false,
            disabled: props.disabled,
            selection_mode: props.selection_mode.clone(),
            selected_keys: match &props.selected_keys {
                Some(keys) => Bindable::controlled(keys.clone()),
                None       => Bindable::uncontrolled(props.default_selected_keys.clone()),
            },
            id: ComponentId::new(),
            locale,
            messages,
        })
    }

    fn transition(
        state: &State,
        event: &Event,
        ctx:   &Context,
        props: &Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled {
            return match event {
                Event::Focus { .. } | Event::Blur => {
                    // Allow focus/blur for AT even when disabled
                    Some(TransitionPlan::to(State::Idle))
                }
                _ => None,
            };
        }

        match event {
            // ── Focus ────────────────────────────────────────────────────
            Event::Focus { item, is_keyboard } => {
                let key = item.clone().or_else(|| {
                    ctx.items.first().map(|t| t.key.clone())
                });
                let kb = *is_keyboard;
                Some(TransitionPlan::to(State::Focused).apply(move |ctx| {
                    ctx.focused_key = key;
                    ctx.focus_visible = kb;
                }))
            }

            Event::Blur => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.focused_key = None;
                    ctx.focus_visible = false;
                }))
            }

            // ── Remove ──────────────────────────────────────────────────
            Event::RemoveTag(key) => {
                let key = key.clone();
                // Find the tag; skip if disabled or not found
                let tag = ctx.items.iter().find(|t| t.key == key)?;
                if tag.disabled { return None; }

                // Determine next focus target after removal
                let idx = ctx.items.iter().position(|t| t.key == key)?;
                let next_key = if idx + 1 < ctx.items.len() {
                    Some(ctx.items[idx + 1].key.clone())
                } else if idx > 0 {
                    Some(ctx.items[idx - 1].key.clone())
                } else {
                    None
                };

                let target_state = if next_key.is_some() { State::Focused } else { State::Idle };
                let k = key.clone();
                let label = tag.label.clone();
                Some(TransitionPlan::to(target_state).apply(move |ctx| {
                    ctx.items.retain(|t| t.key != k);
                    ctx.selected_keys.get_mut_owned().retain(|k2| *k2 != k);
                    ctx.focused_key = next_key;
                }).with_named_effect("announce-removed", move |ctx, _props, _send| {
                    let platform = use_platform_effects();
                    let msg = (ctx.messages.removed_announcement)(&label, &ctx.locale);
                    platform.announce(&msg);
                    no_cleanup()
                }))
            }

            // ── Navigation ──────────────────────────────────────────────
            Event::FocusNext => {
                let current = ctx.focused_key.as_ref()?;
                let idx = ctx.items.iter().position(|t| t.key == *current)?;
                let next = ctx.items.iter().skip(idx + 1)
                    .find(|t| !t.disabled)
                    .map(|t| t.key.clone());
                let target_key = next.unwrap_or_else(|| current.clone());
                Some(TransitionPlan::to(State::Focused).apply(move |ctx| {
                    ctx.focused_key = Some(target_key);
                    ctx.focus_visible = true;
                }))
            }

            Event::FocusPrevious => {
                let current = ctx.focused_key.as_ref()?;
                let idx = ctx.items.iter().position(|t| t.key == *current)?;
                let prev = ctx.items.iter().take(idx).rev()
                    .find(|t| !t.disabled)
                    .map(|t| t.key.clone());
                let target_key = prev.unwrap_or_else(|| current.clone());
                Some(TransitionPlan::to(State::Focused).apply(move |ctx| {
                    ctx.focused_key = Some(target_key);
                    ctx.focus_visible = true;
                }))
            }

            Event::FocusFirst => {
                let first = ctx.items.iter()
                    .find(|t| !t.disabled)
                    .map(|t| t.key.clone())?;
                Some(TransitionPlan::to(State::Focused).apply(move |ctx| {
                    ctx.focused_key = Some(first);
                    ctx.focus_visible = true;
                }))
            }

            Event::FocusLast => {
                let last = ctx.items.iter().rev()
                    .find(|t| !t.disabled)
                    .map(|t| t.key.clone())?;
                Some(TransitionPlan::to(State::Focused).apply(move |ctx| {
                    ctx.focused_key = Some(last);
                    ctx.focus_visible = true;
                }))
            }
        }
    }

    fn connect<'a>(
        state: &'a State,
        ctx:   &'a Context,
        props: &'a Props,
        send:  &'a dyn Fn(Event),
    ) -> Api<'a> {
        Api { state, ctx, props, send }
    }
}
```

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "tag-group"]
pub enum Part {
    Root,
    Label,
    List,
    Tag { key: Key },
    TagRemove { key: Key },
}

/// API for the TagGroup component.
pub struct Api<'a> {
    /// The current state of the TagGroup.
    state: &'a State,
    /// The current context of the TagGroup.
    ctx:   &'a Context,
    /// The current props of the TagGroup.
    props: &'a Props,
    /// The event sender for the TagGroup.
    send:  &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Returns the attributes for the root element.
    pub fn root_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        p.set(HtmlAttr::Role, "grid");
        p.set(HtmlAttr::Aria(AriaAttr::Atomic), "false");
        p.set(HtmlAttr::Aria(AriaAttr::Relevant), "additions removals");
        if let Some(label) = &self.props.label {
            p.set(HtmlAttr::Aria(AriaAttr::Label), label);
        }
        if self.ctx.disabled {
            p.set_bool(HtmlAttr::Data("ars-disabled"), true);
            p.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }
        p.set(HtmlAttr::Data("ars-state"), match self.state {
            State::Idle    => "idle",
            State::Focused => "focused",
        });
        p
    }

    /// Returns the attributes for the label element.
    pub fn label_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Label.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        let label_id = format!("{}-label", self.ctx.id);
        p.set(HtmlAttr::Id, label_id);
        p
    }

    /// Returns the attributes for the list element.
    pub fn list_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::List.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        p.set(HtmlAttr::Role, "row");
        let label_id = format!("{}-label", self.ctx.id);
        p.set(HtmlAttr::Aria(AriaAttr::LabelledBy), label_id);
        p
    }

    /// Returns the attributes for the tag element.
    pub fn tag_attrs(&self, key: &Key) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Tag { key: Key::default() }.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        p.set(HtmlAttr::Data("ars-key"), key.to_string());
        p.set(HtmlAttr::Role, "gridcell");

        let item = self.ctx.items.iter().find(|t| &t.key == key);
        let is_disabled = self.ctx.disabled
            || item.map_or(false, |t| t.disabled);
        let is_focused = self.ctx.focused_key.as_ref() == Some(key);
        let is_selected = self.ctx.selected_keys.get().contains(key);

        if is_disabled {
            p.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
            p.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }
        if is_selected {
            p.set(HtmlAttr::Aria(AriaAttr::Selected), "true");
            p.set_bool(HtmlAttr::Data("ars-selected"), true);
        }

        // Roving tabindex
        p.set(HtmlAttr::TabIndex, if is_focused { "0" } else { "-1" });
        if is_focused && self.ctx.focus_visible {
            p.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        // Event handlers (focus, blur, keydown for navigation/removal) are typed methods on the Api struct.
        p
    }

    /// Returns the attributes for the tag remove element.
    pub fn tag_remove_attrs(&self, key: &Key) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::TagRemove { key: Key::default() }.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        p.set(HtmlAttr::Role, "button");
        p.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.remove_label)(&self.ctx.locale));
        p.set(HtmlAttr::TabIndex, "-1"); // Not separately tabbable; parent tag handles keyboard

        let item = self.ctx.items.iter().find(|t| &t.key == key);
        let is_disabled = self.ctx.disabled
            || item.map_or(false, |t| t.disabled);

        if is_disabled {
            p.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }
        // Event handlers (click to remove tag) are typed methods on the Api struct.
        p
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match &part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::List => self.list_attrs(),
            Part::Tag { key } => self.tag_attrs(key),
            Part::TagRemove { key } => self.tag_remove_attrs(key),
        }
    }
}
```

## 2. Anatomy

```text
TagGroup
├── Root               (container; data-ars-scope="tag-group" data-ars-part="root")
├── Label              (visible label text; data-ars-part="label")
├── List               (tag list wrapper; role="row"; data-ars-part="list")
│   ├── Tag            (individual tag; role="gridcell"; data-ars-part="tag")
│   │   └── TagRemove  (remove button; role="button"; data-ars-part="tag-remove")
│   └── ...
```

| Part        | Element    | Key Attributes                                                                                                     |
| ----------- | ---------- | ------------------------------------------------------------------------------------------------------------------ |
| `Root`      | `<div>`    | `role="grid"`, `aria-label`, `data-ars-state`, `data-ars-disabled`                                                 |
| `Label`     | `<span>`   | `id="{id}-label"`                                                                                                  |
| `List`      | `<div>`    | `role="row"`, `aria-labelledby`                                                                                    |
| `Tag`       | `<div>`    | `role="gridcell"`, `tabindex` (roving), `aria-selected`, `aria-disabled`, `data-ars-key`, `data-ars-focus-visible` |
| `TagRemove` | `<button>` | `role="button"`, `aria-label="Remove"`, `tabindex="-1"`                                                            |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

- **Grid pattern**: The tag group uses `role="grid"` with a single `role="row"` containing
  `role="gridcell"` for each tag. This provides a well-established ARIA pattern for
  collections of removable items.
- **Roving tabindex**: Only the currently focused tag has `tabindex="0"`; all others have
  `tabindex="-1"`. When no tag is focused, the first non-disabled tag receives `tabindex="0"`.
- **Focus management on removal**: When a tag is removed, focus moves to the next tag. If
  the removed tag was last, focus moves to the previous tag. If no tags remain, focus moves
  to the tag group container.
- **Remove button**: The inline remove button (`tag-remove`) has `tabindex="-1"` and is
  activated only via pointer click. Keyboard users remove tags with Delete or Backspace
  while the tag itself is focused.
- **Live region**: The root element uses `aria-relevant="additions removals"` so assistive
  technologies announce when tags are added or removed.
- **Disabled**: `aria-disabled="true"` on root and individual tags prevents interaction.
  Disabled tags are skipped during keyboard navigation.

### 3.2 Keyboard Interaction

| Key                        | Action                                                                          |
| -------------------------- | ------------------------------------------------------------------------------- |
| `Tab`                      | Focus enters the tag group on the first (or last-focused) tag; Tab again exits. |
| `ArrowRight` / `ArrowDown` | Move focus to the next tag.                                                     |
| `ArrowLeft` / `ArrowUp`    | Move focus to the previous tag.                                                 |
| `Home`                     | Move focus to the first tag.                                                    |
| `End`                      | Move focus to the last tag.                                                     |
| `Delete` / `Backspace`     | Remove the currently focused tag.                                               |

## 4. Internationalization

- Tag labels must come from a localized message catalog; do not hard-code English strings.

### 4.1 Plural Rules for Count-Displaying Components

All components that display counts (TagGroup tag counts, Badge numeric values,
Table row counts, GridList item counts) MUST use CLDR plural rules for
locale-correct pluralization. The template syntax follows ICU MessageFormat:

```icu
{count, plural,
    =0    {No items}
    one   {# item}
    other {# items}
}
```

CLDR defines six plural categories: `zero`, `one`, `two`, `few`, `many`, `other`.
Not all languages use all categories:

| Language | Categories Used                  | Example                                              |
| -------- | -------------------------------- | ---------------------------------------------------- |
| English  | one, other                       | "1 item" / "2 items"                                 |
| Arabic   | zero, one, two, few, many, other | "٠ عناصر" / "عنصر واحد" / "عنصران" / "٣ عناصر" / ... |
| Polish   | one, few, many, other            | "1 element" / "2 elementy" / "5 elementów"           |
| Japanese | other (only)                     | "3件"                                                |
| French   | one, other                       | "1 élément" / "2 éléments"                           |

**Implementation:** `ars-i18n` provides `PluralRules::select(locale, count) -> PluralCategory`
backed by ICU4X `PluralRules`. Message closures in `XyzMessages` structs receive the count
and locale, and MUST use `PluralRules::select()` to choose the correct form.

**Examples for count-displaying components:**

```rust
// Badge: accessible label with plural form
pub badge_label: MessageFn<dyn Fn(u64, &str, &Locale) -> String + Send + Sync>,
// Usage: (3, "notification", en) → "3 notifications"
// Usage: (1, "notification", en) → "1 notification"
// Usage: (3, "إشعار", ar) → "٣ إشعارات"

// TagGroup: count announcement
pub tag_count_label: MessageFn<dyn Fn(usize, &Locale) -> String + Send + Sync>,
// Usage: (5, en) → "5 tags"
// Usage: (1, en) → "1 tag"

// Table: row count for screen reader
pub row_count_label: MessageFn<dyn Fn(usize, &Locale) -> String + Send + Sync>,
// Usage: (100, en) → "100 rows"
// Usage: (1, de) → "1 Zeile"
```

- The remove button `aria-label` ("Remove") is localizable via a messages struct:

```rust
#[derive(Clone, Debug)]
pub struct Messages {
    /// Accessible label for tag remove buttons. Default: "Remove".
    pub remove_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Announcement when a tag is removed. Receives the tag label.
    pub removed_announcement: MessageFn<dyn Fn(&str, &Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            remove_label: MessageFn::static_str("Remove"),
            removed_announcement: MessageFn::new(|label, _locale| format!("{label}, removed")),
        }
    }
}

impl ComponentMessages for Messages {}
```

- **RTL**: In right-to-left locales, ArrowRight moves to the _previous_ tag and ArrowLeft
  moves to the _next_ tag (visual direction is reversed). The adapter applies `dir="rtl"`
  on the root element, and the keyboard handler swaps Left/Right semantics accordingly.

## 5. Library Parity

> Compared against: React Aria (`TagGroup`).

### 5.1 Props

| Feature                                   | ars-ui                                   | React Aria                   | Notes                                                                               |
| ----------------------------------------- | ---------------------------------------- | ---------------------------- | ----------------------------------------------------------------------------------- |
| `selection_mode`                          | `selection::Mode`                        | `SelectionMode`              | Equivalent                                                                          |
| `selected_keys` / `default_selected_keys` | `BTreeSet<Key>`                          | `Iterable<Key> \| 'all'`     | Equivalent                                                                          |
| `disabled`                                | `bool` (group-level)                     | --                           | ars-ui has global disable                                                           |
| `disabledKeys`                            | Per-tag `disabled: bool` on `Tag` struct | `Iterable<Key>`              | Equivalent; ars-ui uses per-item flag                                               |
| `disallow_empty_selection`                | `bool`                                   | `bool`                       | Added from React Aria                                                               |
| `escapeKeyBehavior`                       | --                                       | `'none' \| 'clearSelection'` | Omitted; TagGroup uses Delete/Backspace for removal, not Escape for selection clear |
| Tag `href`                                | --                                       | `string`                     | Omitted; tags are not typically link targets in ars-ui's design                     |

**Gaps:** None.

### 5.2 Anatomy

| Part        | ars-ui      | React Aria                                | Notes                                  |
| ----------- | ----------- | ----------------------------------------- | -------------------------------------- |
| Root        | `Root`      | `TagGroup`                                | --                                     |
| Label       | `Label`     | `Label`                                   | --                                     |
| List        | `List`      | `TagList`                                 | --                                     |
| Tag         | `Tag`       | `Tag`                                     | --                                     |
| TagRemove   | `TagRemove` | Built-in via `allowsRemoving` render prop | ars-ui has explicit remove button part |
| Description | --          | `Text` (slot="description")               | Adapter-level concern                  |
| FieldError  | --          | `FieldError`                              | Adapter-level concern                  |

**Gaps:** None. Description and error slots are adapter-level.

### 5.3 Events

| Callback              | ars-ui                               | React Aria          | Notes      |
| --------------------- | ------------------------------------ | ------------------- | ---------- |
| `on_selection_change` | Adapter layer (Bindable observation) | `onSelectionChange` | Equivalent |
| `on_remove`           | `RemoveTag` event                    | `onRemove`          | Equivalent |

**Gaps:** None.

### 5.4 Features

| Feature                     | ars-ui                         | React Aria           |
| --------------------------- | ------------------------------ | -------------------- |
| Selection (single/multiple) | Yes                            | Yes                  |
| Tag removal                 | Yes (Delete/Backspace + click) | Yes (`onRemove`)     |
| Disabled tags               | Yes (per-tag)                  | Yes (`disabledKeys`) |
| Keyboard navigation         | Yes (Arrow keys, Home, End)    | Yes                  |
| Focus management on removal | Yes                            | Yes                  |
| Live region announcements   | Yes (`aria-relevant`)          | Yes                  |

**Gaps:** None.

### 5.5 Summary

- **Overall:** Full parity.
- **Divergences:** React Aria uses `escapeKeyBehavior` for selection clearing; ars-ui omits this since TagGroup's primary keyboard interaction is tag removal (Delete/Backspace), not selection management. React Aria supports `href` on tags for link semantics; ars-ui treats tags as non-link items by design.
- **Recommended additions:** None.
