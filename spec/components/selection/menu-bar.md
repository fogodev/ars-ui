---
component: MenuBar
category: selection
tier: complex
foundation_deps: [architecture, accessibility, interactions, collections]
shared_deps: [selection-patterns]
related: []
references:
    radix-ui: Menubar
---

# MenuBar

A horizontal bar of top-level menu triggers (File, Edit, View...) where hovering between
open menus switches which popup is shown.

Top-level menus are stored as a `StaticCollection<menu_bar::Menu>` (from `06-collections.md`).
Navigation uses `Collection` trait methods.

## 1. State Machine

```rust
/// Payload for top-level menu bar entries.
#[derive(Clone, Debug)]
pub struct Menu {
    /// The label of the menu bar menu.
    pub label: String,
}
```

### 1.1 States

```rust
/// The state of the MenuBar component.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// No menu is active; arrow keys focus triggers.
    Inactive,
    /// A menu is open; arrow keys navigate within it.
    Active {
        /// The key of the active menu.
        menu: Key,
    },
}
```

### 1.2 Events

```rust
/// The events of the MenuBar component.
#[derive(Clone, Debug)]
pub enum Event {
    /// Focus a top-level menu trigger.
    FocusItem(Key),
    /// Activate (open) a menu popup.
    ActivateMenu(Key),
    /// Deactivate — close current menu.
    DeactivateMenu,
    /// Move focus to next top-level trigger (wraps).
    MoveToNextMenu,
    /// Move focus to previous top-level trigger (wraps).
    MoveToPrevMenu,
    /// Close everything.
    Close,
    /// Focus the menu bar.
    Focus {
        /// Whether the focus is from a keyboard event.
        is_keyboard: bool,
    },
    /// Blur the menu bar.
    Blur,
}
```

### 1.3 Context

```rust
/// The context of the `MenuBar` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The resolved locale for message formatting.
    pub locale: Locale,
    /// The menus of the menu bar.
    pub menus: StaticCollection<Menu>,
    /// The active menu of the menu bar.
    pub active_menu: Option<Key>,
    /// The focused item of the menu bar.
    pub focused_item: Option<Key>,
    /// Whether the focus is visible.
    pub focus_visible: bool,
    /// Component IDs for part identification.
    pub ids: ComponentIds,
    /// Resolved messages for accessibility labels.
    pub messages: Messages,
}
```

### 1.4 Props

```rust
/// Props for the MenuBar state machine.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The id of the menu bar.
    pub id: String,
    /// Whether the menu bar is disabled.
    pub disabled: bool,
    /// Orientation of the menu bar. Default: `Horizontal`.
    pub orientation: Orientation,
    /// Text direction for RTL support. Default: `Ltr`.
    pub dir: Direction,
    /// Whether focus wraps from the last trigger back to the first (and vice versa).
    /// Default: `true`.
    pub loop_focus: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            disabled: false,
            orientation: Orientation::Horizontal,
            dir: Direction::Ltr,
            loop_focus: true,
        }
    }
}
```

### 1.5 Full Machine Implementation

```rust
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Api<'a> = Api<'a>;
    type Messages = Messages;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let locale = env.locale.clone();
        let messages = messages.clone();
        let ctx = Context {
            locale,
            menus: StaticCollection::default(),
            active_menu: None,
            focused_item: None,
            focus_visible: false,
            ids: ComponentIds::from_id(&props.id),
            messages,
        };
        (State::Inactive, ctx)
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            // ActivateMenu: open the menu, enter Active state
            (_, Event::ActivateMenu(key)) => {
                let key = key.clone();
                Some(TransitionPlan::to(State::Active { menu: key.clone() }).apply(move |ctx| {
                    ctx.active_menu = Some(key.clone());
                    ctx.focused_item = Some(key);
                }).with_effect(PendingEffect::new("focus_menu_content", |ctx, _props, _send| {
                    if let Some(ref menu_key) = ctx.active_menu {
                        let platform = use_platform_effects();
                        let content_id = ctx.ids.item("menu-content", &menu_key);
                        platform.focus_element_by_id(&content_id);
                    }
                    no_cleanup()
                })))
            }

            // MoveToNextMenu: in Active state, close current, open next
            (State::Active { .. }, Event::MoveToNextMenu) => {
                let next = match &ctx.focused_item {
                    Some(k) => ctx.menus.key_after(k),
                    None => ctx.menus.first_key().cloned(),
                };
                // Wrap around
                let next = next.or_else(|| ctx.menus.first_key().cloned());
                if let Some(next_key) = next {
                    let nk = next_key.clone();
                    Some(TransitionPlan::to(State::Active { menu: next_key }).apply(move |ctx| {
                        ctx.active_menu = Some(nk.clone());
                        ctx.focused_item = Some(nk);
                    }).with_effect(PendingEffect::new("focus_menu_content", |ctx, _props, _send| {
                        if let Some(ref menu_key) = ctx.active_menu {
                            let platform = use_platform_effects();
                            let content_id = ctx.ids.item("menu-content", &menu_key);
                            platform.focus_element_by_id(&content_id);
                        }
                        no_cleanup()
                    })))
                } else {
                    None
                }
            }

            // Deactivate or Close — return to Inactive
            (State::Active { .. }, Event::DeactivateMenu) |
            (State::Active { .. }, Event::Close) => {
                Some(TransitionPlan::to(State::Inactive).apply(|ctx| {
                    ctx.active_menu = None;
                }))
            }

            // MoveToPrevMenu — mirror of MoveToNextMenu with reverse direction
            (State::Active { .. }, Event::MoveToPrevMenu) => {
                let prev = match &ctx.focused_item {
                    Some(k) => ctx.menus.key_before(k),
                    None => ctx.menus.last_key().cloned(),
                };

                let prev = prev.or_else(|| ctx.menus.last_key().cloned());

                if let Some(prev_key) = prev {
                    let pk = prev_key.clone();

                    Some(TransitionPlan::to(State::Active { menu: prev_key }).apply(move |ctx| {
                        ctx.active_menu = Some(pk.clone());
                        ctx.focused_item = Some(pk);
                    }).with_effect(PendingEffect::new("focus_menu_content", |ctx, _props, _send| {
                        if let Some(ref menu_key) = ctx.active_menu {
                            let platform = use_platform_effects();
                            let content_id = ctx.ids.item("menu-content", &menu_key);
                            platform.focus_element_by_id(&content_id);
                        }
                        no_cleanup()
                    })))
                } else {
                    None
                }
            }

            // FocusItem — focus a specific top-level trigger by Key (Inactive)
            (State::Inactive, Event::FocusItem(key)) => {
                let key = key.clone();

                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focused_item = Some(key);
                }).with_effect(PendingEffect::new("focus_element", |ctx, _props, _send| {
                    if let Some(ref item_key) = ctx.focused_item {
                        let platform = use_platform_effects();
                        let trigger_id = ctx.ids.item("trigger", &item_key);
                        platform.focus_element_by_id(&trigger_id);
                    }
                    no_cleanup()
                })))
            }

            // FocusItem — in Active state, focusing a different trigger switches the open menu
            (State::Active { .. }, Event::FocusItem(key)) => {
                let key = key.clone();

                Some(TransitionPlan::to(State::Active { menu: key.clone() }).apply(move |ctx| {
                    ctx.active_menu = Some(key.clone());
                    ctx.focused_item = Some(key);
                }).with_effect(PendingEffect::new("focus_menu_content", |ctx, _props, _send| {
                    if let Some(ref menu_key) = ctx.active_menu {
                        let platform = use_platform_effects();
                        let content_id = ctx.ids.item("menu-content", &menu_key);
                        platform.focus_element_by_id(&content_id);
                    }
                    no_cleanup()
                })))
            }

            // Focus — keyboard/pointer focus on the menubar root
            (State::Inactive, Event::Focus { is_keyboard }) => {
                let is_kb = *is_keyboard;
                let first = ctx.menus.first_key().cloned();

                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focus_visible = is_kb;
                    if ctx.focused_item.is_none() {
                        ctx.focused_item = first;
                    }
                }))
            }

            // Blur — return to Inactive from any state
            (_, Event::Blur) => {
                Some(TransitionPlan::to(State::Inactive).apply(|ctx| {
                    ctx.active_menu = None;
                    ctx.focused_item = None;
                    ctx.focus_visible = false;
                }))
            }

            // In Inactive state, MoveToNextMenu moves focus between triggers without opening
            (State::Inactive, Event::MoveToNextMenu) => {
                let next = match &ctx.focused_item {
                    Some(k) => ctx.menus.key_after(k),
                    None => ctx.menus.first_key().cloned(),
                };

                let next = next.or_else(|| ctx.menus.first_key().cloned());

                if let Some(next_key) = next {
                    Some(TransitionPlan::context_only(move |ctx| {
                        ctx.focused_item = Some(next_key);
                    }).with_effect(PendingEffect::new("focus_element", |ctx, _props, _send| {
                        if let Some(ref item_key) = ctx.focused_item {
                            let platform = use_platform_effects();
                            let trigger_id = ctx.ids.item("trigger", &item_key);
                            platform.focus_element_by_id(&trigger_id);
                        }
                        no_cleanup()
                    })))
                } else {
                    None
                }
            }

            // In Inactive state, MoveToPrevMenu moves focus between triggers without opening
            (State::Inactive, Event::MoveToPrevMenu) => {
                let prev = match &ctx.focused_item {
                    Some(k) => ctx.menus.key_before(k),
                    None => ctx.menus.last_key().cloned(),
                };

                let prev = prev.or_else(|| ctx.menus.last_key().cloned());

                if let Some(prev_key) = prev {
                    Some(TransitionPlan::context_only(move |ctx| {
                        ctx.focused_item = Some(prev_key);
                    }).with_effect(PendingEffect::new("focus_element", |ctx, _props, _send| {
                        if let Some(ref item_key) = ctx.focused_item {
                            let platform = use_platform_effects();
                            let trigger_id = ctx.ids.item("trigger", &item_key);
                            platform.focus_element_by_id(&trigger_id);
                        }
                        no_cleanup()
                    })))
                } else {
                    None
                }
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
}
```

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "menu-bar"]
pub enum Part {
    Root,
    Menu { key: Key },
    MenuTrigger { key: Key },
    MenuPositioner { key: Key },
    MenuContent { key: Key },
}

/// API for the MenuBar component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Attributes for the root menubar container.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "menubar");
        attrs.set(HtmlAttr::Aria(AriaAttr::Orientation), "horizontal");
        if self.props.disabled { attrs.set_bool(HtmlAttr::Data("ars-disabled"), true); }
        attrs
    }

    /// Attributes for a top-level menu wrapper.
    pub fn menu_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Menu { key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.item("menu", key));
        attrs
    }

    /// Attributes for a top-level menu trigger button.
    pub fn menu_trigger_attrs(&self, key: &Key) -> AttrMap {
        let trigger_id = self.ctx.ids.item("trigger", key);
        let content_id = self.ctx.ids.item("menu-content", key);
        let is_active = self.ctx.active_menu.as_ref() == Some(key);
        let is_focused = self.ctx.focused_item.as_ref() == Some(key);
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::MenuTrigger { key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, trigger_id);
        attrs.set(HtmlAttr::Role, "menuitem");
        attrs.set(HtmlAttr::Aria(AriaAttr::HasPopup), "menu");
        attrs.set(HtmlAttr::Aria(AriaAttr::Expanded), if is_active { "true" } else { "false" });
        attrs.set(HtmlAttr::Aria(AriaAttr::Controls), content_id);
        attrs.set(HtmlAttr::TabIndex, if is_focused { "0" } else { "-1" });
        if is_active { attrs.set_bool(HtmlAttr::Data("ars-active"), true); }
        if self.props.disabled { attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true"); }
        attrs
    }

    /// Attributes for a menu positioner wrapper.
    pub fn menu_positioner_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::MenuPositioner { key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.item_part("menu", key, "positioner"));
        attrs
    }

    /// Attributes for a menu content panel.
    pub fn menu_content_attrs(&self, key: &Key) -> AttrMap {
        let content_id = self.ctx.ids.item("menu-content", key);
        let trigger_id = self.ctx.ids.item("trigger", key);
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::MenuContent { key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, content_id);
        attrs.set(HtmlAttr::Role, "menu");
        attrs.set(HtmlAttr::TabIndex, "-1");
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), trigger_id);
        attrs
    }

    /// Handle click on a menu trigger.
    pub fn on_trigger_click(&self, key: &Key) {
        if self.ctx.active_menu.as_ref() == Some(key) {
            (self.send)(Event::DeactivateMenu);
        } else {
            (self.send)(Event::ActivateMenu(key.clone()));
        }
    }

    /// Handle keydown on a menu trigger.
    pub fn on_trigger_keydown(&self, key: &Key, data: &KeyboardEventData) {
        let menu_key = key.clone();
        match data.key {
            KeyboardKey::ArrowDown | KeyboardKey::Enter | KeyboardKey::Space => {
                (self.send)(Event::ActivateMenu(menu_key));
            }
            KeyboardKey::ArrowRight => {
                if self.props.dir == Direction::Rtl {
                    (self.send)(Event::MoveToPrevMenu);
                } else {
                    (self.send)(Event::MoveToNextMenu);
                }
            }
            KeyboardKey::ArrowLeft => {
                if self.props.dir == Direction::Rtl {
                    (self.send)(Event::MoveToNextMenu);
                } else {
                    (self.send)(Event::MoveToPrevMenu);
                }
            }
            _ => {}
        }
    }

    /// Handle pointer enter on a menu trigger (switches open menu in Active state).
    pub fn on_trigger_pointer_enter(&self, key: &Key) {
        if self.ctx.active_menu.is_some() {
            (self.send)(Event::FocusItem(key.clone()));
        }
    }

    /// Handle keydown on menu content (delegates to inner Menu for item navigation;
    /// handles ArrowLeft/Right to switch between top-level menus).
    pub fn on_content_keydown(&self, data: &KeyboardEventData) {
        match data.key {
            KeyboardKey::ArrowRight => {
                if self.props.dir == Direction::Rtl {
                    (self.send)(Event::MoveToPrevMenu);
                } else {
                    (self.send)(Event::MoveToNextMenu);
                }
            }
            KeyboardKey::ArrowLeft => {
                if self.props.dir == Direction::Rtl {
                    (self.send)(Event::MoveToNextMenu);
                } else {
                    (self.send)(Event::MoveToPrevMenu);
                }
            }
            KeyboardKey::Escape => {
                (self.send)(Event::Close);
            }
            _ => {}
        }
    }

    /// Handle focus on the menu bar root.
    pub fn on_root_focus(&self, data: &FocusEventData) {
        (self.send)(Event::Focus { is_keyboard: data.is_keyboard });
    }

    /// Handle blur on the menu bar root.
    pub fn on_root_blur(&self) {
        (self.send)(Event::Blur);
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Menu { ref key } => self.menu_attrs(key),
            Part::MenuTrigger { ref key } => self.menu_trigger_attrs(key),
            Part::MenuPositioner { ref key } => self.menu_positioner_attrs(key),
            Part::MenuContent { ref key } => self.menu_content_attrs(key),
        }
    }
}
```

## 2. Anatomy

| Part             | Selector                                                       | Element    |
| ---------------- | -------------------------------------------------------------- | ---------- |
| `Root`           | `[data-ars-scope="menu-bar"][data-ars-part="root"]`            | `<div>`    |
| `Menu`           | `[data-ars-scope="menu-bar"][data-ars-part="menu"]`            | `<div>`    |
| `MenuTrigger`    | `[data-ars-scope="menu-bar"][data-ars-part="menu-trigger"]`    | `<button>` |
| `MenuPositioner` | `[data-ars-scope="menu-bar"][data-ars-part="menu-positioner"]` | `<div>`    |
| `MenuContent`    | `[data-ars-scope="menu-bar"][data-ars-part="menu-content"]`    | `<div>`    |
| `MenuItem`       | `[data-ars-scope="menu-bar"][data-ars-part="menu-item"]`       | `<div>`    |

Plus all Menu item-type parts (`CheckboxItem`, `RadioItem`, `Separator`, `SubTrigger`, `SubContent`).

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property           | Element       | Value                                        |
| ------------------ | ------------- | -------------------------------------------- |
| `role`             | `Root`        | `menubar`                                    |
| `aria-orientation` | `Root`        | `horizontal`                                 |
| `role`             | `MenuTrigger` | `menuitem`                                   |
| `aria-haspopup`    | `MenuTrigger` | `menu`                                       |
| `aria-expanded`    | `MenuTrigger` | `true` when menu is open                     |
| `tabindex`         | `MenuTrigger` | Roving: active trigger gets `0`, others `-1` |

### 3.2 Keyboard Interaction

| Key             | Inactive Mode               | Active Mode                       |
| --------------- | --------------------------- | --------------------------------- |
| ArrowLeft/Right | Move focus between triggers | Close current, open adjacent menu |
| ArrowDown       | Open current menu           | Navigate within menu              |
| Enter / Space   | Open current menu           | Activate menu item                |
| Escape          | ---                         | Close menu -> Inactive            |
| Tab             | Leave menubar               | Close menu, leave menubar         |

## 4. Internationalization

### 4.1 Messages

```rust
/// Translatable messages for MenuBar.
#[derive(Clone, Debug)]
pub struct Messages {
    // No component-generated text — all labels are consumer-provided.
    // Struct exists for pattern conformance and future extension.
}

impl Default for Messages {
    fn default() -> Self {
        Self {}
    }
}

impl ComponentMessages for Messages {}
```

- **RTL**: ArrowLeft/Right swap direction for horizontal menubar.
- **Trigger labels**: User-provided, localized.
- **Keyboard shortcut text**: Follows OS conventions (Ctrl vs Cmd).

## 5. Library Parity

> Compared against: Radix UI (`Menubar`).

### 5.1 Props

| Feature                | ars-ui        | Radix UI                                   | Notes                                                |
| ---------------------- | ------------- | ------------------------------------------ | ---------------------------------------------------- |
| Disabled               | `disabled`    | --                                         | ars-ui exclusive                                     |
| Orientation            | `orientation` | --                                         | Radix is horizontal-only                             |
| Direction (RTL)        | `dir`         | `dir`                                      | --                                                   |
| Loop focus             | `loop_focus`  | `loop`                                     | --                                                   |
| Controlled active menu | --            | `value` / `defaultValue` / `onValueChange` | Radix exposes which menu is open as controlled state |

**Gaps:** None. Radix's controlled `value`/`onValueChange` for tracking which menu is open is a convenience; ars-ui manages this internally via `State::Active { menu }`.

### 5.2 Anatomy

| Part                      | ars-ui              | Radix UI                        | Notes             |
| ------------------------- | ------------------- | ------------------------------- | ----------------- |
| Root                      | `Root`              | `Root`                          | Menubar container |
| MenuTrigger               | `MenuTrigger`       | `Trigger`                       | --                |
| MenuPositioner            | `MenuPositioner`    | `Portal`                        | --                |
| MenuContent               | `MenuContent`       | `Content`                       | --                |
| Item (within menu)        | delegated to `Menu` | `Item`                          | --                |
| Group                     | delegated to `Menu` | `Group`                         | --                |
| Label                     | delegated to `Menu` | `Label`                         | --                |
| CheckboxItem              | delegated to `Menu` | `CheckboxItem`                  | --                |
| RadioGroup                | delegated to `Menu` | `RadioGroup`                    | --                |
| RadioItem                 | delegated to `Menu` | `RadioItem`                     | --                |
| ItemIndicator             | delegated to `Menu` | `ItemIndicator`                 | --                |
| Separator                 | delegated to `Menu` | `Separator`                     | --                |
| Arrow                     | delegated to `Menu` | `Arrow`                         | --                |
| Sub/SubTrigger/SubContent | delegated to `Menu` | `Sub`/`SubTrigger`/`SubContent` | --                |

**Gaps:** None. ars-ui delegates individual menu content to the `Menu` component rather than re-declaring all menu parts.

### 5.3 Events

| Callback           | ars-ui                        | Radix UI            | Notes |
| ------------------ | ----------------------------- | ------------------- | ----- |
| Active menu change | via `State::Active { menu }`  | `onValueChange`     | --    |
| Item action        | delegated to `Menu.on_action` | per-item `onSelect` | --    |

**Gaps:** None.

### 5.4 Features

| Feature                     | ars-ui                        | Radix UI |
| --------------------------- | ----------------------------- | -------- |
| Horizontal menubar          | Yes                           | Yes      |
| Vertical menubar            | Yes (`orientation: Vertical`) | No       |
| Arrow key menu switching    | Yes                           | Yes      |
| Hover-to-switch when active | Yes                           | Yes      |
| Submenus (within menus)     | Yes (via `Menu`)              | Yes      |
| Checkbox/radio items        | Yes (via `Menu`)              | Yes      |
| Typeahead                   | Yes (via `Menu`)              | Yes      |
| RTL support                 | Yes                           | Yes      |
| Focus wrapping              | Yes                           | Yes      |

**Gaps:** None.

### 5.5 Summary

- **Overall:** Full parity -- no gaps identified.
- **Divergences:** (1) ars-ui supports vertical orientation; Radix is horizontal-only; (2) ars-ui delegates individual menu content to the `Menu` component rather than re-declaring all menu item types in the MenuBar spec; (3) Radix exposes the active menu as a controlled `value` string; ars-ui uses internal state tracking.
- **Recommended additions:** None.
