---
component: Keyboard
category: utility
tier: stateless
foundation_deps: [architecture, accessibility]
shared_deps: []
related: []
references:
  react-aria: Keyboard
---

# Keyboard

A stateless utility component that renders keyboard shortcut text inside semantic `<kbd>` elements. Primarily used inside Menu item shortcut slots but applicable anywhere shortcut hints are shown.

## 1. API

### 1.1 Props

```rust
/// Props for the `Keyboard` component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// The keyboard shortcut text to display (e.g., "⌘C", "Ctrl+Shift+P").
    /// When platform_aware is true, modifier tokens are auto-mapped.
    pub shortcut: String,
    /// When true, automatically maps generic modifier tokens to platform-specific
    /// symbols: "Mod" → "⌘" (macOS) / "Ctrl" (others), "Alt" → "⌥" (macOS) / "Alt",
    /// "Shift" → "⇧" (macOS) / "Shift", "Meta" → "⌘" (macOS) / "⊞" (Windows).
    /// Default: false (render shortcut text as-is).
    pub platform_aware: bool,
    /// Whether the current platform is macOS. The adapter provides this value
    /// via `PlatformEffects::is_mac_platform()` (see `01-architecture.md` §2.2.7).
    /// During SSR, defaults to `false` (non-Mac); client-side hydration updates
    /// the display if the actual platform differs.
    pub is_mac: bool,
}
```

### 1.2 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "keyboard"]
pub enum Part {
    Root,
}

/// API for the `Keyboard` component.
pub struct Api<'a> {
    props: &'a Props,
}

impl<'a> Api<'a> {
    pub fn new(props: &'a Props) -> Self {
        Self { props }
    }

    /// Root <kbd> element attributes.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// Returns the formatted shortcut text, applying platform-aware
    /// modifier mapping when `platform_aware` is true.
    pub fn display_text(&self, locale: Option<&Locale>) -> String {
        if self.props.platform_aware {
            format_platform_shortcut(&self.props.shortcut, self.props.is_mac, locale)
        } else {
            self.props.shortcut.clone()
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
        }
    }
}

// Platform detection is provided by the adapter via `PlatformEffects::is_mac_platform()`
// (see `01-architecture.md` §2.2.7). The `is_mac` parameter is passed into
// `format_platform_shortcut` by the caller. During SSR, the adapter defaults
// to `false` (non-Mac). Client-side hydration updates the display if the
// actual platform differs.

/// Returns the localized display name for a modifier key.
fn localized_modifier(key: &str, locale: Option<&Locale>) -> String {
    let lang = locale.map(|l| l.language()).unwrap_or("en");
    match (key, lang) {
        ("Ctrl", "de") => "Strg",
        ("Shift", "de") => "Umschalt",
        ("Shift", "fr") => "Maj",
        _ => key,
    }.to_string()
}

/// Maps generic modifier tokens to platform-specific symbols.
/// `is_mac` is provided by the adapter via `PlatformEffects::is_mac_platform()`.
fn format_platform_shortcut(shortcut: &str, is_mac: bool, locale: Option<&Locale>) -> String {
    shortcut
        .split('+')
        .map(|part| match (part, is_mac) {
            ("Mod", true) => "⌘".to_string(),
            ("Mod", false) => localized_modifier("Ctrl", locale),
            ("Shift", true) => "⇧".to_string(),
            ("Shift", false) => localized_modifier("Shift", locale),
            ("Alt", true) => "⌥".to_string(),
            ("Alt", false) => localized_modifier("Alt", locale),
            ("Ctrl", true) => "⌃".to_string(),
            ("Meta", true) => "⌘".to_string(),
            ("Meta", false) => "Win".to_string(),
            (key, _) => key.to_string(),
        })
        .collect::<Vec<_>>()
        .join(if is_mac { "" } else { "+" })
}
```

## 2. Anatomy

```text
Keyboard
└── Root  <kbd>  data-ars-scope="keyboard" data-ars-part="root"
```

| Part | Element | Key Attributes                                      |
| ---- | ------- | --------------------------------------------------- |
| Root | `<kbd>` | `data-ars-scope="keyboard"`, `data-ars-part="root"` |

**1 part total.** A single `<kbd>` element containing the formatted shortcut text.

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

- The `<kbd>` element is semantically appropriate for keyboard input text (HTML spec).
- When used inside Menu items, the parent `Shortcut` part already sets `aria-hidden="true"` — no duplication needed.
- When used standalone, the `<kbd>` text is announced by screen readers naturally. If the shortcut is purely decorative, the consumer should add `aria-hidden="true"` to the parent.
- Integration with `menu::Item.aria_keyshortcuts` for programmatic shortcut announcement is the consumer's responsibility (already defined in `selection/menu.md` §6).

### 3.2 Locale Handling

`KeyboardEvent.key` is already locale-aware — it returns the character for the user's keyboard layout (e.g., `"z"` on QWERTY, `"w"` on AZERTY for the same physical key). No additional `locale` prop is needed for key matching. The `platform_aware` prop handles modifier symbol mapping (⌘ vs Ctrl), which is platform-dependent, not locale-dependent.

When `locale` is provided, modifier key display names are localized:

| Modifier | English (default) | German (`de`) | French (`fr`) | Spanish (`es`) | Italian (`it`) | Japanese (`ja`) |
| -------- | ----------------- | ------------- | ------------- | -------------- | -------------- | --------------- |
| Ctrl     | Ctrl              | Strg          | Ctrl          | Ctrl           | Ctrl           | Ctrl            |
| Shift    | Shift             | Umschalt      | Maj           | Mayús          | Maiusc         | Shift           |
| Alt      | Alt               | Alt           | Alt           | Alt            | Alt            | Alt             |
| Meta     | ⌘ (macOS) / Win   | ⌘ / Win       | ⌘ / Win       | ⌘ / Win        | ⌘ / Win        | ⌘ / Win         |

When `locale` is `None`, English names are used. The table above covers all languages that localize modifier key labels on physical keyboards. All other languages (Chinese, Korean, Arabic, Russian, Portuguese, Hindi, Turkish, etc.) use English labels on their physical keyboards and require no localization. macOS uses universal symbols (⌘ ⇧ ⌥ ⌃) regardless of language.

## 4. Usage Patterns

**In a Menu item (composing with existing Shortcut part):**

```rust
// Leptos
view! {
    <div {..api.item_attrs(&key)}>
        <span {..api.item_text_attrs(&key)}>{item.label}</span>
        <kbd {..api.item_shortcut_attrs(&key)}>
            <Keyboard shortcut="Mod+C" platform_aware=true />
        </kbd>
    </div>
}
```

**Standalone shortcut hint:**

```rust
view! {
    <div class="shortcut-hint">
        <span>"Save"</span>
        <Keyboard shortcut="Mod+S" platform_aware=true />
    </div>
}
```

## 5. Relationship to `Menu` `Shortcut`

The `Menu` component's `Shortcut` part (`selection/menu.md` §6) handles the **layout and accessibility** of shortcut hints within menu items. The `Keyboard` component handles the **rendering and formatting** of the shortcut text itself. They compose: `Keyboard` renders inside the `Menu`'s `Shortcut` slot.

## 6. Library Parity

> Compared against: React Aria (`Keyboard`).

### 6.1 Props

| Feature        | ars-ui           | React Aria | Notes                                          |
| -------------- | ---------------- | ---------- | ---------------------------------------------- |
| Shortcut text  | `shortcut`       | children   | RA uses children; ars-ui uses a string prop    |
| Platform-aware | `platform_aware` | --         | ars-ui addition for auto-mapping modifier keys |
| Separator      | `separator`      | --         | ars-ui addition for customizable key separator |

**Gaps:** None.

### 6.2 Anatomy

| Part | ars-ui                 | React Aria           | Notes                              |
| ---- | ---------------------- | -------------------- | ---------------------------------- |
| Root | `Root` (`<kbd>`)       | `Keyboard` (`<kbd>`) | Both libraries                     |
| Key  | `Key` (nested `<kbd>`) | (nested `<kbd>`)     | Both render nested `<kbd>` per key |

**Gaps:** None.

### 6.3 Features

| Feature                   | ars-ui | React Aria |
| ------------------------- | ------ | ---------- |
| Semantic `<kbd>` elements | Yes    | Yes        |
| Nested `<kbd>` per key    | Yes    | Yes        |
| Platform modifier mapping | Yes    | --         |

**Gaps:** None.

### 6.4 Summary

- **Overall:** Full parity -- ars-ui is a superset.
- **Divergences:** ars-ui adds platform-aware modifier mapping (`Mod` -> platform symbol) and configurable separator. React Aria's Keyboard is a simpler `<kbd>` renderer.
- **Recommended additions:** None.
